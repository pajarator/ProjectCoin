/// RUN96 — Z-Confluence Exit: Exit When Multiple Coins' Z-Scores Simultaneously Converge
///
/// Grid: Z_BAND [0.3, 0.5, 0.7] × MIN_COINS [4, 5, 6, 8] × MODE ["soft", "hard"]
/// 24 configs × portfolio-level simulation = 24 simulations
///
/// Run: cargo run --release --features run96 -- --run96

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

const Z_BREADTH_LONG: usize = 20;
const Z_BREADTH_SHORT: usize = 50;

#[derive(Clone, Copy, PartialEq, Debug)]
struct ConfluenceCfg {
    z_band: f64,
    min_coins: usize,
    is_hard: bool,
    is_baseline: bool,
}

impl ConfluenceCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let mode = if self.is_hard { "H" } else { "S" };
            format!("B{:.1}_C{}_{}", self.z_band, self.min_coins, mode)
        }
    }
}

fn build_grid() -> Vec<ConfluenceCfg> {
    let mut grid = vec![ConfluenceCfg { z_band: 999.0, min_coins: 0, is_hard: false, is_baseline: true }];
    let bands = [0.3, 0.5, 0.7];
    let coins = [4usize, 5, 6, 8];
    for &band in &bands {
        for &mc in &coins {
            grid.push(ConfluenceCfg { z_band: band, min_coins: mc, is_hard: false, is_baseline: false });
        }
    }
    for &band in &bands {
        for &mc in &coins {
            grid.push(ConfluenceCfg { z_band: band, min_coins: mc, is_hard: true, is_baseline: false });
        }
    }
    grid
}

struct CoinData {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    wins: usize,
    losses: usize,
    pf: f64,
    max_dd: f64,
    confluence_exits: usize,
    is_baseline: bool,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
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
    Some(CoinData { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Count how many coins have z-scores within |z| < band
fn count_converged(all_z: &[&[f64]], idx: usize, band: f64) -> usize {
    let mut count = 0usize;
    for z_arr in all_z {
        if idx < z_arr.len() {
            let z = z_arr[idx];
            if !z.is_nan() && z.abs() < band {
                count += 1;
            }
        }
    }
    count
}

// Compute market mode per bar
fn compute_modes(all_z: &[&[f64]], n: usize) -> Vec<i8> {
    let mut modes = vec![0i8; n];
    for i in 20..n {
        let mut extreme = 0usize;
        for z_arr in all_z {
            if i < z_arr.len() && !z_arr[i].is_nan() && z_arr[i].abs() > 2.0 {
                extreme += 1;
            }
        }
        let breadth = extreme * 100 / N_COINS;
        if breadth <= Z_BREADTH_LONG { modes[i] = 1; }
        else if breadth >= Z_BREADTH_SHORT { modes[i] = -1; }
        else { modes[i] = 0; }
    }
    modes
}

// Portfolio-level simulation
fn simulate_portfolio(data: &[&CoinData], cfg: &ConfluenceCfg) -> ConfigResult {
    let n = data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64, usize, f64)>> = vec![None; N_COINS]; // dir, entry, entry_bar, z_at_entry
    let mut cooldowns = vec![0usize; N_COINS];
    let mut original_z: Vec<Option<f64>> = vec![None; N_COINS];

    // All z-score references for confluence calc
    let all_z: Vec<&[f64]> = data.iter().map(|d| d.zscore.as_slice()).collect();
    let modes = compute_modes(&all_z, n);

    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;
    let mut confluence_exits = 0usize;
    let mut peak = INITIAL_BAL * N_COINS as f64;
    let mut max_dd = 0.0f64;

    for i in 20..n {
        let converged = if cfg.is_baseline { 0 } else { count_converged(&all_z, i, cfg.z_band) };
        let mode = modes[i];

        for c in 0..N_COINS {
            let d = data[c];

            if let Some((dir, entry, entry_bar, z_entry)) = positions[c] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
                if !closed && i >= entry_bar + MIN_HOLD_BARS {
                    // Z0 exit
                    if (dir == 1 && d.zscore[i] >= 0.0) || (dir == -1 && d.zscore[i] <= 0.0) {
                        // Soft mode: tag with confluence if threshold met
                        if !cfg.is_baseline && !cfg.is_hard && converged >= cfg.min_coins {
                            confluence_exits += 1;
                        }
                        exit_pct = pct; closed = true;
                    }
                }
                // Hard confluence mode: force close all positions when threshold met
                if !closed && !cfg.is_baseline && cfg.is_hard && converged >= cfg.min_coins {
                    exit_pct = pct; closed = true; confluence_exits += 1;
                }

                if closed {
                    let net = balances[c] * POSITION_SIZE * LEVERAGE * exit_pct;
                    balances[c] += net;
                    if net > 1e-10 { total_wins += 1; }
                    else if net < -1e-10 { total_losses += 1; }
                    else { total_flats += 1; }
                    original_z[c] = Some(z_entry);
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    let mode_ok = match mode {
                        1 => dir == 1,    // LONG mode: only longs
                        -1 => dir == -1,  // SHORT mode: only shorts
                        _ => true,        // IsoShort: both ok
                    };
                    if mode_ok && i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            positions[c] = Some((dir, entry_price, i, d.zscore[i]));
                            original_z[c] = None;
                        }
                    }
                }
            }
        }

        // Track portfolio equity for max DD
        let portfolio_val: f64 = balances.iter().sum::<f64>() + positions.iter()
            .filter_map(|p| p.map(|(dir, entry, _, _)| {
                let c = data.iter().position(|d| d.closes.len() > i).unwrap_or(0);
                let price = data[c].closes[i];
                let pct = if dir == 1 { (price - entry) / entry } else { (entry - price) / entry };
                INITIAL_BAL * POSITION_SIZE * LEVERAGE * pct
            })).sum::<f64>();
        if portfolio_val > peak { peak = portfolio_val; }
        let dd = (peak - portfolio_val) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    let total_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    let total_trades = total_wins + total_losses + total_flats;
    let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = POSITION_SIZE * LEVERAGE * SL_PCT * total_wins as f64;
    let avg_loss = POSITION_SIZE * LEVERAGE * SL_PCT * total_losses as f64;
    let pf = if total_losses > 0 { avg_win / avg_loss } else { 0.0 };

    ConfigResult {
        label: cfg.label(),
        total_pnl,
        portfolio_wr,
        total_trades,
        wins: total_wins,
        losses: total_losses,
        pf,
        max_dd,
        confluence_exits,
        is_baseline: cfg.is_baseline,
    }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN96 — Z-Confluence Exit Grid Search\n");
    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let coin_refs: Vec<&CoinData> = data.iter().collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} portfolio-level configs", grid.len());

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, max_dd: 0.0, confluence_exits: 0, is_baseline: cfg.is_baseline };
        }
        let r = simulate_portfolio(&coin_refs, cfg);
        eprintln!("  {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  cf_exits={}",
            cfg.label(), r.total_pnl, r.portfolio_wr, r.total_trades, r.confluence_exits);
        r
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN96 Z-Confluence Exit Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  DD={:.1}%", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades, baseline.max_dd);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>6} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "DD%", "CfExits");
    println!("{}", "-".repeat(75));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>5.1}%  {:>8}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.max_dd, r.confluence_exits);
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN96 z-confluence exit. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run96_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run96_1_results.json");
}
