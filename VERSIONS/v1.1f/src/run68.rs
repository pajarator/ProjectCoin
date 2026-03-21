/// RUN68 — BTC Correlation Position Sizing
///
/// Grid: CORR_LOOKBACK [10, 15, 20] × CORR_MULTIPLIER [0.3, 0.5, 0.7] × CORR_MIN [0.2, 0.4]
/// Total: 3 × 3 × 2 = 18 configs + baseline = 19
///
/// Higher BTC correlation → larger position. Position = base × (1 + mult × corr), clamped.
/// Low correlation (< min_thresh) → reduce position to 50%.
///
/// Run: cargo run --release --features run68 -- --run68

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
struct CorrSizingCfg {
    corr_lookback: usize,
    corr_multiplier: f64,
    corr_min_thresh: f64,
}

impl CorrSizingCfg {
    fn label(&self) -> String {
        format!("L{}_M{:.1}_T{:.1}", self.corr_lookback, self.corr_multiplier, self.corr_min_thresh)
    }
}

fn build_grid() -> Vec<CorrSizingCfg> {
    let mut grid = vec![]; // baseline (no correlation sizing)
    let looks = [10, 15, 20];
    let mults = [0.3, 0.5, 0.7];
    let threshs = [0.2, 0.4];
    for &l in &looks {
        for &m in &mults {
            for &t in &threshs {
                grid.push(CorrSizingCfg { corr_lookback: l, corr_multiplier: m, corr_min_thresh: t });
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
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
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

    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Compute rolling correlation between coin returns and btc returns
fn compute_btc_corr(coin: &[f64], btc: &[f64], lookback: usize) -> Vec<f64> {
    let n = coin.len().min(btc.len());
    let mut corr = vec![f64::NAN; n];
    for i in lookback..n {
        // Compute returns
        let mut coin_ret = Vec::with_capacity(lookback);
        let mut btc_ret = Vec::with_capacity(lookback);
        for j in i+1-lookback..=i {
            if j > 0 {
                coin_ret.push((coin[j] - coin[j-1]) / coin[j-1]);
                btc_ret.push((btc[j] - btc[j-1]) / btc[j-1]);
            }
        }
        if coin_ret.len() < lookback { continue; }
        let coin_mean = coin_ret.iter().sum::<f64>() / lookback as f64;
        let btc_mean = btc_ret.iter().sum::<f64>() / lookback as f64;
        let mut cov = 0.0;
        let mut coin_var = 0.0;
        let mut btc_var = 0.0;
        for j in 0..lookback {
            let cd = coin_ret[j] - coin_mean;
            let bd = btc_ret[j] - btc_mean;
            cov += cd * bd;
            coin_var += cd * cd;
            btc_var += bd * bd;
        }
        let den = (coin_var * btc_var).sqrt();
        corr[i] = if den > 0.0 { cov / den } else { 0.0 };
    }
    corr
}

fn simulate(d: &CoinData15m) -> (f64, usize, usize, usize) {
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
                let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
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
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

// Simulate with correlation-adjusted position sizing
fn simulate_corr(d: &CoinData15m, btc_corr: &[f64], cfg: CorrSizingCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, f64)> = None; // dir, entry_price, corr_mult
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for i in 1..n {
        if let Some((dir, entry, corr_mult_at_entry)) = pos {
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
                let pos_size = BASE_POSITION_SIZE * corr_mult_at_entry;
                let net = (bal * pos_size * LEVERAGE) * exit_pct;
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
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 {
                        let corr = btc_corr.get(i).copied().unwrap_or(f64::NAN);
                        let corr_mult = if corr.is_nan() || corr < cfg.corr_min_thresh {
                            0.5 // low correlation = reduce position
                        } else {
                            (1.0 + cfg.corr_multiplier * corr).min(1.0 + cfg.corr_multiplier)
                        };
                        pos = Some((dir, entry_price, corr_mult));
                    }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN68 — BTC Correlation Position Sizing\n");
    eprintln!("Loading 15m data for {} coins + BTC...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing coin data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    // Load BTC separately for correlation
    let btc_data = load_15m("BTC");
    if btc_data.is_none() { eprintln!("Missing BTC data!"); return; }
    let btc_data = btc_data.unwrap();
    eprintln!("  BTC — {} bars (for correlation)", btc_data.closes.len());

    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    // Precompute BTC correlations for each coin and each lookback
    let corr_lookbacks = [10usize, 15, 20];
    let mut btc_corrs: Vec<Vec<Vec<f64>>> = Vec::new(); // [coin][lookback][bar]
    for d in &coin_data {
        let mut coin_corrs: Vec<Vec<f64>> = Vec::new();
        for &lb in &corr_lookbacks {
            coin_corrs.push(compute_btc_corr(&d.closes, &btc_data.closes, lb));
        }
        btc_corrs.push(coin_corrs);
    }

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0,
                total_trades: 0, pf: 0.0, is_baseline: false,
            };
        }
        let mut total_pnl = 0.0;
        let mut wins_sum = 0usize;
        let mut losses_sum = 0usize;
        let mut flats_sum = 0usize;

        for (ci, d) in coin_data.iter().enumerate() {
            // Find which corr index matches this cfg's lookback
            let lb_idx = corr_lookbacks.iter().position(|&l| l == cfg.corr_lookback).unwrap_or(1);
            let btc_corr = &btc_corrs[ci][lb_idx];

            let (pnl, wins, losses, flats) = simulate_corr(d, btc_corr, *cfg);
            total_pnl += pnl;
            wins_sum += wins;
            losses_sum += losses;
            flats_sum += flats;
        }

        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if losses_sum > 0 { gross / (losses_sum as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(),
            total_pnl,
            portfolio_wr,
            total_trades,
            pf,
            is_baseline: false,
        }
    }).collect();

    // Also compute baseline (no correlation sizing)
    if shutdown.load(Ordering::SeqCst) { return; }
    let mut baseline_pnl = 0.0;
    let mut baseline_wins = 0usize;
    let mut baseline_losses = 0usize;
    let mut baseline_flats = 0usize;
    for d in &coin_data {
        let (pnl, wins, losses, flats) = simulate(d);
        baseline_pnl += pnl;
        baseline_wins += wins;
        baseline_losses += losses;
        baseline_flats += flats;
    }
    let baseline_trades = baseline_wins + baseline_losses + baseline_flats;
    let baseline_wr = if baseline_trades > 0 { baseline_wins as f64 / baseline_trades as f64 * 100.0 } else { 0.0 };
    let baseline_gross = baseline_wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
    let baseline_pf = if baseline_losses > 0 { baseline_gross / (baseline_losses as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

    let baseline_result = ConfigResult {
        label: "BASELINE".to_string(),
        total_pnl: baseline_pnl,
        portfolio_wr: baseline_wr,
        total_trades: baseline_trades,
        pf: baseline_pf,
        is_baseline: true,
    };

    let mut all_results = vec![baseline_result];
    all_results.extend(results);

    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN68 BTC Correlation Position Sizing Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline_pnl, baseline_wr, baseline_trades);
    println!("\n{:>3}  {:<15} {:>8} {:>8} {:>6}  {:>6}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(58));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline_pnl;
        println!("{:>3}  {:<15} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(58));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN68 BTC correlation sizing. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len() - 1, baseline_pnl, best.label, best.total_pnl, best.total_pnl - baseline_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run68_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run68_1_results.json");
}