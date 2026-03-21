/// RUN48 — Z-Score Recovery Suppression: Anti-Chase Re-Entry Gate
///
/// After a regime trade exits, the coin's z-score has typically reverted close
/// to the mean. If another entry fires immediately, the trader "chases" the
/// same coin after mean-reversion has already completed.
///
/// Fix: track z_at_exit. For N bars (suppress window), block re-entries unless
/// z has drifted back below a threshold (fresh deviation).
///
/// Grid: SUPPRESS_BARS [0, 4, 6, 8, 12, 16] × ENTRY_THRESHOLD [-0.5, -0.75, -1.0, -1.25]
///       × MODE [1=after_win_only, 2=after_any_exit]
/// Total: 6 × 4 × 2 = 48 configs
///
/// Run: cargo run --release --features run48 -- --run48

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct ZRecCfg {
    suppress_bars: u32,
    entry_threshold: f64,
    mode: u8, // 0=disabled, 1=after_win_only, 2=after_any_exit
}

impl ZRecCfg {
    fn label(&self) -> String {
        if self.suppress_bars == 0 { "DISABLED".to_string() }
        else {
            let mode_s = match self.mode {
                1 => "W",
                2 => "A",
                _ => "X",
            };
            format!("SB{}_T{:+.2}_{}", self.suppress_bars, self.entry_threshold, mode_s)
        }
    }
}

fn build_grid() -> Vec<ZRecCfg> {
    let mut grid = Vec::new();
    // Baseline: disabled
    grid.push(ZRecCfg { suppress_bars: 0, entry_threshold: -1.0, mode: 0 });

    let bars_vals = [4u32, 6, 8, 12, 16];
    let thresh_vals = [-0.5f64, -0.75, -1.0, -1.25];
    let modes = [1u8, 2];

    for &sb in &bars_vals {
        for &t in &thresh_vals {
            for &m in &modes {
                grid.push(ZRecCfg { suppress_bars: sb, entry_threshold: t, mode: m });
            }
        }
    }
    grid
}

struct CoinData15m { name: String, closes: Vec<f64>, opens: Vec<f64>, zscore: Vec<f64> }
#[derive(Serialize)] struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, flats: usize, wr: f64 }
#[derive(Serialize)] struct ConfigResult { label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize, pf: f64, is_baseline: bool, coins: Vec<CoinResult> }
#[derive(Serialize)] struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
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

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: ZRecCfg) -> (f64, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

    // Z-recovery suppression state
    let mut suppress_bars_remaining: u32 = 0;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false; let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= 2 { exit_pct = pct; closed = true; }
            if closed {
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                let was_win = net > 1e-10;
                if was_win { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }

                // Set z-recovery suppression
                if cfg.mode > 0 {
                    if cfg.mode == 2 || (cfg.mode == 1 && was_win) {
                        suppress_bars_remaining = cfg.suppress_bars;
                    }
                }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 { cooldown -= 1; }
        else {
            // Z-recovery suppression gate
            if cfg.mode > 0 && suppress_bars_remaining > 0 {
                let z = d.zscore[i];
                if !z.is_nan() && z >= cfg.entry_threshold {
                    // Blocked: coin has recovered too much
                } else {
                    // Allow entry (z drifted back below threshold, or suppress window expired)
                    try_entry(d, i, &mut pos);
                }
            } else {
                try_entry(d, i, &mut pos);
            }
        }

        // Decrement suppress counter each bar (when no position)
        if pos.is_none() && suppress_bars_remaining > 0 {
            suppress_bars_remaining -= 1;
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats, wins + losses + flats)
}

fn try_entry(d: &CoinData15m, i: usize, pos: &mut Option<(i8, f64)>) {
    if let Some(dir) = regime_signal(d, i) {
        if i+1 < d.closes.len() {
            let entry_price = d.opens[i+1];
            if entry_price > 0.0 { *pos = Some((dir, entry_price)); }
        }
    }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN48 — Z-Score Recovery Suppression Grid Search\n");
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
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.suppress_bars == 0, coins: vec![] };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, _trades) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, flats, wr }
        }).collect();
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.suppress_bars == 0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN48 Z-Score Recovery Suppression Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(60));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN48 Z-recovery suppression. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run48_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run48_1_results.json");
}
