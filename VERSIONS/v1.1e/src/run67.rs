/// RUN67 — Scalp Entry Z-Score Threshold Tightening
///
/// Grid: SCALP_Z_THRESHOLD [1.5, 1.75, 2.0, 2.25, 2.5] × baseline
/// Baseline: no z-score filter (scalp uses strategy-specific indicators only)
/// Total: 5 configs + baseline = 6
///
/// Scalp: LONG when z < -z_thresh AND RSI < 30 AND vol_spike
///        SHORT when z > z_thresh AND RSI > 70 AND vol_spike
/// SL=0.10%, TP=0.80%
///
/// Run: cargo run --release --features run67 -- --run67

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SCALP_SL: f64 = 0.001;  // 0.10%
const SCALP_TP: f64 = 0.008;  // 0.80%
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct ScalpZCfg {
    z_threshold: f64, // 0 = disabled (baseline)
}

impl ScalpZCfg {
    fn label(&self) -> String {
        if self.z_threshold == 0.0 { "DISABLED".to_string() }
        else { format!("Z{:+.1}", self.z_threshold) }
    }
}

fn build_grid() -> Vec<ScalpZCfg> {
    let mut grid = vec![ScalpZCfg { z_threshold: 0.0 }]; // baseline
    let thresholds: [f64; 5] = [1.5, 1.75, 2.0, 2.25, 2.5];
    for z in thresholds {
        grid.push(ScalpZCfg { z_threshold: z });
    }
    grid
}

struct CoinData1m {
    name: String,
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    volumes: Vec<f64>,
    zscore: Vec<f64>,
    rsi: Vec<f64>,
    vol_ma: Vec<f64>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    blocked: usize,
    tp_hits: usize,
    sl_hits: usize,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_1m(coin: &str) -> Option<CoinData1m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut closes = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut volumes = Vec::new();

    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let _o: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if cc.is_nan() || hh.is_nan() || ll.is_nan() || vv.is_nan() { continue; }
        closes.push(cc); highs.push(hh); lows.push(ll); volumes.push(vv);
    }

    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score (20-period on closes)
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // RSI(14)
    let mut rsi = vec![f64::NAN; n];
    for i in 14..n {
        let mut gain_sum = 0.0;
        let mut loss_sum = 0.0;
        for j in i+1-14..=i {
            let delta = closes[j] - closes[j-1];
            if delta > 0.0 { gain_sum += delta; }
            else { loss_sum += -delta; }
        }
        let avg_gain = gain_sum / 14.0;
        let avg_loss = loss_sum / 14.0;
        rsi[i] = if avg_loss == 0.0 { 100.0 } else { 100.0 - 100.0/(1.0 + avg_gain/avg_loss) };
    }

    // Volume MA (20-period)
    let mut vol_ma = vec![f64::NAN; n];
    for i in 20..n {
        vol_ma[i] = volumes[i+1-20..=i].iter().sum::<f64>() / 20.0;
    }

    Some(CoinData1m { name: coin.to_string(), closes, highs, lows, volumes, zscore, rsi, vol_ma })
}

// Check if volume spike (volume > 1.5× volume MA)
fn vol_spike(d: &CoinData1m, i: usize) -> bool {
    if d.vol_ma[i].is_nan() || d.vol_ma[i] == 0.0 { return false; }
    d.volumes[i] > d.vol_ma[i] * 1.5
}

fn scalp_entry(d: &CoinData1m, i: usize, cfg: ScalpZCfg) -> Option<i8> {
    if i < 25 || d.zscore[i].is_nan() || d.rsi[i].is_nan() { return None; }

    let z = d.zscore[i];
    let rsi = d.rsi[i];
    let spike = vol_spike(d, i);

    // LONG: extreme oversold
    if z < -2.0 && rsi < 30.0 && spike {
        if cfg.z_threshold == 0.0 || z < -cfg.z_threshold {
            return Some(1);
        }
    }
    // SHORT: extreme overbought
    if z > 2.0 && rsi > 70.0 && spike {
        if cfg.z_threshold == 0.0 || z > cfg.z_threshold {
            return Some(-1);
        }
    }
    None
}

