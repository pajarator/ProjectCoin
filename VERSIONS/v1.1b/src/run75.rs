/// RUN75 — Sharpe-Weighted Capital Allocation
///
/// Rebalance per-coin capital based on trailing Sharpe ratio.
/// More capital to higher-Sharpe coins, less to lower-Sharpe coins.
///
/// Grid: FREQ [84, 168, 336] × WINDOW [10, 20, 30] × MIN_CAP [0.25, 0.50] × MAX_CAP [1.5, 2.0, 3.0]
/// Total: 3 × 3 × 2 × 3 = 54 + baseline = 55 configs
///
/// Run: cargo run --release --features run75 -- --run75

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct SharpeCfg {
    freq: usize,
    window: usize,
    min_cap_mult: f64,
    max_cap_mult: f64,
    is_baseline: bool,
}

impl SharpeCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("F{}_W{}_MIN{:.2}_MAX{:.1}", self.freq, self.window, self.min_cap_mult, self.max_cap_mult) }
    }
}

fn build_grid() -> Vec<SharpeCfg> {
    let mut grid = vec![SharpeCfg { freq: 0, window: 0, min_cap_mult: 0.0, max_cap_mult: 0.0, is_baseline: true }];
    let freqs = [84, 168, 336];
    let windows = [10, 20, 30];
    let min_caps = [0.25, 0.50];
    let max_caps = [1.5, 2.0, 3.0];
    for &f in &freqs {
        for &w in &windows {
            for &min_c in &min_caps {
                for &max_c in &max_caps {
                    grid.push(SharpeCfg { freq: f, window: w, min_cap_mult: min_c, max_cap_mult: max_c, is_baseline: false });
                }
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?; let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?; let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Compute trailing Sharpe from a list of PnL percentages (annualized for 15m bars)
fn trailing_sharpe(pnl_pcts: &[f64], window: usize) -> f64 {
    if pnl_pcts.len() < 2 { return 0.0; }
    let start = pnl_pcts.len().saturating_sub(window);
    let trades = &pnl_pcts[start..];
    if trades.len() < 2 { return 0.0; }
    let mean: f64 = trades.iter().sum::<f64>() / trades.len() as f64;
    let variance: f64 = trades.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / trades.len() as f64;
    let std = variance.sqrt();
    if std == 0.0 { return 0.0; }
    // Annualize: 35040 bars/year (15m bars in a year)
    let annualized_mean = mean * 35040.0;
    let annualized_std = std * (35040.0_f64.sqrt());
    annualized_mean / annualized_std
}

// Portfolio-level simulation with Sharpe-weighted rebalancing
fn simulate_portfolio(data: &[CoinData15m], cfg: &SharpeCfg) -> (f64, usize, usize, usize, Vec<CoinResult>) {
    let n = data[0].closes.len();

    // Per-coin state
    let mut bals: Vec<f64> = vec![INITIAL_BAL; N_COINS];
    let mut pos: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldown: Vec<usize> = vec![0; N_COINS];
    let mut trade_pnls: Vec<Vec<f64>> = vec![Vec::new(); N_COINS]; // PnL % per closed trade
    let mut last_rebalance_bar: Vec<usize> = vec![0; N_COINS];

    // Stats
    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;
    let mut coin_wins = vec![0usize; N_COINS];
    let mut coin_losses = vec![0usize; N_COINS];
    let mut coin_flats = vec![0usize; N_COINS];

    let bars_per_year = 35040.0;
    let bars_per_rebal = cfg.freq as f64;

    for i in 1..n {
        // Process exits for all coins first
        for ci in 0..N_COINS {
            let d = &data[ci];
            if let Some((dir, entry)) = pos[ci] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

                if closed {
                    let bal = bals[ci];
                    let net = bal * BASE_POSITION_SIZE * LEVERAGE * exit_pct;
                    bals[ci] += net;
                    if net > 1e-10 {
                        total_wins += 1; coin_wins[ci] += 1;
                    } else if net < -1e-10 {
                        total_losses += 1; coin_losses[ci] += 1;
                    } else {
                        total_flats += 1; coin_flats[ci] += 1;
                    }
                    trade_pnls[ci].push(exit_pct);
                    pos[ci] = None;
                    cooldown[ci] = COOLDOWN;
                }
            } else if cooldown[ci] > 0 {
                cooldown[ci] -= 1;
            }
        }

        // Rebalance by Sharpe (only in non-baseline configs)
        if !cfg.is_baseline && cfg.freq > 0 {
            let bars_since_first = i.saturating_sub(last_rebalance_bar[0]);
            // Use bar 0 as reference for rebalance frequency (all coins same bar index)
            if bars_since_first >= cfg.freq {
                // Compute trailing Sharpe for each coin
                let mut sharpes: Vec<f64> = Vec::new();
                for ci in 0..N_COINS {
                    let sh = trailing_sharpe(&trade_pnls[ci], cfg.window);
                    sharpes.push(sh);
                }

                let sum_sharpe: f64 = sharpes.iter().sum();
                if sum_sharpe > 0.0 && sum_sharpe.is_finite() {
                    let total_portfolio: f64 = bals.iter().sum();
                    let base_cap = INITIAL_BAL;

                    // Target: weight by Sharpe
                    let mut targets: Vec<f64> = Vec::new();
                    for &sh in &sharpes {
                        let weight = sh / sum_sharpe;
                        let target = (total_portfolio * weight).max(base_cap * cfg.min_cap_mult).min(base_cap * cfg.max_cap_mult);
                        targets.push(target);
                    }

                    // Apply: move balance from over-allocated to under-allocated
                    for ci in 0..N_COINS {
                        let diff = targets[ci] - bals[ci];
                        bals[ci] += diff;
                    }
                }

                // Sync rebalance bar across all coins
                for ci in 0..N_COINS {
                    last_rebalance_bar[ci] = i;
                }
            }
        }

        // Process entries
        for ci in 0..N_COINS {
            if cooldown[ci] > 0 || pos[ci].is_some() { continue; }
            let d = &data[ci];
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        pos[ci] = Some((dir, entry_price));
                    }
                }
            }
        }
    }

    // Final PnL
    let mut total_pnl = 0.0;
    let mut coin_results: Vec<CoinResult> = Vec::new();
    for ci in 0..N_COINS {
        let pnl = bals[ci] - INITIAL_BAL;
        total_pnl += pnl;
        let trades = coin_wins[ci] + coin_losses[ci] + coin_flats[ci];
        let wr = if trades > 0 { coin_wins[ci] as f64 / trades as f64 * 100.0 } else { 0.0 };
        coin_results.push(CoinResult {
            coin: data[ci].name.clone(), pnl, trades,
            wins: coin_wins[ci], losses: coin_losses[ci], wr,
        });
    }

    (total_pnl, total_wins, total_losses, total_flats, coin_results)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN75 — Sharpe-Weighted Capital Allocation\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × portfolio", grid.len());

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![],
            };
        }
        let (total_pnl, wins, losses, flats, coin_results) = simulate_portfolio(&coin_data, cfg);
        let total_trades = wins + losses + flats;
        let portfolio_wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let losses_f = losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN75 Sharpe-Weighted Capital Allocation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<30} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(75));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<30} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN75 sharpe-weighted allocation. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run75_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run75_1_results.json");
}