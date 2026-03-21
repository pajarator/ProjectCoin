/// RUN76 — Volatility-Adaptive Stop Loss
///
/// Scale SL inversely with ATR percentile rank: tight in high-vol, wide in low-vol.
/// SL = BASE / atr_pct_rank, clamped [SL_MIN, SL_MAX].
///
/// Grid: BASE [0.0020, 0.0030, 0.0040] × WINDOW [50, 100, 200] × MIN [0.0010, 0.0015, 0.0020] × MAX [0.0050, 0.0060, 0.0080]
/// Total: 3 × 3 × 3 × 3 = 81 + baseline = 82 configs
///
/// Run: cargo run --release --features run76 -- --run76

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;
const FIXED_SL: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct VolSlCfg {
    base: f64,
    pct_window: usize,
    sl_min: f64,
    sl_max: f64,
    is_baseline: bool,
}

impl VolSlCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("B{:.4}_W{}_MIN{:.4}_MAX{:.4}", self.base, self.pct_window, self.sl_min, self.sl_max) }
    }
}

fn build_grid() -> Vec<VolSlCfg> {
    let mut grid = vec![VolSlCfg { base: 0.0, pct_window: 0, sl_min: 0.0, sl_max: 0.0, is_baseline: true }];
    let bases = [0.0020, 0.0030, 0.0040];
    let windows = [50, 100, 200];
    let mins = [0.0010, 0.0015, 0.0020];
    let maxs = [0.0050, 0.0060, 0.0080];
    for &base in &bases {
        for &pw in &windows {
            for &min in &mins {
                for &max in &maxs {
                    grid.push(VolSlCfg { base, pct_window: pw, sl_min: min, sl_max: max, is_baseline: false });
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
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    atr_pct: Vec<f64>, // ATR as % of price
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, avg_sl: f64 }
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
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
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

    // ATR(14) as % of price
    let mut tr = vec![f64::NAN; n];
    let mut atr = vec![f64::NAN; n];
    for i in 1..n {
        tr[i] = (highs[i] - lows[i]).max((highs[i] - closes[i-1]).abs()).max((lows[i] - closes[i-1]).abs());
    }
    for i in 14..n {
        atr[i] = tr[i+1-14..=i].iter().filter(|&&x| !x.is_nan()).sum::<f64>() / 14.0;
    }
    let mut atr_pct = vec![f64::NAN; n];
    for i in 14..n {
        atr_pct[i] = atr[i] / closes[i];
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, atr_pct })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Compute ATR percentile rank
fn atr_percentile_rank(atr_pct_history: &[f64], current: f64) -> f64 {
    if atr_pct_history.is_empty() { return 0.5; }
    let below = atr_pct_history.iter().filter(|&&x| x < current).count() as f64;
    below / atr_pct_history.len() as f64
}

// Compute effective SL for a given config at current bar
fn effective_sl(cfg: &VolSlCfg, atr_pct_history: &[f64], current_atr_pct: f64) -> f64 {
    if cfg.is_baseline { return FIXED_SL; }
    let rank = atr_percentile_rank(atr_pct_history, current_atr_pct);
    let clamped_rank = rank.clamp(0.10, 0.90);
    let adaptive_sl = cfg.base / clamped_rank;
    adaptive_sl.clamp(cfg.sl_min, cfg.sl_max)
}

fn simulate(d: &CoinData15m, cfg: &VolSlCfg) -> (f64, usize, usize, usize, f64, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    // (dir, entry_price, entry_bar, eff_sl_at_entry)
    let mut pos: Option<(i8, f64, usize, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut atr_hist: Vec<f64> = Vec::new();
    let mut total_sl_sum = 0.0f64;
    let mut sl_count = 0usize;

    for i in 1..n {
        // Update ATR history
        let current_atr = d.atr_pct[i];
        if !current_atr.is_nan() && current_atr > 0.0 {
            atr_hist.push(current_atr);
            if atr_hist.len() > 200 { atr_hist.remove(0); }
        }

        if let Some((dir, entry, entry_bar, eff_sl)) = pos {
            let held = i - entry_bar;
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL exit — use fixed SL computed at entry
            if pct <= -eff_sl { exit_pct = -eff_sl; closed = true; total_sl_sum += eff_sl; sl_count += 1; }

            // Z-cross exit (after min hold)
            if !closed && held >= MIN_HOLD_BARS {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }

            // Force close on last bar
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * BASE_POSITION_SIZE * LEVERAGE * exit_pct;
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
                        let eff_sl = effective_sl(cfg, &atr_hist, current_atr);
                        pos = Some((dir, entry_price, i, eff_sl));
                    }
                }
            }
        }
    }

    let trades = wins + losses + flats;
    let avg_sl = if sl_count > 0 { total_sl_sum / sl_count as f64 } else { FIXED_SL };
    (bal - INITIAL_BAL, wins, losses, flats, avg_sl, sl_count)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN76 — Volatility-Adaptive Stop Loss\n");
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
                pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![],
            };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, avg_sl, _sl_count) = simulate(d, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, avg_sl }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * FIXED_SL * BASE_POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * FIXED_SL * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let avg_sl_all: f64 = coin_results.iter().map(|c| c.avg_sl * c.trades as f64).sum::<f64>() / total_trades.max(1) as f64;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  avgSL={:.4}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades, avg_sl_all);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN76 Volatility-Adaptive Stop Loss Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<35} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<35} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN76 vol-adaptive SL. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run76_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run76_1_results.json");
}
