/// RUN53 — Partial Exit / Scale-Out: Progressive Profit-Taking
///
/// Grid: TIER1_PCT [0.3, 0.4, 0.5, 0.6] × TIER2_PCT [0.6, 0.8, 1.0, 1.2] × EXIT_FRAC [0.33, 0.50]
/// Total: 32 configs + baseline = 33
///
/// Run: cargo run --release --features run53 -- --run53

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
struct ScaleCfg {
    tier1_pct: f64,
    tier2_pct: f64,
    exit_frac: f64,
}

impl ScaleCfg {
    fn label(&self) -> String {
        if self.tier1_pct == 0.0 { "DISABLED".to_string() }
        else { format!("T1P{:+.2}_T2P{:+.2}_F{:.2}", self.tier1_pct, self.tier2_pct, self.exit_frac) }
    }
}

fn build_grid() -> Vec<ScaleCfg> {
    let mut grid = vec![ScaleCfg { tier1_pct: 0.0, tier2_pct: 0.0, exit_frac: 0.0 }];
    let t1_vals = [0.003, 0.004, 0.005, 0.006];
    let t2_vals = [0.006, 0.008, 0.010, 0.012];
    let frac_vals = [0.33, 0.50];
    for &t1 in &t1_vals {
        for &t2 in &t2_vals {
            for &f in &frac_vals {
                grid.push(ScaleCfg { tier1_pct: t1, tier2_pct: t2, exit_frac: f });
            }
        }
    }
    grid
}

struct CoinData15m { name: String, closes: Vec<f64>, opens: Vec<f64>, zscore: Vec<f64> }
#[derive(Serialize)] struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, flats: usize, wr: f64 }
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

fn simulate(d: &CoinData15m, cfg: ScaleCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let disabled = cfg.tier1_pct == 0.0;

    // Track remaining position fraction for partial exits
    let mut remaining_frac = 1.0; // fraction of original position still held

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // Tier 1 partial exit
            if !disabled && remaining_frac > 0.01 && pct >= cfg.tier1_pct {
                let frac_to_exit = BASE_POSITION_SIZE * cfg.exit_frac * remaining_frac;
                let net = (bal * frac_to_exit * LEVERAGE) * pct;
                bal += net;
                remaining_frac -= cfg.exit_frac;
                if remaining_frac < 0.01 { remaining_frac = 0.0; }
                if remaining_frac < 0.01 { exit_pct = pct; closed = true; }
            }

            // Tier 2 partial exit
            if !closed && !disabled && remaining_frac > 0.01 && pct >= cfg.tier2_pct {
                let frac_to_exit = BASE_POSITION_SIZE * cfg.exit_frac * remaining_frac;
                let net = (bal * frac_to_exit * LEVERAGE) * pct;
                bal += net;
                remaining_frac -= cfg.exit_frac;
                if remaining_frac < 0.01 { exit_pct = pct; closed = true; }
            }

            // Full exit conditions
            if !closed {
                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(d, i);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= 2 { exit_pct = pct; closed = true; }
            }

            if closed {
                // Exit remaining position at exit_pct
                if remaining_frac > 0.01 {
                    let frac_to_exit = BASE_POSITION_SIZE * remaining_frac;
                    let net = (bal * frac_to_exit * LEVERAGE) * exit_pct;
                    bal += net;
                }
                // Use entry price for win/loss determination
                let full_net = (entry * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                if full_net > 1e-10 { wins += 1; } else if full_net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
                remaining_frac = 1.0;
            }
        } else if cooldown > 0 { cooldown -= 1; }
        else {
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); remaining_frac = 1.0; }
                }
            }
        }
    }
    let pnl = bal - INITIAL_BAL;
    // Count wins/losses properly
    (pnl, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN53 — Partial Exit / Scale-Out\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.tier1_pct == 0.0 };
        }
        let mut total_pnl = 0.0; let mut wins_sum = 0usize; let mut losses_sum = 0usize; let mut flats_sum = 0usize;
        for d in &coin_data {
            let (pnl, wins, losses, flats) = simulate(d, *cfg);
            total_pnl += pnl;
            wins_sum += wins; losses_sum += losses; flats_sum += flats;
        }
        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.tier1_pct == 0.0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);
        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf: 0.0, is_baseline }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN53 Partial Scale-Out Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<28} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(65));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<28} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN53 partial scale-out. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run53_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run53_1_results.json");
}
