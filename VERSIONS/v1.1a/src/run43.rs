/// RUN43 — Breadth Momentum Filter: Anticipating Regime Transitions
///
/// Hypothesis: Breadth velocity (rate of change) predicts regime transitions
/// better than static breadth levels.
///
/// Grid:
///   BREADTH_VEL_WINDOW:  [2, 4, 6, 8] bars
///   BREADTH_VEL_THRESH:  [0.05, 0.10, 0.15, 0.20] (fraction of breadth change per bar)
///   = 16 configs
///
/// Run: cargo run --release --features run43 -- --run43

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: u32 = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Grid ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct BreadthCfg {
    vel_window: usize,   // bars to look back for velocity
    vel_thresh: f64,    // breadth must change by this fraction to trigger
}

impl BreadthCfg {
    fn label(&self) -> String {
        format!("VW{}_VT{:.2}", self.vel_window, self.vel_thresh)
    }
}

fn build_grid() -> Vec<BreadthCfg> {
    let mut grid = Vec::new();
    // Baseline: no filter
    grid.push(BreadthCfg { vel_window: 999, vel_thresh: 999.0 });
    let vel_windows = [2usize, 4, 6, 8];
    let vel_threshs = [0.05f64, 0.10, 0.15, 0.20];
    for vw in vel_windows {
        for vt in vel_threshs {
            grid.push(BreadthCfg { vel_window: vw, vel_thresh: vt });
        }
    }
    grid
}

// ── Data structures ─────────────────────────────────────────────────────────
struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    uptrend: Vec<bool>,  // true if z-score < -1.5 (oversold, potential long)
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    wr: f64,
    pf: f64,
    delta_pnl: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    total_delta: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

// ── Helpers ─────────────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s=&data[i+1-w..=i]; let m=s.iter().sum::<f64>()/w as f64; let v=s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}

// ── CSV loader ────────────────────────────────────────────────────────────────
fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || hh.is_nan() || ll.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let sma = rmean(&closes, 20);
    let mut zscore = vec![f64::NAN; n];
    let mut uptrend = vec![false; n];
    for i in 20..n {
        if !sma[i].is_nan() {
            let window = &closes[i+1-20..=i];
            let mean = window.iter().sum::<f64>()/20.0;
            let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
            zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
            uptrend[i] = zscore[i] < -1.5; // oversold = potential long signal
        }
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, uptrend, zscore })
}

