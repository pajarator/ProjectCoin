/// RUN128 — Schaff Trend Cycle Confirmation: MACD+Stochastic Hybrid Filter
///
/// Grid: LONG_THRESH [20, 25, 30] × SHORT_THRESH [70, 75, 80] × TURN [true, false]
/// 18 configs × 18 coins = 324 simulations (parallel per coin)
///
/// Run: cargo run --release --features run128 -- --run128

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
struct StcCfg {
    long_thresh: f64,
    short_thresh: f64,
    turn_required: bool,
    is_baseline: bool,
}

impl StcCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let t = if self.turn_required { "T" } else { "N" };
            format!("LT{}_ST{}_{}", self.long_thresh as usize, self.short_thresh as usize, t)
        }
    }
}

fn build_grid() -> Vec<StcCfg> {
    let mut grid = vec![StcCfg { long_thresh: 0.0, short_thresh: 0.0, turn_required: false, is_baseline: true }];
    for &lt in &[20.0f64, 25.0, 30.0] {
        for &st in &[70.0f64, 75.0, 80.0] {
            for &tr in &[true, false] {
                grid.push(StcCfg { long_thresh: lt, short_thresh: st, turn_required: tr, is_baseline: false });
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
    stc: Vec<f64>,
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

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn compute_ema(data: &[f64], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if period == 0 || period > n { return out; }
    let alpha = 2.0 / (period as f64 + 1.0);
    out[period-1] = data[0..period].iter().sum::<f64>() / period as f64;
    let mut ema_val = out[period-1];
    for i in period..n {
        ema_val = alpha * data[i] + (1.0 - alpha) * ema_val;
        out[i] = ema_val;
    }
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

    // Compute STC: MACD with stochastic cycle smoothing
    let fast = 12; let slow = 26; let cycle = 10;
    let ema_fast = compute_ema(&closes, fast);
    let ema_slow = compute_ema(&closes, slow);
    let mut macd = vec![f64::NAN; n];
    for i in 0..n {
        if !ema_fast[i].is_nan() && !ema_slow[i].is_nan() {
            macd[i] = ema_fast[i] - ema_slow[i];
        }
    }

    // Stochastic of MACD over cycle period, then EMA smooth
    let mut stc = vec![f64::NAN; n];
    for i in cycle..n {
        if macd[i].is_nan() { continue; }
        let mut min_macd = f64::MAX;
        let mut max_macd = f64::MIN;
        for j in 0..cycle {
            let idx = i - j;
            if !macd[idx].is_nan() {
                if macd[idx] < min_macd { min_macd = macd[idx]; }
                if macd[idx] > max_macd { max_macd = macd[idx]; }
            }
        }
        let range = max_macd - min_macd;
        if range > 1e-10 {
            let pct_k = (macd[i] - min_macd) / range * 100.0;
            // EMA smooth
            if !stc[i-1].is_nan() {
                let alpha = 2.0 / (cycle as f64 + 1.0);
                stc[i] = alpha * pct_k + (1.0 - alpha) * stc[i-1];
            } else {
                stc[i] = pct_k;
            }
        }
    }

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, stc })
}

fn simulate(d: &CoinData, cfg: &StcCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut entries_filtered = 0usize;

    for i in 50..n {
        cooldown = cooldown.saturating_sub(1);

        if let Some((dir, entry, entry_bar)) = pos.as_mut() {
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
                if cfg.is_baseline {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            pos = Some((dir, entry_price, i));
                        }
                    }
                } else {
                    let stc_val = d.stc[i];
                    let stc_prior = d.stc[i-1];
                    let passes_filter = if stc_val.is_nan() || stc_prior.is_nan() {
                        true
                    } else if dir == 1 {
                        // LONG: STC < long_thresh AND (not turn_required OR rising)
                        if stc_val >= cfg.long_thresh { false }
                        else if cfg.turn_required && stc_val <= stc_prior { false }
                        else { true }
                    } else {
                        // SHORT: STC > short_thresh AND (not turn_required OR falling)
                        if stc_val <= cfg.short_thresh { false }
                        else if cfg.turn_required && stc_val >= stc_prior { false }
                        else { true }
                    };

                    if passes_filter {
                        if i + 1 < n {
                            let entry_price = d.opens[i + 1];
                            if entry_price > 0.0 {
                                pos = Some((dir, entry_price, i));
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
    eprintln!("RUN128 — Schaff Trend Cycle Confirmation Grid Search\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], entries_filtered_total: 0, filter_rate: 0.0 };
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
        let entries_filtered_total: usize = coin_results.iter().map(|c| c.entries_filtered).sum();
        let total_signals = total_trades + entries_filtered_total;
        let filter_rate = if total_signals > 0 { entries_filtered_total as f64 / total_signals as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  filtered={} ({:.1}%)",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, entries_filtered_total, filter_rate);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, entries_filtered_total, filter_rate }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN128 Schaff Trend Cycle Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Filtered", "FilterRate");
    println!("{}", "-".repeat(78));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.entries_filtered_total, r.filter_rate);
    }
    println!("{}", "=".repeat(78));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN128 STC filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run128_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run128_1_results.json");
}
