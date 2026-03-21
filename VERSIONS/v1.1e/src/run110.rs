/// RUN110 — BB Width Compression Entry: Enter When BB Width Has Been Compressed
///
/// Grid: THRESH [0.60, 0.70, 0.80] × BARS [3, 4, 5] × Z_RELAX [0.10, 0.20, 0.30]
/// 27 configs × 18 coins = 486 simulations (parallel per coin)
///
/// Run: cargo run --release --features run110 -- --run110

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
struct BbCompCfg {
    bb_thresh: f64,
    bb_bars: usize,
    z_relax: f64,
    is_baseline: bool,
}

impl BbCompCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("BT{:.2}_BB{}_ZR{:.2}", self.bb_thresh, self.bb_bars, self.z_relax) }
    }
}

fn build_grid() -> Vec<BbCompCfg> {
    let mut grid = vec![BbCompCfg { bb_thresh: 999.0, bb_bars: 999, z_relax: 0.0, is_baseline: true }];
    for &bt in &[0.60, 0.70, 0.80] {
        for &bb in &[3usize, 4, 5] {
            for &zr in &[0.10, 0.20, 0.30] {
                grid.push(BbCompCfg { bb_thresh: bt, bb_bars: bb, z_relax: zr, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData {
    name: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    bb_width: Vec<f64>,
    bb_width_avg: Vec<f64>,
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
    compression_entries: usize,
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
    compression_entries_total: usize,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s=&data[i+1-w..=i]; let m=s.iter().sum::<f64>()/w as f64; let v=s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}

fn load_15m(coin: &str) -> Option<CoinData> {
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

    // BB width
    let bb_sma = rmean(&closes, 20);
    let bb_std_dev = rstd(&closes, 20);
    let mut bb_upper = vec![f64::NAN; n]; let mut bb_lower = vec![f64::NAN; n]; let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !bb_sma[i].is_nan() && !bb_std_dev[i].is_nan() {
            bb_upper[i] = bb_sma[i] + 2.0 * bb_std_dev[i];
            bb_lower[i] = bb_sma[i] - 2.0 * bb_std_dev[i];
            bb_width_raw[i] = bb_upper[i] - bb_lower[i];
        }
    }
    let bb_width_avg = rmean(&bb_width_raw, 20);

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, bb_width: bb_width_raw, bb_width_avg })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &BbCompCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut compression_entries = 0usize;

    // BB compression counter
    let mut bb_comp_count = 0usize;

    for i in 20..n {
        cooldown = cooldown.saturating_sub(1);

        // Update BB compression counter
        if !cfg.is_baseline {
            let bb_ratio = if !d.bb_width_avg[i].is_nan() && d.bb_width_avg[i] > 0.0 {
                d.bb_width[i] / d.bb_width_avg[i]
            } else {
                999.0
            };
            if bb_ratio < cfg.bb_thresh {
                bb_comp_count += 1;
            } else {
                bb_comp_count = 0;
            }
        }

        if let Some((dir, entry, entry_bar, _z_entry)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
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
                let base_thresh = 2.0;
                if cfg.is_baseline {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            pos = Some((dir, entry_price, i, d.zscore[i]));
                        }
                    }
                } else {
                    // Apply compression relaxation
                    let effective_thresh = if bb_comp_count >= cfg.bb_bars {
                        base_thresh - cfg.z_relax
                    } else {
                        base_thresh
                    };

                    let z = d.zscore[i];
                    let passes_thresh = if dir == 1 { z < -effective_thresh } else { z > effective_thresh };

                    if passes_thresh {
                        if i + 1 < n {
                            let entry_price = d.opens[i + 1];
                            if entry_price > 0.0 {
                                pos = Some((dir, entry_price, i, z));
                                if bb_comp_count >= cfg.bb_bars {
                                    compression_entries += 1;
                                }
                                bb_comp_count = 0; // reset after entry
                            }
                        }
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

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, compression_entries }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN110 — BB Width Compression Entry Grid Search\n");
    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], compression_entries_total: 0 };
        }
        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };
        let compression_entries_total: usize = coin_results.iter().map(|c| c.compression_entries).sum();

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  comp_entries={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, compression_entries_total);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, compression_entries_total }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN110 BB Width Compression Entry Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "CompEntries");
    println!("{}", "-".repeat(72));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>10}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.compression_entries_total);
    }
    println!("{}", "=".repeat(72));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN110 BB compression entry. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run110_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run110_1_results.json");
}
