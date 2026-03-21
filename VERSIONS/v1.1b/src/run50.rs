/// RUN50 — Candle Composition Filter: Volume Profile Imbalance as Entry Quality Gate
///
/// Grid: MIN_BODY_RATIO [0.3, 0.4, 0.5]
///     × MIN_SHADOW_RATIO [0.10, 0.15, 0.20]
///     × MIN_VOL_MULT [1.0, 1.2, 1.5]
/// Total: 27 configs
///
/// Run: cargo run --release --features run50 -- --run50

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
struct CandleCfg {
    min_body_ratio: f64,
    min_shadow_ratio: f64,
    min_vol_mult: f64,
}

impl CandleCfg {
    fn label(&self) -> String {
        if self.min_body_ratio == 0.0 { "DISABLED".to_string() }
        else { format!("B{:.1}_S{:.2}_V{:.1}", self.min_body_ratio, self.min_shadow_ratio, self.min_vol_mult) }
    }
}

fn build_grid() -> Vec<CandleCfg> {
    let mut grid = vec![CandleCfg { min_body_ratio: 0.0, min_shadow_ratio: 0.0, min_vol_mult: 0.0 }]; // baseline
    let bwr_vals = [0.3, 0.4, 0.5];
    let shadow_vals = [0.10, 0.15, 0.20];
    let vol_vals = [1.0, 1.2, 1.5];
    for &b in &bwr_vals {
        for &s in &shadow_vals {
            for &v in &vol_vals {
                grid.push(CandleCfg { min_body_ratio: b, min_shadow_ratio: s, min_vol_mult: v });
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    volumes: Vec<f64>,
    zscore: Vec<f64>,
    vol_ma: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, flats: usize, wr: f64 }

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    let mut volumes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc); volumes.push(vv);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / 20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // Volume MA
    let mut vol_ma = vec![f64::NAN; n];
    let mut vol_sum = 0.0;
    for i in 0..n {
        vol_sum += volumes[i];
        if i >= 20 { vol_sum -= volumes[i-20]; }
        if i + 1 >= 20 { vol_ma[i] = vol_sum / 20.0; }
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, volumes, zscore, vol_ma })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn candle_passes(d: &CoinData15m, i: usize, cfg: CandleCfg) -> bool {
    let open = d.opens[i];
    let high = d.highs[i];
    let low = d.lows[i];
    let close = d.closes[i];
    let vol = d.volumes[i];
    let vol_ma = d.vol_ma[i];

    let range = high - low;
    if range < 1e-10 { return true; } // skip degenerate candles

    let body = (close - open).abs();
    let body_ratio = body / range;

    let upper_shadow = high - open.max(close);
    let lower_shadow = open.min(close) - low;
    let upper_ratio = upper_shadow / range;
    let lower_ratio = lower_shadow / range;

    // Body ratio check
    if body_ratio < cfg.min_body_ratio { return false; }

    // Volume check
    if !vol_ma.is_nan() && vol < vol_ma * cfg.min_vol_mult { return false; }

    true
}

fn simulate(d: &CoinData15m, cfg: CandleCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= 2 { exit_pct = pct; closed = true; }
            if closed {
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d, i) {
                // Apply candle composition filter
                let passes = candle_passes(d, i, cfg);
                if passes && i + 1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN50 — Candle Composition Filter\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.min_body_ratio == 0.0, coins: vec![] };
        }
        let mut total_pnl = 0.0;
        let mut wins_sum = 0usize;
        let mut losses_sum = 0usize;
        let mut flats_sum = 0usize;
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, flats, wr }
        }).collect();
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.min_body_ratio == 0.0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN50 Candle Composition Filter Results ===");
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

    let notes = format!("RUN50 candle composition filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run50_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run50_1_results.json");
}
