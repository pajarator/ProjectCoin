/// RUN58 — CME Gap Fill Strategy
///
/// Grid: GAP_THRESHOLD [0.005, 0.008, 0.010, 0.015, 0.020] × SL [0.003, 0.005, 0.007] × TP [0.010, 0.015, 0.020, 0.025] × MAX_HOLD [24, 48, 72, 96]
/// Total: 240 configs + baseline = 241
/// Note: Limited to coins with identifiable Friday-Monday gap structure
///
/// Run: cargo run --release --features run58 -- --run58

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use chrono::Datelike;

const INITIAL_BAL: f64 = 100.0;
const LEVERAGE: f64 = 5.0;
const BASE_POSITION_SIZE: f64 = 0.02;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct GapCfg {
    gap_threshold: f64,
    sl: f64,
    tp: f64,
    max_hold: usize,
}

impl GapCfg {
    fn label(&self) -> String {
        if self.gap_threshold == 0.0 { "DISABLED".to_string() }
        else { format!("GT{:.3}_SL{:.3}_TP{:.3}_MH{}", self.gap_threshold, self.sl, self.tp, self.max_hold) }
    }
}

fn build_grid() -> Vec<GapCfg> {
    let mut grid = vec![GapCfg { gap_threshold: 0.0, sl: 0.005, tp: 0.015, max_hold: 48 }]; // baseline (disabled)
    let thresholds = [0.005, 0.008, 0.010, 0.015, 0.020];
    let sls = [0.003, 0.005, 0.007];
    let tps = [0.010, 0.015, 0.020, 0.025];
    let holds = [24, 48, 72, 96];
    for &gt in &thresholds {
        for &sl in &sls {
            for &tp in &tps {
                for &mh in &holds {
                    grid.push(GapCfg { gap_threshold: gt, sl, tp, max_hold: mh });
                }
            }
        }
    }
    grid
}

struct CoinData15m { name: String, timestamps: Vec<String>, opens: Vec<f64>, closes: Vec<f64>, highs: Vec<f64>, lows: Vec<f64> }
#[derive(Serialize)] struct ConfigResult { label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize, pf: f64, is_baseline: bool }
#[derive(Serialize)] struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut timestamps = Vec::new(); let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let ts = it.next()?.to_string();
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        timestamps.push(ts); opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
    }
    if closes.len() < 100 { return None; }
    Some(CoinData15m { name: coin.to_string(), timestamps, opens, closes, highs, lows })
}

fn identify_friday_monday_gaps(d: &CoinData15m) -> Vec<(usize, f64, f64)> {
    // Find gaps: Monday open vs Friday close
    // A "Monday" bar has timestamp starting with weekday 0 (Monday)
    let mut gaps = Vec::new();
    let n = d.timestamps.len();
    for i in 1..n {
        let curr_ts = &d.timestamps[i];
        let prev_ts = &d.timestamps[i-1];
        // Check if current is Monday (weekday 0) and prev is Friday (weekday 4)
        // Format: "YYYY-MM-DD HH:MM:SS"
        if curr_ts.len() < 10 || prev_ts.len() < 10 { continue; }
        let curr_date = &curr_ts[..10];
        let prev_date = &prev_ts[..10];
        let curr_weekday = match chrono::NaiveDate::parse_from_str(curr_date, "%Y-%m-%d") {
            Ok(d) => d.weekday(),
            Err(_) => continue,
        };
        let prev_weekday = match chrono::NaiveDate::parse_from_str(prev_date, "%Y-%m-%d") {
            Ok(d) => d.weekday(),
            Err(_) => continue,
        };
        // weekday(): Monday=0, ..., Friday=4, Saturday=5, Sunday=6
        if curr_weekday == chrono::Weekday::Mon && prev_weekday == chrono::Weekday::Fri {
            let friday_close = d.closes[i-1];
            let monday_open = d.opens[i];
            let gap_pct = (monday_open - friday_close) / friday_close;
            gaps.push((i, gap_pct, friday_close));
        }
    }
    gaps
}

fn simulate_gap_fill(d: &CoinData15m, cfg: GapCfg) -> (f64, usize, usize, usize) {
    if cfg.gap_threshold == 0.0 {
        // Baseline: no gap fill trading
        return (0.0, 0, 0, 0);
    }

    let gaps = identify_friday_monday_gaps(d);
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

    for &(gap_idx, gap_pct, _friday_close) in &gaps {
        if gap_pct.abs() < cfg.gap_threshold { continue; }

        // Direction: fade the gap (trade opposite to gap direction)
        let dir: i8 = if gap_pct > 0.0 { -1 } else { 1 }; // short on gap up, long on gap down
        let entry_price = d.opens[gap_idx]; // enter at Monday open
        let target_pct = gap_pct.abs().min(cfg.tp); // target: fill the gap (capped at TP)

        let mut held = 0usize;
        let mut closed = false;
        let mut exit_pct = 0.0;

        for j in gap_idx..n.min(gap_idx + cfg.max_hold + 1) {
            held += 1;
            let pct = if dir == 1 { (d.closes[j]-entry_price)/entry_price } else { (entry_price-d.closes[j])/entry_price };

            // Check SL
            if pct <= -cfg.sl { exit_pct = -cfg.sl; closed = true; break; }
            // Check TP (filled the gap)
            if pct >= target_pct { exit_pct = target_pct; closed = true; break; }
        }

        // Time exit
        if !closed { exit_pct = if dir == 1 { (d.closes[n.min(gap_idx + cfg.max_hold)]-entry_price)/entry_price } else { (entry_price-d.closes[n.min(gap_idx + cfg.max_hold)])/entry_price }; }

        let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
        bal += net;
        if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
    }

    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN58 — CME Gap Fill Strategy\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded {
            let gaps = identify_friday_monday_gaps(data);
            eprintln!("  {} — {} bars, {} Friday-Monday gaps", name, data.closes.len(), gaps.len());
        }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    // Count total gap opportunities
    let total_gaps: usize = coin_data.iter().map(|d| identify_friday_monday_gaps(d).len()).sum();
    eprintln!("\nTotal gap opportunities across all coins: {}", total_gaps);

    let grid = build_grid();
    eprintln!("Grid: {} configs × {} coins (gap fill only)", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.gap_threshold == 0.0 };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        for d in &coin_data {
            let (pnl, wins, losses, flats) = simulate_gap_fill(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
        }
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.gap_threshold == 0.0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        if d % 20 == 0 || d == total_cfgs {
            eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        }
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN58 CME Gap Fill Results ===");
    println!("Baseline (no gap fill): PnL={:+.2}  WR=N/A  Trades=0", baseline.total_pnl);
    println!("\n{:>3}  {:<30} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(65));
    for (i,r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<30} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl && best.total_trades > 0;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN58 CME gap fill. {} configs. Best: {} (PnL={:.2}, WR={:.1}%, trades={})",
        results.len(), best.label, best.total_pnl, best.portfolio_wr, best.total_trades);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run58_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run58_1_results.json");
}
