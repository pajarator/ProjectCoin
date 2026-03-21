/// RUN79 — Breadth-Adaptive Position Sizing
///
/// Scale regime RISK based on market breadth:
/// - Low breadth (LONG zone): boost LONG risk × BREADTH_LONG_BOOST
/// - High breadth (ISO_SHORT zone): boost ISO_SHORT risk × BREADTH_ISO_BOOST, reduce LONG risk
///
/// Grid: LONG_BOOST [1.1, 1.3, 1.5] × LONG_REDUCE [0.7, 0.8, 0.9] × ISO_BOOST [1.1, 1.3, 1.5]
/// Total: 3 × 3 × 3 = 27 + baseline = 28 configs
///
/// Run: cargo run --release --features run79 -- --run79

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.10; // 10% base risk
const LEVERAGE: f64 = 5.0;
const RISK_MIN: f64 = 0.05;
const RISK_MAX: f64 = 0.15;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct BreadthCfg {
    long_boost: f64,
    long_reduce: f64,
    iso_boost: f64,
    is_baseline: bool,
}

impl BreadthCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("LB{:.1}_LR{:.1}_IB{:.1}", self.long_boost, self.long_reduce, self.iso_boost) }
    }
}

fn build_grid() -> Vec<BreadthCfg> {
    let mut grid = vec![BreadthCfg { long_boost: 1.0, long_reduce: 1.0, iso_boost: 1.0, is_baseline: true }];
    let boosts = [1.1, 1.3, 1.5];
    let reduces = [0.7, 0.8, 0.9];
    let iso_boosts = [1.1, 1.3, 1.5];
    for &lb in &boosts {
        for &lr in &reduces {
            for &ib in &iso_boosts {
                grid.push(BreadthCfg { long_boost: lb, long_reduce: lr, iso_boost: ib, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

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

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Compute breadth (% of coins with z < -1.5) at each bar
fn compute_breadth(data: &[CoinData15m]) -> Vec<f64> {
    let n = data[0].closes.len();
    let mut breadth = vec![0.0; n];
    for i in 20..n {
        let mut count = 0usize;
        let mut valid = 0usize;
        for d in data {
            if !d.zscore[i].is_nan() {
                valid += 1;
                if d.zscore[i] < -1.5 { count += 1; }
            }
        }
        breadth[i] = if valid > 0 { count as f64 / valid as f64 } else { 0.0 };
    }
    breadth
}

fn effective_risk(cfg: &BreadthCfg, breadth: f64, dir: i8) -> f64 {
    if cfg.is_baseline { return BASE_POSITION_SIZE; }
    let base = BASE_POSITION_SIZE;
    if dir == 1 {
        // LONG: boost when breadth < 15%, reduce when > 35%
        if breadth < 0.15 {
            (base * cfg.long_boost).min(RISK_MAX)
        } else if breadth > 0.35 {
            (base * cfg.long_reduce).max(RISK_MIN)
        } else {
            base
        }
    } else {
        // ISO_SHORT: boost when breadth > 35%
        if breadth > 0.35 {
            (base * cfg.iso_boost).min(RISK_MAX)
        } else {
            base
        }
    }
}

fn simulate(d: &CoinData15m, breadth: &[f64], cfg: &BreadthCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, f64)> = None; // dir, entry_price, risk_at_entry
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for i in 1..n {
        let b = breadth[i];

        if let Some((dir, entry, risk)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * risk * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        let risk = effective_risk(cfg, b, dir);
                        pos = Some((dir, entry_price, risk));
                    }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN79 — Breadth-Adaptive Position Sizing\n");
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
    let breadth = compute_breadth(&coin_data);
    eprintln!("Breadth computed ({} bars), range: {:.1}% - {:.1}%",
        breadth.len(),
        breadth.iter().filter(|&&x| x > 0.0).fold(f64::INFINITY, |a, &x| a.min(x)) * 100.0,
        breadth.iter().fold(0.0f64, |a, &x| a.max(x)) * 100.0);

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![],
            };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats) = simulate(d, &breadth, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN79 Breadth-Adaptive Position Sizing Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(75));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN79 breadth-adaptive sizing. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run79_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run79_1_results.json");
}
