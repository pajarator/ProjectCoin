/// RUN63 — BTC Trend Confirmation for Regime Entries
///
/// Grid: RET_THRESHOLD [0.005, 0.010, 0.015] × SMA_CONFIRM [true, false]
/// Total: 6 configs + baseline = 7
///
/// Run: cargo run --release --features run63 -- --run63

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
struct BtcTrendCfg {
    ret_threshold: f64,
    sma_confirm: bool,
}

impl BtcTrendCfg {
    fn label(&self) -> String {
        if !self.sma_confirm && self.ret_threshold == 0.0 { "DISABLED".to_string() }
        else { format!("RET{:.3}_SMA{}", self.ret_threshold, if self.sma_confirm { "T" } else { "F" }) }
    }
}

fn build_grid() -> Vec<BtcTrendCfg> {
    let mut grid = vec![BtcTrendCfg { ret_threshold: 0.0, sma_confirm: false }]; // baseline
    let rets = [0.005, 0.010, 0.015];
    let sma_confirms = [false, true];
    for &r in &rets {
        for &s in &sma_confirms {
            grid.push(BtcTrendCfg { ret_threshold: r, sma_confirm: s });
        }
    }
    grid
}

struct CoinData15m { name: String, closes: Vec<f64>, opens: Vec<f64>, zscore: Vec<f64> }
#[derive(Serialize)] struct ConfigResult { label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize, pf: f64, is_baseline: bool }
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

fn simulate_with_btc(d: &CoinData15m, btc_d: &CoinData15m, cfg: BtcTrendCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

    for i in 1..n {
        // Compute BTC SMA9, SMA20, and 16-bar return
        let btc_sma9 = if i >= 9 {
            btc_d.closes[i+1-9..=i].iter().sum::<f64>() / 9.0
        } else { 0.0 };
        let btc_sma20 = if i >= 20 {
            btc_d.closes[i+1-20..=i].iter().sum::<f64>() / 20.0
        } else { 0.0 };
        let btc_ret16 = if i >= 16 {
            (btc_d.closes[i] - btc_d.closes[i-16]) / btc_d.closes[i-16]
        } else { 0.0 };

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
                // BTC trend filter
                if cfg.ret_threshold > 0.0 || cfg.sma_confirm {
                    let btc_sma9_valid = btc_sma9 > 0.0 && btc_sma20 > 0.0;
                    let btc_ret_valid = btc_ret16.abs() > 1e-10;

                    if btc_sma9_valid && btc_ret_valid {
                        match dir {
                            1 => {
                                // LONG: block if BTC in downtrend
                                let btc_down = if cfg.sma_confirm {
                                    btc_sma9 < btc_sma20 && btc_ret16 < -cfg.ret_threshold
                                } else {
                                    btc_ret16 < -cfg.ret_threshold
                                };
                                if btc_down { continue; }
                            }
                            -1 => {
                                // SHORT: block if BTC in uptrend
                                let btc_up = if cfg.sma_confirm {
                                    btc_sma9 > btc_sma20 && btc_ret16 > cfg.ret_threshold
                                } else {
                                    btc_ret16 > cfg.ret_threshold
                                };
                                if btc_up { continue; }
                            }
                            _ => {}
                        }
                    }
                }
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
    eprintln!("RUN63 — BTC Trend Confirmation for Regime Entries\n");
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

    // BTC data is the last coin
    let btc_idx = COIN_NAMES.iter().position(|&n| n == "BTC").unwrap();
    let btc_data = &coin_data[btc_idx];

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.ret_threshold == 0.0 && !cfg.sma_confirm };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        for ci in 0..N_COINS {
            if ci == btc_idx { continue; } // skip BTC itself
            let (pnl, wins, losses, flats) = simulate_with_btc(&coin_data[ci], btc_data, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
        }
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.ret_threshold == 0.0 && !cfg.sma_confirm;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN63 BTC Trend Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<16} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(58));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<16} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(58));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN63 BTC trend filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run63_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run63_1_results.json");
}
