/// RUN94 — Partial Reentry After Cooldown
///
/// Allow partial re-entry during cooldown if z-score is more extreme:
/// - REENTRY_Z_MULT: z must be this much more extreme vs original entry
/// - REENTRY_SIZE_PCT: re-entry at this fraction of normal size
/// - REENTRY_MAX_COUNT: max consecutive re-entries per coin
///
/// Grid: Z_MULT [1.1, 1.2, 1.3, 1.5] × SIZE_PCT [0.3, 0.5, 0.7] × MAX_COUNT [1, 2, 3]
/// Total: 4 × 3 × 3 = 36 + baseline = 37 configs
///
/// Run: cargo run --release --features run94 -- --run94

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

#[derive(Clone, Debug)]
struct ReentryCfg {
    z_mult: f64,
    size_pct: f64,
    max_count: usize,
    is_baseline: bool,
}

impl ReentryCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("ZM{:.1}_SP{:.1}_MC{}", self.z_mult, self.size_pct, self.max_count) }
    }
}

fn build_grid() -> Vec<ReentryCfg> {
    let mut grid = vec![ReentryCfg { z_mult: 0.0, size_pct: 1.0, max_count: 0, is_baseline: true }];
    let zms = [1.1, 1.2, 1.3, 1.5];
    let sps = [0.3, 0.5, 0.7];
    let mcs = [1usize, 2, 3];
    for &zm in &zms {
        for &sp in &sps {
            for &mc in &mcs {
                grid.push(ReentryCfg { z_mult: zm, size_pct: sp, max_count: mc, is_baseline: false });
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
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, reentries: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, total_reentries: usize, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: &ReentryCfg) -> (f64, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, entry, entry_bar, z_at_entry
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut reentries = 0usize;
    // Per-coin reentry tracking
    let mut reentry_count = 0usize;
    let mut original_z: Option<f64> = None;
    let mut pos_was_reentry = false;
    let mut pos_size_mult = 1.0;

    for i in 1..n {
        if let Some((dir, entry, entry_bar, z_entry)) = pos {
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
                let effective_size = POSITION_SIZE * pos_size_mult;
                let net = bal * effective_size * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                original_z = Some(z_entry);
                pos = None;
                cooldown = COOLDOWN;
                reentry_count = 0;
                pos_was_reentry = false;
                pos_size_mult = 1.0;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
            // Check for re-entry during cooldown
            if !cfg.is_baseline && cfg.z_mult > 0.0 {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if let Some(orig_z) = original_z {
                        if reentry_count < cfg.max_count {
                            let threshold = orig_z * cfg.z_mult;
                            let can_reenter = match dir {
                                1 => d.zscore[i] < threshold, // more negative for longs
                                -1 => d.zscore[i] > threshold, // more positive for shorts
                                _ => false,
                            };
                            if can_reenter && i + 1 < n {
                                let entry_price = d.opens[i + 1];
                                if entry_price > 0.0 {
                                    pos = Some((dir, entry_price, i, d.zscore[i]));
                                    reentry_count += 1;
                                    reentries += 1;
                                    pos_was_reentry = true;
                                    pos_size_mult = cfg.size_pct;
                                    cooldown = 0; // cancel remaining cooldown
                                }
                            }
                        }
                    }
                }
            }
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        pos = Some((dir, entry_price, i, d.zscore[i]));
                        original_z = None;
                        reentry_count = 0;
                        pos_was_reentry = false;
                        pos_size_mult = 1.0;
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    (pnl, total_trades, wins, losses, reentries)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN94 — Partial Reentry After Cooldown\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, total_reentries: 0, coins: vec![]
            };
        }
        let coin_results: Vec<CoinResult> = (0..N_COINS).map(|c| {
            let (pnl, trades, wins, losses, reentries) = simulate(&coin_data[c], cfg);
            CoinResult {
                coin: coin_data[c].name.clone(),
                pnl,
                trades,
                wins,
                losses,
                wr: if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 },
                reentries,
            }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = total_losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let total_reentries: usize = coin_results.iter().map(|c| c.reentries).sum();

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  reentries={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, total_reentries);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
            is_baseline: cfg.is_baseline, total_reentries, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN94 Partial Reentry After Cooldown Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<25} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<25} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN94 partial reentry. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run94_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run94_1_results.json");
}