// Count blocked entries (entries that pass base conditions but fail z-threshold)
fn scalp_entry_blocked(d: &CoinData1m, i: usize, cfg: ScalpZCfg) -> bool {
    if i < 25 || d.zscore[i].is_nan() || d.rsi[i].is_nan() { return false; }
    let z = d.zscore[i];
    let rsi = d.rsi[i];
    let spike = vol_spike(d, i);

    // Base conditions pass but z-threshold blocks
    if z < -2.0 && rsi < 30.0 && spike && cfg.z_threshold > 0.0 && z >= -cfg.z_threshold {
        return true;
    }
    if z > 2.0 && rsi > 70.0 && spike && cfg.z_threshold > 0.0 && z <= cfg.z_threshold {
        return true;
    }
    false
}

fn simulate(d: &CoinData1m, cfg: ScalpZCfg) -> (f64, usize, usize, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None; // dir, entry_price
    let mut blocked = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut tp_hits = 0usize;
    let mut sl_hits = 0usize;

    // For speed: step through every 5th bar for entry signals
    // scalp trades last minutes to hours, we can sample at 5-bar intervals
    let step = 5;

    for i in (step..n).step_by(step) {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };

            let mut closed = false;
            let mut exit_pct = 0.0;

            // TP check first
            if pct >= SCALP_TP { exit_pct = SCALP_TP; closed = true; tp_hits += 1; }
            // SL check
            else if pct <= -SCALP_SL { exit_pct = -SCALP_SL; closed = true; sl_hits += 1; }

            if closed {
                let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
            }
        } else {
            // Check for entry
            let entry = scalp_entry(d, i, cfg);
            if entry.is_none() && cfg.z_threshold > 0.0 {
                // Count blocked entries for z-threshold configs
                if scalp_entry_blocked(d, i, cfg) { blocked += 1; }
            }
            if let Some(dir) = entry {
                if i + 1 < n {
                    let entry_price = d.closes[i + 1]; // enter next bar
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }

    (bal - INITIAL_BAL, wins, losses, flats, blocked, tp_hits, sl_hits)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN67 — Scalp Z-Score Threshold\n");
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData1m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData1m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0,
                total_trades: 0, pf: 0.0, is_baseline: cfg.z_threshold == 0.0,
                blocked: 0, tp_hits: 0, sl_hits: 0,
            };
        }
        let mut total_pnl = 0.0;
        let mut wins_sum = 0usize;
        let mut losses_sum = 0usize;
        let mut flats_sum = 0usize;
        let mut blocked_sum = 0usize;
        let mut tp_sum = 0usize;
        let mut sl_sum = 0usize;

        for d in &coin_data {
            let (pnl, wins, losses, flats, blocked, tp, sl) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins;
            losses_sum += losses;
            flats_sum += flats;
            blocked_sum += blocked;
            tp_sum += tp;
            sl_sum += sl;
        }

        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross_win = wins_sum as f64 * SCALP_TP * BASE_POSITION_SIZE * LEVERAGE;
        let gross_loss = losses_sum as f64 * SCALP_SL * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  blocked={}  TP={}  SL={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades, blocked_sum, tp_sum, sl_sum);

        ConfigResult {
            label: cfg.label(),
            total_pnl,
            portfolio_wr,
            total_trades,
            pf,
            is_baseline: cfg.z_threshold == 0.0,
            blocked: blocked_sum,
            tp_hits: tp_sum,
            sl_hits: sl_sum,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN67 Scalp Z-Score Threshold Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<10} {:>8} {:>8} {:>6}  {:>6}  {:>7}  {:>6}  {:>6}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Blocked", "TP", "SL");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<10} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}  {:>7}  {:>6}  {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.blocked, r.tp_hits, r.sl_hits);
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN67 scalp z-threshold. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run67_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run67_1_results.json");
}