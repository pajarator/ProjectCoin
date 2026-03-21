/// RUN54 — Volatility Regime Entry Filter: Trade Only When Volatility Is Below Median
///
/// Grid: VOL_WINDOW [15, 20, 30] × VOL_THRESHOLD [0.50, 0.60, 0.70, 0.80] × MODE [1=strict, 2=relaxed]
/// Total: 24 configs + baseline = 25
///
/// Run: cargo run --release --features run54 -- --run54

use rayon::prelude::*;
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
struct VolCfg {
    vol_window: usize,
    vol_threshold: f64,
    mode: u8, // 1=strict (<threshold), 2=relaxed (<0.75)
}

impl VolCfg {
    fn label(&self) -> String {
        if self.vol_window == 0 { "DISABLED".to_string() }
        else { format!("VW{}_VT{:.2}_M{}", self.vol_window, self.vol_threshold, self.mode) }
    }
}

fn build_grid() -> Vec<VolCfg> {
    let mut grid = vec![VolCfg { vol_window: 0, vol_threshold: 0.0, mode: 0 }]; // baseline
    let windows = [15, 20, 30];
    let thresholds = [0.50, 0.60, 0.70, 0.80];
    let modes = [1, 2];
    for &w in &windows {
        for &t in &thresholds {
            for &m in &modes {
                grid.push(VolCfg { vol_window: w, vol_threshold: t, mode: m });
            }
        }
    }
    grid
}

struct CoinData15m { name: String, closes: Vec<f64>, opens: Vec<f64>, zscore: Vec<f64>, atr_rank: Vec<f64> }
#[derive(Serialize)] struct ConfigResult { label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize, pf: f64, is_baseline: bool }
#[derive(Serialize)] struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // ATR
    let mut trs = vec![0.0; n];
    for i in 1..n {
        trs[i] = (highs[i] - lows[i]).max((highs[i] - closes[i-1]).abs()).max((lows[i] - closes[i-1]).abs());
    }
    let mut atr = vec![f64::NAN; n];
    let atr_window = 14;
    for i in atr_window..n {
        let sum: f64 = trs[i+1-atr_window..=i].iter().sum();
        atr[i] = sum / atr_window as f64;
    }

    // ATR rank: percentile of current ATR within rolling window
    let vol_window = 20;
    let mut atr_rank = vec![f64::NAN; n];
    for i in vol_window..n {
        let window_atrs = &atr[i+1-vol_window..=i];
        let current = atr[i];
        if current.is_nan() { continue; }
        let count_below = window_atrs.iter().filter(|&&x| x <= current && !x.is_nan()).count() as f64;
        let valid_count = window_atrs.iter().filter(|&&x| !x.is_nan()).count() as f64;
        atr_rank[i] = if valid_count > 0.0 { count_below / valid_count } else { 0.5 };
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, atr_rank })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn passes_vol_filter(d: &CoinData15m, i: usize, cfg: VolCfg) -> bool {
    if cfg.vol_window == 0 { return true; } // disabled
    let rank = d.atr_rank[i];
    if rank.is_nan() { return true; }
    match cfg.mode {
        1 => rank < cfg.vol_threshold,       // strict: below threshold
        2 => rank < 0.75,                   // relaxed: below 75th percentile
        _ => true,
    }
}

fn simulate(d: &CoinData15m, cfg: VolCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

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
                let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 { cooldown -= 1; }
        else {
            if let Some(dir) = regime_signal(d, i) {
                // Volatility filter check
                if !passes_vol_filter(d, i, cfg) { continue; }
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN54 — Volatility Regime Entry Filter\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.vol_window == 0 };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        for d in &coin_data {
            let (pnl, wins, losses, flats) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
        }
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.vol_window == 0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN54 Volatility Regime Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<24} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(62));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<24} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(62));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN54 volatility regime filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run54_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run54_1_results.json");
}
