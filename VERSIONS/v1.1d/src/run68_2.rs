/// RUN68.2 — BTC Correlation Position Sizing: Walk-Forward Validation
///
/// 3-window walk-forward: train 2mo, test 1mo
/// For each window: find best CORR_LOOKBACK × CORR_MULTIPLIER × CORR_MIN per coin
/// Compare: per-coin best vs universal (portfolio-level best) vs baseline
///
/// Run: cargo run --release --features run68_2 -- --run68-2

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
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

#[derive(Clone, Debug)]
struct CorrSizingCfg {
    corr_lookback: usize,
    corr_multiplier: f64,
    corr_min_thresh: f64,
    label: String,
}

impl CorrSizingCfg {
    fn new(lb: usize, m: f64, t: f64) -> Self {
        let label = format!("L{}_M{:.1}_T{:.1}", lb, m, t);
        Self { corr_lookback: lb, corr_multiplier: m, corr_min_thresh: t, label }
    }
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
struct WindowResult {
    window: usize,
    train_pnl: f64,
    test_pnl: f64,
    test_delta: f64,
    best_cfg: String,
    best_train_pnl: f64,
}

#[derive(Serialize)]
struct WFOutput {
    windows: Vec<WindowResult>,
    summary: String,
}

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

fn compute_btc_corr(coin: &[f64], btc: &[f64], lookback: usize) -> Vec<f64> {
    let n = coin.len().min(btc.len());
    let mut corr = vec![f64::NAN; n];
    for i in lookback..n {
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

fn simulate_baseline(d: &CoinData15m, start: usize, end: usize) -> f64 {
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    for i in start..end.min(d.closes.len()) {
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
                bal += (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < d.closes.len() {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    bal - INITIAL_BAL
}

fn simulate_corr(d: &CoinData15m, btc_corr: &[f64], cfg: &CorrSizingCfg, start: usize, end: usize) -> f64 {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, f64)> = None;
    let mut cooldown = 0usize;
    for i in start..end.min(n) {
        if let Some((dir, entry, cm)) = pos {
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
                let pos_size = BASE_POSITION_SIZE * cm;
                bal += (bal * pos_size * LEVERAGE) * exit_pct;
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
                            0.5
                        } else {
                            (1.0 + cfg.corr_multiplier * corr).min(1.0 + cfg.corr_multiplier)
                        };
                        pos = Some((dir, entry_price, corr_mult));
                    }
                }
            }
        }
    }
    bal - INITIAL_BAL
}

// 5 months ≈ 150 days ≈ 14400 15m bars
// Window 1: train bars 0-5760 (≈2mo), test 5760-8640 (≈1mo)
// Window 2: train bars 2880-8640, test 8640-11520
// Window 3: train bars 5760-11520, test 11520-14400
const WINDOWS: [(usize, usize, usize); 3] = [
    (0, 5760, 8640),      // window 1: train 0-5760, test 5760-8640
    (2880, 8640, 11520),  // window 2: train 2880-8640, test 8640-11520
    (5760, 11520, 14400), // window 3: train 5760-11520, test 11520-14400
];

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN68.2 — BTC Correlation Walk-Forward\n");

    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let btc_data = load_15m("BTC").unwrap();

    // All grid configs
    let all_cfgs: Vec<CorrSizingCfg> = vec![
        CorrSizingCfg::new(10, 0.3, 0.2), CorrSizingCfg::new(10, 0.3, 0.4),
        CorrSizingCfg::new(10, 0.5, 0.2), CorrSizingCfg::new(10, 0.5, 0.4),
        CorrSizingCfg::new(10, 0.7, 0.2), CorrSizingCfg::new(10, 0.7, 0.4),
        CorrSizingCfg::new(15, 0.3, 0.2), CorrSizingCfg::new(15, 0.3, 0.4),
        CorrSizingCfg::new(15, 0.5, 0.2), CorrSizingCfg::new(15, 0.5, 0.4),
        CorrSizingCfg::new(15, 0.7, 0.2), CorrSizingCfg::new(15, 0.7, 0.4),
        CorrSizingCfg::new(20, 0.3, 0.2), CorrSizingCfg::new(20, 0.3, 0.4),
        CorrSizingCfg::new(20, 0.5, 0.2), CorrSizingCfg::new(20, 0.5, 0.4),
        CorrSizingCfg::new(20, 0.7, 0.2), CorrSizingCfg::new(20, 0.7, 0.4),
    ];

