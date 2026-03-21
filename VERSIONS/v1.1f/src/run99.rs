/// RUN99 — Z-Score Momentum Divergence: Exit When Z-Score and Price Momentum Diverge
///
/// Grid: THRESHOLD [-0.3, -0.5, -0.7, -1.0] × BARS [2, 3, 5] = 20 configs
/// 20 portfolio-level configs × 18 coins
///
/// Run: cargo run --release --features run99 -- --run99

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

const Z_BREADTH_LONG: usize = 20;
const Z_BREADTH_SHORT: usize = 50;

#[derive(Clone, Copy, PartialEq, Debug)]
struct DivergenceCfg {
    threshold: f64,
    bars: usize,
    is_baseline: bool,
}

impl DivergenceCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("T{:.1}_B{}", self.threshold.abs(), self.bars) }
    }
}

fn build_grid() -> Vec<DivergenceCfg> {
    let mut grid = vec![DivergenceCfg { threshold: 0.0, bars: 0, is_baseline: true }];
    let threshs = [-0.3, -0.5, -0.7, -1.0];
    let bar_vals = [2usize, 3, 5];
    for &t in &threshs {
        for &b in &bar_vals {
            grid.push(DivergenceCfg { threshold: t, bars: b, is_baseline: false });
        }
    }
    grid
}

struct CoinData {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    wins: usize,
    losses: usize,
    pf: f64,
    divergence_exits: usize,
    is_baseline: bool,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
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
    Some(CoinData { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Portfolio-level simulation
fn simulate_portfolio(data: &[&CoinData], cfg: &DivergenceCfg) -> ConfigResult {
    let n = data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64, usize, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut original_z: Vec<Option<f64>> = vec![None; N_COINS];
    // Track consecutive divergence bars per coin
    let mut div_count: Vec<usize> = vec![0usize; N_COINS];

    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;
    let mut divergence_exits = 0usize;

    for i in 25..n {
        // Compute market mode at this bar
        let mut extreme = 0usize;
        for d in data {
            if i < d.zscore.len() && !d.zscore[i].is_nan() && d.zscore[i].abs() > 2.0 {
                extreme += 1;
            }
        }
        let breadth = extreme * 100 / N_COINS;
        let mode = if breadth <= Z_BREADTH_LONG { 1i8 } else if breadth >= Z_BREADTH_SHORT { -1i8 } else { 0 };

        for c in 0..N_COINS {
            let d = data[c];

            if let Some((dir, entry, entry_bar, z_entry)) = positions[c] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
                if !closed && i >= entry_bar + MIN_HOLD_BARS {
                    // Z0 exit check
                    if (dir == 1 && d.zscore[i] >= 0.0) || (dir == -1 && d.zscore[i] <= 0.0) {
                        exit_pct = pct; closed = true;
                    }
                }

                // Divergence exit check
                if !closed && !cfg.is_baseline && i >= entry_bar + 5 {
                    // z_momentum = z_current - z_N_bars_ago (N=5)
                    let lookback = 5;
                    if i >= lookback + 1 {
                        let z_now = d.zscore[i];
                        let z_then = d.zscore[i - lookback];
                        let z_momentum = z_now - z_then;

                        // price momentum: for long, positive if price moved up
                        let price_now = d.closes[i];
                        let price_then = d.closes[i - lookback];
                        let price_momentum = (price_now - price_then) / price_then;

                        // For LONG: expect z to recover toward 0 (z_momentum > 0 for long)
                        // Negative z_momentum = z diverging (not recovering despite price movement)
                        let div_signal = match dir {
                            1 => {
                                // LONG: z should be recovering (less negative) as price rises
                                // Negative z_momentum while price rising = divergence
                                if price_momentum > 0.0 && z_momentum < cfg.threshold {
                                    div_count[c] += 1;
                                } else {
                                    div_count[c] = 0;
                                }
                                div_count[c] >= cfg.bars
                            }
                            -1 => {
                                // SHORT: z should be recovering (less positive) as price falls
                                if price_momentum < 0.0 && z_momentum > -cfg.threshold {
                                    div_count[c] += 1;
                                } else {
                                    div_count[c] = 0;
                                }
                                div_count[c] >= cfg.bars
                            }
                            _ => false,
                        };

                        if div_signal {
                            exit_pct = pct; closed = true; divergence_exits += 1; div_count[c] = 0;
                        }
                    }
                }

                if closed {
                    let net = balances[c] * POSITION_SIZE * LEVERAGE * exit_pct;
                    balances[c] += net;
                    if net > 1e-10 { total_wins += 1; }
                    else if net < -1e-10 { total_losses += 1; }
                    else { total_flats += 1; }
                    original_z[c] = Some(z_entry);
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                    div_count[c] = 0;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    let mode_ok = match mode {
                        1 => dir == 1,
                        -1 => dir == -1,
                        _ => true,
                    };
                    if mode_ok && i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            positions[c] = Some((dir, entry_price, i, d.zscore[i]));
                            original_z[c] = None;
                            div_count[c] = 0;
                        }
                    }
                }
            }
        }
    }

    let total_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    let total_trades = total_wins + total_losses + total_flats;
    let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = POSITION_SIZE * LEVERAGE * SL_PCT * total_wins as f64;
    let avg_loss = POSITION_SIZE * LEVERAGE * SL_PCT * total_losses as f64;
    let pf = if total_losses > 0 { avg_win / avg_loss } else { 0.0 };

    ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, divergence_exits, is_baseline: cfg.is_baseline }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN99 — Z-Score Momentum Divergence Grid Search\n");
    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let coin_refs: Vec<&CoinData> = data.iter().collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} portfolio-level configs", grid.len());

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, divergence_exits: 0, is_baseline: cfg.is_baseline };
        }
        let r = simulate_portfolio(&coin_refs, cfg);
        eprintln!("  {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  div_exits={}",
            cfg.label(), r.total_pnl, r.portfolio_wr, r.total_trades, r.divergence_exits);
        r
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN99 Z-Score Momentum Divergence Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<15} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "DivExits");
    println!("{}", "-".repeat(65));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<15} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.divergence_exits);
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN99 z-momentum divergence. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run99_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run99_1_results.json");
}