// ── Regime signal ────────────────────────────────────────────────────────────
fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// ── Simulation with breadth velocity filter ────────────────────────────────────
// coin_upt: for each bar, which coins are in uptrend (true/false per coin)
// breadth[i]: fraction of coins in uptrend at bar i
// breadth_vel[i]: change in breadth over vel_window bars
fn simulate_with_filter(
    coin_data: &[CoinData15m],
    cfg: BreadthCfg,
    breadth: &[f64],
) -> (f64, usize, usize, usize, usize, Vec<f64>, Vec<f64>) {
    // Simulate across all coins using shared breadth signal
    let n = coin_data[0].closes.len();
    let mut total_pnl = 0.0;
    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;
    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut coin_states: Vec<Option<(i8, f64, f64)>> = vec![None; coin_data.len()];
    let mut coin_cooldowns: Vec<usize> = vec![0usize; coin_data.len()];

    let is_baseline = cfg.vel_window == 999;

    for i in 1..n {
        // Compute breadth velocity
        let vel = if is_baseline || i < cfg.vel_window {
            0.0
        } else {
            let prev_idx = i - cfg.vel_window;
            breadth[i] - breadth[prev_idx]
        };

        let suppress_long = !is_baseline && vel > cfg.vel_thresh && breadth[i] > 0.15;
        let suppress_short = !is_baseline && vel < -cfg.vel_thresh && breadth[i] < 0.25;

        for (ci, d) in coin_data.iter().enumerate() {
            // Handle existing position
            if let Some((dir, entry, _notional)) = coin_states[ci] {
                let pct = if dir == 1 {
                    (d.closes[i] - entry) / entry
                } else {
                    (entry - d.closes[i]) / entry
                };
                let mut closed = false;
                let mut exit_pct = 0.0;
                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed && i > 0 {
                    let new_dir = regime_signal(d, i);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= 20 { exit_pct = pct; closed = true; }

                if closed {
                    let notional = INITIAL_BAL * POSITION_SIZE * LEVERAGE;
                    let net = notional * exit_pct;
                    total_pnl += net;
                    if net > 1e-10 { win_pnls.push(net); total_wins += 1; }
                    else if net < -1e-10 { loss_pnls.push(net); total_losses += 1; }
                    else { total_flats += 1; }
                    coin_states[ci] = None;
                    coin_cooldowns[ci] = COOLDOWN;
                }
            } else if coin_cooldowns[ci] > 0 {
                coin_cooldowns[ci] -= 1;
            } else {
                // Breadth velocity filter
                if suppress_long || suppress_short { continue; }

                if let Some(dir) = regime_signal(d, i) {
                    if i+1 < d.closes.len() {
                        let entry_price = d.opens[i+1];
                        if entry_price > 0.0 {
                            coin_states[ci] = Some((dir, entry_price, 0.0));
                        }
                    }
                }
            }
        }
    }

    // Close open positions
    for (ci, d) in coin_data.iter().enumerate() {
        if let Some((dir, entry, _)) = coin_states[ci] {
            let pct = if dir == 1 {
                (d.closes[n-1] - entry) / entry
            } else {
                (entry - d.closes[n-1]) / entry
            };
            let notional = INITIAL_BAL * POSITION_SIZE * LEVERAGE;
            let net = notional * pct;
            total_pnl += net;
            if net > 1e-10 { win_pnls.push(net); total_wins += 1; }
            else if net < -1e-10 { loss_pnls.push(net); total_losses += 1; }
            else { total_flats += 1; }
        }
    }

    (total_pnl, total_wins, total_losses, total_flats, total_wins + total_losses + total_flats, win_pnls, loss_pnls)
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN43 — Breadth Momentum Filter Grid Search\n");

    // Load data
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    let all_ok = raw_data.iter().all(|r| r.is_some());
    if !all_ok { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let n = coin_data[0].closes.len();

    // Compute breadth: fraction of coins in uptrend at each bar
    let mut breadth = vec![0.0f64; n];
    for i in 0..n {
        let mut up = 0usize;
        for d in &coin_data {
            if d.uptrend[i] { up += 1; }
        }
        breadth[i] = up as f64 / N_COINS as f64;
    }
    eprintln!("\nBreadth: min={:.2} max={:.2} mean={:.2}", breadth.iter().cloned().fold(f64::INFINITY, f64::min), breadth.iter().cloned().fold(f64::NEG_INFINITY, f64::max), breadth.iter().sum::<f64>()/n as f64);

    // Build grid
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × 1 portfolio simulation", grid.len());

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    // Baseline: simulate with no filter
    let baseline = simulate_with_filter(&coin_data, BreadthCfg { vel_window: 999, vel_thresh: 999.0 }, &breadth);
    let baseline_pnl = baseline.0;
    let baseline_trades = baseline.4;
    let baseline_wr = if baseline_trades > 0 { baseline.1 as f64 / baseline.4 as f64 * 100.0 } else { 0.0 };
    eprintln!("\nBaseline: PnL={:+.2} WR={:.1}% trades={}", baseline_pnl, baseline_wr, baseline_trades);

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, total_delta: 0.0,
                portfolio_wr: 0.0, total_trades: 0, pf: 0.0,
                is_baseline: cfg.vel_window == 999, coins: vec![],
            };
        }

        let (pnl, wins, losses, _flats, trades, win_pnls, loss_pnls) =
            simulate_with_filter(&coin_data, *cfg, &breadth);

        let total_trades = trades;
        let portfolio_wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win = if wins > 0 { win_pnls.iter().sum::<f64>() / wins as f64 } else { 0.0 };
        let avg_loss = if losses > 0 { loss_pnls.iter().sum::<f64>() / losses as f64 } else { 0.0 };
        let pf = if avg_loss.abs() > 1e-8 { avg_win / avg_loss.abs() } else { 0.0 };
        let is_baseline = cfg.vel_window == 999;

        // Per-coin results (simplified — all coins share the same breadth signal)
        let coin_results: Vec<CoinResult> = coin_data.iter().enumerate().map(|(_ci, d)| {
            CoinResult {
                coin: d.name.clone(),
                pnl: 0.0, // portfolio-level only
                trades: 0,
                wins: 0,
                losses: 0,
                wr: 0.0,
                pf: 0.0,
                delta_pnl: 0.0,
            }
        }).collect();

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        let delta = pnl - baseline_pnl;
        eprintln!("  [{:>2}/{}] {:<18} PnL={:>+8.2} Δ={:>+7.2} WR={:>5.1}% trades={}",
            d, total_cfgs, cfg.label(), pnl, delta, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl: pnl, total_delta: delta,
            portfolio_wr, total_trades, pf, is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN43 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run43_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    eprintln!();
    let baseline_cfg = results.iter().find(|r| r.is_baseline).unwrap();

    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_delta.partial_cmp(&a.total_delta).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN43 Breadth Momentum Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline_cfg.total_pnl, baseline_cfg.portfolio_wr, baseline_cfg.total_trades);
    println!("\n{:>3}  {:<18} {:>8} {:>8} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(65));
    for (i,r) in sorted.iter().enumerate() {
        println!("{:>3}  {:<18} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}",
            i+1, r.label, r.total_pnl, r.total_delta, r.portfolio_wr, r.total_trades);
        if i >= 19 { println!("  ... ({} more)", sorted.len()-20); break; }
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.total_delta > 0.0;
    println!("\nVERDICT: {} (best ΔPnL={:+.2})",
        if is_positive { "POSITIVE" } else { "NEGATIVE" },
        best.total_delta);

    let notes = format!("RUN43 breadth momentum filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:.2})",
        results.len(), baseline_cfg.total_pnl, best.label, best.total_pnl, best.total_delta);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run43_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run43_1_results.json");
}
