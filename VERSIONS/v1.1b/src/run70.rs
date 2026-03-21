/// RUN70 — Z-Score Convergence Filter
///
/// Grid: CONVERGENCE_THRESHOLD [0.40, 0.50, 0.60, 0.70] × Z_CONVERGENCE_WINDOW [1, 2]
/// Total: 4 × 2 = 8 configs + baseline = 9
///
/// Convergence = % of coins with rising z-score (recovering toward mean)
/// Require convergence >= threshold for entry confirmation.
///
/// Run: cargo run --release --features run70 -- --run70

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct ConvCfg {
    conv_threshold: f64,
    conv_window: usize,
}

impl ConvCfg {
    fn label(&self) -> String {
        format!("T{:+.0}_W{}", self.conv_threshold * 100.0, self.conv_window)
    }
}

fn build_grid() -> Vec<ConvCfg> {
    let mut grid = vec![ConvCfg { conv_threshold: 0.0, conv_window: 1 }]; // baseline
    let thresholds = [0.40, 0.50, 0.60, 0.70];
    let windows = [1, 2];
    for &t in &thresholds {
        for &w in &windows {
            grid.push(ConvCfg { conv_threshold: t, conv_window: w });
        }
    }
    grid
}

struct CoinData15m {
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
    pf: f64,
    is_baseline: bool,
    block_rate: f64,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
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
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData15m { closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Portfolio-level simulation with convergence filter
fn simulate_portfolio(data: &[CoinData15m], cfg: ConvCfg) -> (f64, usize, usize, usize, usize) {
    let n = data[0].closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldown = vec![0usize; N_COINS];
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut total_blocked = 0usize;
    let mut _total_entries = 0usize;

    for i in 1..n {
        // Step 1: compute convergence (how many coins have rising z-score)
        let mut rising = 0usize;
        let mut valid = 0usize;
        for d in data {
            if i >= 1 && !d.zscore[i].is_nan() && !d.zscore[i-1].is_nan() {
                valid += 1;
                if d.zscore[i] > d.zscore[i-1] {
                    rising += 1;
                }
            }
        }
        let conv_pct = if valid > 0 { rising as f64 / valid as f64 } else { 0.0 };

        // Step 2: process exits
        for ci in 0..N_COINS {
            let d = &data[ci];
            if let Some((dir, entry)) = pos[ci] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let z = d.zscore[i];
                    let new_dir = regime_signal(z);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= 2 { exit_pct = pct; closed = true; }

                if closed {
                    let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                    bal += net;
                    if net > 1e-10 { wins += 1; }
                    else if net < -1e-10 { losses += 1; }
                    else { flats += 1; }
                    pos[ci] = None;
                    cooldown[ci] = COOLDOWN;
                }
            }
        }

        // Step 3: process entries
        for ci in 0..N_COINS {
            if cooldown[ci] > 0 {
                cooldown[ci] -= 1;
                continue;
            }
            if pos[ci].is_some() { continue; }

            let d = &data[ci];
            if let Some(dir) = regime_signal(d.zscore[i]) {
                // Check convergence filter
                if cfg.conv_threshold > 0.0 {
                    // Compute convergence over conv_window bars ending at i
                    let mut rising_count = 0usize;
                    let mut valid_count = 0usize;
                    for w_i in (i+1-cfg.conv_window).max(1)..=i {
                        if w_i >= 1 && !d.zscore[w_i].is_nan() && w_i > 0 && !d.zscore[w_i-1].is_nan() {
                            valid_count += 1;
                            if d.zscore[w_i] > d.zscore[w_i-1] {
                                rising_count += 1;
                            }
                        }
                    }
                    let conv = if valid_count > 0 { rising_count as f64 / valid_count as f64 } else { 0.0 };
                    if conv < cfg.conv_threshold {
                        total_blocked += 1;
                        continue;
                    }
                }

                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 {
                        pos[ci] = Some((dir, entry_price));
                        _total_entries += 1;
                    }
                }
            }
        }
    }

    (bal - INITIAL_BAL, wins, losses, flats, total_blocked)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN70 — Z-Score Convergence Filter\n");
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
    eprintln!("\nGrid: {} configs × portfolio", grid.len());

    // Run baseline first (no parallelism since it's portfolio-level)
    let baseline_result;
    {
        let cfg = grid[0];
        let (pnl, wins, losses, flats, blocked) = simulate_portfolio(&coin_data, cfg);
        let total_trades = wins + losses + flats;
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if losses > 0 { gross / (losses as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let attempted = wins + losses + flats + blocked;
        let block_rate = if attempted > 0 { blocked as f64 / attempted as f64 * 100.0 } else { 0.0 };
        baseline_result = ConfigResult {
            label: "BASELINE".to_string(), total_pnl: pnl, portfolio_wr: wr,
            total_trades, pf, is_baseline: true, block_rate,
        };
        eprintln!("  Baseline  PnL={:>+8.2}  WR={:>5.1}%  trades={}  blocked={}", pnl, wr, total_trades, blocked);
    }

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len() - 1;

    let mut results: Vec<ConfigResult> = vec![baseline_result];

    for &cfg in &grid[1..] {
        if shutdown.load(Ordering::SeqCst) { break; }
        let (pnl, wins, losses, flats, blocked) = simulate_portfolio(&coin_data, cfg);
        let total_trades = wins + losses + flats;
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if losses > 0 { gross / (losses as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let attempted = wins + losses + flats + blocked;
        let block_rate = if attempted > 0 { blocked as f64 / attempted as f64 * 100.0 } else { 0.0 };
        let total_entries = wins + losses + flats + blocked;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  blocked={} ({:.1}%)",
            d, total_cfgs, cfg.label(), pnl, wr, total_trades, blocked, block_rate);
        results.push(ConfigResult {
            label: cfg.label(), total_pnl: pnl, portfolio_wr: wr,
            total_trades, pf, is_baseline: false, block_rate,
        });
    }

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = &results[0];
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN70 Z-Score Convergence Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<10} {:>8} {:>8} {:>6}  {:>6}  {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "BlockRate%");
    println!("{}", "-".repeat(60));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<10} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}  {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.block_rate);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN70 z-convergence. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run70_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run70_1_results.json");
}