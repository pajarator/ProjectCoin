/// RUN108 — Momentum Hour Filter: 1m Momentum Filter on 15m Regime Entries
///
/// Grid: LOOKBACK [3, 6, 12] × LONG_MIN [-0.001, -0.002, -0.003] × SHORT_MAX [0.001, 0.002, 0.003]
/// 27 configs × 18 coins = 486 simulations (parallel per coin)
///
/// Run: cargo run --release --features run108 -- --run108

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

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct MomHourCfg {
    lookback: usize,
    long_min: f64,
    short_max: f64,
    is_baseline: bool,
}

impl MomHourCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("LB{}_LM{:.3}_SM{:.3}", self.lookback, self.long_min.abs(), self.short_max.abs()) }
    }
}

fn build_grid() -> Vec<MomHourCfg> {
    let mut grid = vec![MomHourCfg { lookback: 6, long_min: -999.0, short_max: 999.0, is_baseline: true }];
    for &lb in &[3usize, 6, 12] {
        for &lm in &[-0.001f64, -0.002, -0.003] {
            for &sm in &[0.001, 0.002, 0.003] {
                grid.push(MomHourCfg { lookback: lb, long_min: lm, short_max: sm, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData {
    name: String,
    // 15m data
    opens_15m: Vec<f64>,
    closes_15m: Vec<f64>,
    highs_15m: Vec<f64>,
    lows_15m: Vec<f64>,
    zscore: Vec<f64>,
    // 1m momentum at each 15m bar (computed from 1m closes)
    roc_1m: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pf: f64,
    entries_filtered: usize,
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
    is_baseline: bool,
    coins: Vec<CoinResult>,
    entries_filtered_total: usize,
    filter_rate: f64,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
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
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc);
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
    Some((opens, closes, highs, lows, zscore))
}

fn load_1m_closes(coin: &str) -> Option<Vec<f64>> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut closes_1m = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let _oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if cc.is_nan() { continue; }
        closes_1m.push(cc);
    }
    if closes_1m.len() < 100 { return None; }
    Some(closes_1m)
}

/// Precompute 1m ROC array at each 15m bar point.
/// roc_1m_at_15m[i] = ROC over lookback bars, sampled at the last 1m bar of 15m bar i.
fn compute_roc_1m_at_15m(closes_1m: &[f64], lookback: usize, n_15m: usize) -> Vec<f64> {
    let mut roc = vec![f64::NAN; n_15m];
    // Each 15m bar corresponds to 15 consecutive 1m bars
    // 15m bar i's last 1m bar is at index 15*(i+1) - 1 = 15*i + 14
    for i in 1..n_15m {
        let last_1m_idx = 15 * (i + 1) - 1; // last 1m bar of this 15m bar
        let past_1m_idx = last_1m_idx.saturating_sub(lookback);
        if past_1m_idx < last_1m_idx && past_1m_idx < closes_1m.len() && last_1m_idx < closes_1m.len() {
            let recent = closes_1m[last_1m_idx];
            let past = closes_1m[past_1m_idx];
            if past > 0.0 {
                roc[i] = (recent - past) / past;
            }
        }
    }
    roc
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &MomHourCfg) -> CoinResult {
    let n = d.closes_15m.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut entries_filtered = 0usize;

    for i in 20..n {
        cooldown = cooldown.saturating_sub(1);

        if let Some((dir, entry, entry_bar, _z_entry)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes_15m[i]-*entry)/ *entry } else { (*entry-d.closes_15m[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            // Regime signal exit
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            // End of data
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
            // Min hold + Z0 exit
            if !closed && i >= *entry_bar + MIN_HOLD_BARS {
                if (*dir == 1 && d.zscore[i] >= 0.0) || (*dir == -1 && d.zscore[i] <= 0.0) {
                    exit_pct = pct; closed = true;
                }
            }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown == 0 {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if cfg.is_baseline {
                    if i + 1 < n {
                        let entry_price = d.opens_15m[i + 1];
                        if entry_price > 0.0 {
                            pos = Some((dir, entry_price, i, d.zscore[i]));
                        }
                    }
                } else {
                    // 1m momentum filter check
                    let roc = d.roc_1m[i];
                    let passes_filter = if dir == 1 {
                        !roc.is_nan() && roc >= cfg.long_min
                    } else {
                        !roc.is_nan() && roc <= cfg.short_max
                    };
                    if passes_filter {
                        if i + 1 < n {
                            let entry_price = d.opens_15m[i + 1];
                            if entry_price > 0.0 {
                                pos = Some((dir, entry_price, i, d.zscore[i]));
                            }
                        }
                    } else {
                        entries_filtered += 1;
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = POSITION_SIZE * LEVERAGE * SL_PCT * wins as f64;
    let avg_loss = POSITION_SIZE * LEVERAGE * SL_PCT * losses as f64;
    let pf = if losses > 0 { avg_win / avg_loss } else { 0.0 };

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, entries_filtered }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN108 — Momentum Hour Filter Grid Search\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data_15m: Vec<Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.1.len()); }
        raw_data_15m.push(loaded);
    }
    if !raw_data_15m.iter().all(|r| r.is_some()) { eprintln!("Missing 15m data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    eprintln!("Loading 1m data for momentum computation...");
    let mut raw_data_1m: Vec<Option<Vec<f64>>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m_closes(name);
        if loaded.is_none() { eprintln!("Missing 1m data for {}!", name); }
        raw_data_1m.push(loaded);
    }
    if !raw_data_1m.iter().all(|r| r.is_some()) { eprintln!("Missing 1m data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    // Unwrap data
    let data_15m: Vec<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> =
        raw_data_15m.into_iter().map(|r| r.unwrap()).collect();
    let data_1m: Vec<Vec<f64>> = raw_data_1m.into_iter().map(|r| r.unwrap()).collect();

    // We need to pick one lookback for the main grid.
    // Precompute roc arrays for all 3 lookbacks per coin
    let grid_full = build_grid();
    let n_15m = data_15m[0].1.len();

    // Precompute all 3 lookback ROC arrays for all coins
    eprintln!("Computing 1m momentum for all lookbacks...");
    let lookbacks = [3usize, 6, 12];
    let mut roc_arrays: Vec<Vec<Vec<f64>>> = Vec::new(); // [lookback_idx][coin_idx][bar]
    for &lb in &lookbacks {
        let mut coin_rocs: Vec<Vec<f64>> = Vec::new();
        for ci in 0..N_COINS {
            let roc = compute_roc_1m_at_15m(&data_1m[ci], lb, n_15m);
            coin_rocs.push(roc);
        }
        roc_arrays.push(coin_rocs);
    }

    // Build data with default lookback=6 for initial build
    let build_data = |lb_idx: usize| -> Vec<CoinData> {
        data_15m.iter().enumerate().map(|(ci, (opens, closes, highs, lows, zscore))| {
            CoinData {
                name: COIN_NAMES[ci].to_string(),
                opens_15m: opens.clone(),
                closes_15m: closes.clone(),
                highs_15m: highs.clone(),
                lows_15m: lows.clone(),
                zscore: zscore.clone(),
                roc_1m: roc_arrays[lb_idx][ci].clone(),
            }
        }).collect()
    };

    // Simulate each config separately (need different lookback per config)
    let done = AtomicUsize::new(0);
    let total_sims = grid_full.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid_full.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], entries_filtered_total: 0, filter_rate: 0.0 };
        }
        // Select lookback index
        let lb_idx = match cfg.lookback {
            3 => 0,
            6 => 1,
            _ => 2, // 12
        };
        let data = build_data(lb_idx);

        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };
        let entries_filtered_total: usize = coin_results.iter().map(|c| c.entries_filtered).sum();
        let total_signals = total_trades + entries_filtered_total;
        let filter_rate = if total_signals > 0 { entries_filtered_total as f64 / total_signals as f64 * 100.0 } else { 0.0 };

        let d_count = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  filtered={} ({:.1}%)",
            d_count, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, entries_filtered_total, filter_rate);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, entries_filtered_total, filter_rate }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN108 Momentum Hour Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Filtered", "FilterRate");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.entries_filtered_total, r.filter_rate);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN108 momentum hour filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, filter_rate={:.1}%)",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl, best.filter_rate);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run108_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run108_1_results.json");
}
