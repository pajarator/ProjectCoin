/// RUN45 — Complement-Scalp Mutual Exclusion + Exhaustion Timer
///
/// Hypothesis: After a complement long fires, suppress scalp entries for N bars
/// to avoid wasting scalp slots on redundant trades.
///
/// Grid:
///   COMPLEMENT_EXHAUSTION: [0, 2, 4, 8, 16] bars (0=disabled)
///
/// Run: cargo run --release --features run45 -- --run45

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

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

#[derive(Clone, Copy, PartialEq, Debug)]
struct CompScalpCfg { exhaustion_bars: usize }

impl CompScalpCfg {
    fn label(&self) -> String {
        if self.exhaustion_bars == 0 { "DISABLED".to_string() }
        else { format!("EX{}_{}", self.exhaustion_bars, (self.exhaustion_bars * 15)) }
    }
}

fn build_grid() -> Vec<CompScalpCfg> {
    (0..=4).map(|i| {
        let bars = match i {
            0 => 0, 1 => 2, 2 => 4, 3 => 8, 4 => 16,
            _ => 0,
        };
        CompScalpCfg { exhaustion_bars: bars }
    }).collect()
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    rsi: Vec<f64>,
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
    complement_trades: usize,
    scalp_trades: usize,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    total_trades: usize,
    portfolio_wr: f64,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len(); let mut out = vec![f64::NAN; n];
    if n < period+1 { return out; }
    let mut gains = vec![0.0; n]; let mut losses = vec![0.0; n];
    for i in 1..n { let d=c[i]-c[i-1]; if d>0.0{gains[i]=d;}else{losses[i]=-d;} }
    let mut sum_g = 0.0; let mut sum_l = 0.0;
    for i in 1..=period { sum_g += gains[i]; sum_l += losses[i]; }
    for i in period..n {
        if i > period { sum_g = sum_g - gains[i-period] + gains[i]; sum_l = sum_l - losses[i-period] + losses[i]; }
        let rs = if sum_l == 0.0 { 100.0 } else { sum_g/sum_l };
        out[i] = 100.0 - 100.0/(1.0 + rs);
    }
    out
}

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
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
    let rsi = rsi_calc(&closes, 14);
    let n = closes.len();
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, rsi, zscore })
}

// Regime signal: z-score based
fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Complement signal: RSI-based (simplified Laguerre RSI proxy: RSI crossing up from <20 or down from >80)
fn complement_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 2 { return None; }
    let rsi = d.rsi[i];
    let rsi_prev = d.rsi[i-1];
    if rsi.is_nan() || rsi_prev.is_nan() { return None; }
    // Crossing up from oversold
    if rsi_prev < 20.0 && rsi >= 20.0 { return Some(1); }
    // Crossing down from overbought
    if rsi_prev > 80.0 && rsi <= 80.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: CompScalpCfg) -> (f64, usize, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut comp_exhaustion = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut complement_trades = 0usize; let mut scalp_trades = 0usize;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false; let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed && i > 0 {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= 20 { exit_pct = pct; closed = true; }
            if closed {
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Try complement first (long only)
            let comp_dir = complement_signal(d, i);
            if comp_dir == Some(1) && i+1 < n {
                let entry_price = d.opens[i+1];
                if entry_price > 0.0 {
                    pos = Some((1, entry_price));
                    complement_trades += 1;
                    if cfg.exhaustion_bars > 0 { comp_exhaustion = cfg.exhaustion_bars; }
                    continue;
                }
            }
            // Try scalp (if not exhausted)
            if comp_exhaustion == 0 {
                let rsi = d.rsi[i];
                if !rsi.is_nan() {
                    // Scalp: RSI oversold/overbought
                    if rsi < 20.0 || rsi > 80.0 {
                        let dir = if rsi < 20.0 { 1 } else { -1 };
                        if i+1 < n {
                            let entry_price = d.opens[i+1];
                            if entry_price > 0.0 {
                                pos = Some((dir, entry_price));
                                scalp_trades += 1;
                            }
                        }
                    }
                }
            }
        }
        if comp_exhaustion > 0 { comp_exhaustion -= 1; }
    }
    let trades = wins + losses + flats;
    (bal - INITIAL_BAL, wins, losses, flats, complement_trades, scalp_trades)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN45 — Complement-Scalp Mutual Exclusion + Exhaustion Timer\n");

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
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, total_trades: 0, portfolio_wr: 0.0, pf: 0.0, is_baseline: cfg.exhaustion_bars == 0, coins: vec![] };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, comp, scalp) = simulate(d, *cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, complement_trades: comp, scalp_trades: scalp }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let wins_t: usize = coin_results.iter().map(|c| c.wins).sum();
        let losses_t: usize = coin_results.iter().map(|c| c.losses).sum();
        let avg_win = if wins_t > 0 { coin_results.iter().filter(|c|c.wins>0).map(|c|c.pnl*c.wins as f64/wins_t as f64).sum::<f64>()/wins_t as f64 } else { 0.0 };
        let avg_loss = if losses_t > 0 { coin_results.iter().filter(|c|c.losses>0).map(|c|c.pnl*c.losses as f64/losses_t as f64).sum::<f64>()/losses_t as f64 } else { 0.0 };
        let pf = if avg_loss.abs() > 1e-8 { avg_win / avg_loss.abs() } else { 0.0 };
        let is_baseline = cfg.exhaustion_bars == 0;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}  comp={}  scalp={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades,
            coin_results.iter().map(|c|c.complement_trades).sum::<usize>(),
            coin_results.iter().map(|c|c.scalp_trades).sum::<usize>());

        ConfigResult { label: cfg.label(), total_pnl, total_trades, portfolio_wr, pf, is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN45 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run45_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN45 Complement-Scalp Exhaustion Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<15} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(60));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<15} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN45 complement-scalp exhaustion. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run45_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run45_1_results.json");
}
