/// RUN65 — BB Squeeze Duration Filter
///
/// Grid: MIN_SQUEEZE [4, 6, 8] × EXIT_MODE [1=strict, 2=relaxed]
/// Total: 6 configs + baseline = 7
///
/// Run: cargo run --release --features run65 -- --run65

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
struct SqueezeCfg {
    min_squeeze: usize, // 0 = disabled (baseline)
    exit_mode: u8, // 1=strict, 2=relaxed
}

impl SqueezeCfg {
    fn label(&self) -> String {
        if self.min_squeeze == 0 { "DISABLED".to_string() }
        else { format!("MS{}_EM{}", self.min_squeeze, self.exit_mode) }
    }
}

fn build_grid() -> Vec<SqueezeCfg> {
    let mut grid = vec![SqueezeCfg { min_squeeze: 0, exit_mode: 0 }]; // baseline
    let squeezes = [4, 6, 8];
    let modes = [1, 2];
    for &m in &squeezes {
        for &e in &modes {
            grid.push(SqueezeCfg { min_squeeze: m, exit_mode: e });
        }
    }
    grid
}

struct CoinData15m { name: String, closes: Vec<f64>, opens: Vec<f64>, highs: Vec<f64>, lows: Vec<f64>, zscore: Vec<f64>, bb_upper: Vec<f64>, bb_lower: Vec<f64>, bb_width: Vec<f64>, bb_width_avg: Vec<f64> }
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

    // Bollinger Bands (20-period, 2 std)
    let bb_period = 20usize;
    let bb_std_mult = 2.0;
    let mut bb_sma = vec![f64::NAN; n];
    let mut bb_upper = vec![f64::NAN; n];
    let mut bb_lower = vec![f64::NAN; n];
    let mut bb_width = vec![f64::NAN; n];

    for i in bb_period..n {
        let window = &closes[i+1-bb_period..=i];
        let mean = window.iter().sum::<f64>()/bb_period as f64;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/bb_period as f64).sqrt();
        bb_sma[i] = mean;
        bb_upper[i] = mean + bb_std_mult * std;
        bb_lower[i] = mean - bb_std_mult * std;
        bb_width[i] = bb_upper[i] - bb_lower[i];
    }

    // BB width average (20-bar)
    let mut bb_width_avg = vec![f64::NAN; n];
    for i in bb_period * 2..n {
        bb_width_avg[i] = bb_width[i+1-bb_period*2..=i].iter().filter(|&&x| !x.is_nan()).sum::<f64>() / bb_period as f64;
        if bb_width_avg[i].is_nan() { bb_width_avg[i] = bb_width[i]; }
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, bb_upper, bb_lower, bb_width, bb_width_avg })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn passes_squeeze_filter(d: &CoinData15m, i: usize, cfg: SqueezeCfg) -> bool {
    if cfg.min_squeeze == 0 { return true; }
    // Count squeeze bars ending at current bar
    let mut squeeze_count = 0usize;
    for j in (0..i).rev() {
        let bw = d.bb_width[j];
        let bw_avg = d.bb_width_avg[j];
        if !bw.is_nan() && !bw_avg.is_nan() && bw > 0.0 && bw_avg > 0.0 && bw < bw_avg * 0.6 {
            squeeze_count += 1;
        } else {
            break;
        }
    }
    squeeze_count >= cfg.min_squeeze
}

fn simulate(d: &CoinData15m, cfg: SqueezeCfg) -> (f64, usize, usize, usize) {
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
                // Squeeze duration filter
                if !passes_squeeze_filter(d, i, cfg) { continue; }
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
    eprintln!("RUN65 — BB Squeeze Duration Filter\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.min_squeeze == 0 };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        for d in &coin_data {
            let (pnl, wins, losses, flats) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
        }
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.min_squeeze == 0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN65 BB Squeeze Duration Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<12} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(52));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<12} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(52));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN65 BB squeeze duration filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run65_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run65_1_results.json");
}
