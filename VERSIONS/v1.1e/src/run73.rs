/// RUN73 — Dynamic Max Hold Based on Entry Z-Score
///
/// Hold extreme-deviation trades longer: max_hold = BASE + (|z_at_entry| - Z_BASE) * FACTOR
/// Capped at MAX_HOLD_CAP.
///
/// Grid: BASE [150, 200, 250] × Z_BASE [1.5, 2.0] × FACTOR [30, 40, 50] × CAP [280, 320, 360]
/// Total: 3 × 2 × 3 × 3 = 54 + baseline = 55 configs
///
/// Run: cargo run --release --features run73 -- --run73

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;
const BASELINE_MAX_HOLD: usize = 240;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct DynHoldCfg {
    base: usize,
    z_base: f64,
    factor: usize,
    cap: usize,
    is_baseline: bool,
}

impl DynHoldCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("B{}_ZB{}_F{}_C{}", self.base, (self.z_base * 10.0) as usize, self.factor, self.cap) }
    }
}

fn build_grid() -> Vec<DynHoldCfg> {
    let mut grid = vec![DynHoldCfg { base: 0, z_base: 0.0, factor: 0, cap: 0, is_baseline: true }];
    let bases = [150, 200, 250];
    let z_bases = [1.5, 2.0];
    let factors = [30, 40, 50];
    let caps = [280, 320, 360];
    for &base in &bases {
        for &z_base in &z_bases {
            for &factor in &factors {
                for &cap in &caps {
                    grid.push(DynHoldCfg { base, z_base, factor, cap, is_baseline: false });
                }
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
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, avg_held: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, avg_held_bars: f64,
    coins: Vec<CoinResult>,
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

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn effective_max_hold(z_entry: f64, cfg: &DynHoldCfg) -> usize {
    if cfg.is_baseline { return BASELINE_MAX_HOLD; }
    let z_abs = z_entry.abs();
    let extra = ((z_abs - cfg.z_base) * cfg.factor as f64).max(0.0);
    (cfg.base as f64 + extra).min(cfg.cap as f64) as usize
}

fn simulate(d: &CoinData15m, cfg: &DynHoldCfg) -> (f64, usize, usize, usize, f64, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    // (dir, entry_price, z_at_entry, entry_bar)
    let mut pos: Option<(i8, f64, f64, usize)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut total_held: usize = 0;

    for i in 1..n {
        if let Some((dir, entry, z_entry, entry_bar)) = pos {
            let held = i - entry_bar;
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL exit
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

            // Z-score crossback exit (after MIN_HOLD)
            if !closed && held >= MIN_HOLD_BARS {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }

            // Dynamic MAX_HOLD safety exit
            if !closed {
                let max_hold = effective_max_hold(z_entry, cfg);
                if held >= max_hold { exit_pct = pct; closed = true; }
            }

            if closed {
                total_held += held;
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 { cooldown -= 1; }
        else {
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    let z_entry = d.zscore[i];
                    if entry_price > 0.0 && !z_entry.is_nan() {
                        pos = Some((dir, entry_price, z_entry, i));
                    }
                }
            }
        }
    }
    let trades = wins + losses + flats;
    let avg_held = if trades > 0 { total_held as f64 / trades as f64 } else { 0.0 };
    (bal - INITIAL_BAL, wins, losses, flats, avg_held, total_held)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN73 — Dynamic Max Hold Based on Entry Z-Score\n");
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
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, avg_held_bars: 0.0, coins: vec![],
            };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, avg_held, _total_held) = simulate(d, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, avg_held }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_held_sum: f64 = coin_results.iter().map(|c| c.avg_held * c.trades as f64).sum();
        let avg_held_bars = if total_trades > 0 { avg_held_sum / total_trades as f64 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  avg_held={:.1}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades, avg_held_bars);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, avg_held_bars,
            coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN73 Dynamic Max Hold Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  avg_held={:.1}",
        baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades, baseline.avg_held_bars);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>9} {:>9}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "AvgHeld", "PF");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.1} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.avg_held_bars, r.pf);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN73 dynamic max hold. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run73_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run73_1_results.json");
}