    // Precompute correlations for each coin, each lookback
    let lookbacks = [10usize, 15, 20];
    let mut btc_corrs: Vec<Vec<Vec<f64>>> = Vec::new();
    for d in &coin_data {
        let mut coin_corrs = Vec::new();
        for &lb in &lookbacks {
            coin_corrs.push(compute_btc_corr(&d.closes, &btc_data.closes, lb));
        }
        btc_corrs.push(coin_corrs);
    }

    let mut window_results: Vec<WindowResult> = Vec::new();

    for (wi, (train_start, train_end, test_start)) in WINDOWS.iter().enumerate() {
        eprintln!("\nWindow {}: train [{}-{}] test [{}-{}]", wi+1, train_start, train_end, test_start, 14400);

        // Per-coin: find best config in training window
        let mut coin_best_cfgs: Vec<CorrSizingCfg> = Vec::new();
        for ci in 0..N_COINS {
            let d = &coin_data[ci];
            let lb_idx = 1; // use lookback=15 as default
            let btc_corr = &btc_corrs[ci][lb_idx];

            let mut best_pnl = f64::NEG_INFINITY;
            let mut best_cfg = all_cfgs[0].clone();
            for cfg in &all_cfgs {
                let pnl = simulate_corr(d, btc_corr, cfg, *train_start, *train_end);
                if pnl > best_pnl { best_pnl = pnl; best_cfg = cfg.clone(); }
            }
            coin_best_cfgs.push(best_cfg);
        }

        // Universal: find best portfolio-level config in training window
        let mut cfg_pnl = vec![0.0f64; all_cfgs.len()];
        for ci in 0..N_COINS {
            let d = &coin_data[ci];
            let lb_idx = 1;
            let btc_corr = &btc_corrs[ci][lb_idx];
            for (ci2, cfg) in all_cfgs.iter().enumerate() {
                cfg_pnl[ci2] += simulate_corr(d, btc_corr, cfg, *train_start, *train_end);
            }
        }
        let universal_idx = cfg_pnl.iter().enumerate().fold(0, |best_i, (i, &p)|
            if p > cfg_pnl[best_i] { i } else { best_i });
        let universal_cfg = &all_cfgs[universal_idx];

        // Evaluate on test window
        let mut total_baseline_test = 0.0;
        let mut total_percoin_test = 0.0;
        let mut total_universal_test = 0.0;
        let mut total_train = 0.0;

        for ci in 0..N_COINS {
            let d = &coin_data[ci];
            let lb_idx = 1;
            let btc_corr = &btc_corrs[ci][lb_idx];

            let train = simulate_corr(d, btc_corr, &all_cfgs[0], *train_start, *train_end); // placeholder
            total_train += train;

            let baseline_test = simulate_baseline(d, *test_start, 14400);
            total_baseline_test += baseline_test;

            let percoin_test = simulate_corr(d, btc_corr, &coin_best_cfgs[ci], *test_start, 14400);
            total_percoin_test += percoin_test;

            let universal_test = simulate_corr(d, btc_corr, universal_cfg, *test_start, 14400);
            total_universal_test += universal_test;
        }

        let percoin_delta = total_percoin_test - total_baseline_test;
        let universal_delta = total_universal_test - total_baseline_test;

        eprintln!("  Baseline test PnL: {:+.2}", total_baseline_test);
        eprintln!("  Per-coin best test PnL: {:+.2} (Δ={:+.2})", total_percoin_test, percoin_delta);
        eprintln!("  Universal ({}) test PnL: {:+.2} (Δ={:+.2})", universal_cfg.label, total_universal_test, universal_delta);

        window_results.push(WindowResult {
            window: wi + 1,
            train_pnl: total_train,
            test_pnl: total_baseline_test, // placeholder
            test_delta: universal_delta,
            best_cfg: universal_cfg.label.clone(),
            best_train_pnl: cfg_pnl[universal_idx],
        });
    }

    // Summary
    let avg_delta: f64 = window_results.iter().map(|w| w.test_delta).sum::<f64>() / 3.0;
    let all_positive = window_results.iter().all(|w| w.test_delta > 0.0);
    let pos_count = window_results.iter().filter(|w| w.test_delta > 0.0).count();

    eprintln!("\n=== RUN68.2 Walk-Forward Summary ===");
    eprintln!("Windows with positive OOS delta: {}/3", pos_count);
    eprintln!("Average OOS delta: {:+.2}", avg_delta);
    eprintln!("VERDICT: {}", if all_positive { "POSITIVE" } else { "NEGATIVE" });

    let summary = format!("RUN68.2 walk-forward: {}/3 windows positive, avg delta={:+.2}", pos_count, avg_delta);
    let output = WFOutput { windows: window_results, summary };
    std::fs::write("/home/scamarena/ProjectCoin/run68_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run68_2_results.json");
}