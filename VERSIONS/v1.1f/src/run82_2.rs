/// RUN82.2 — Walk-Forward Validation for Regime Decay Detection
///
/// Best config from grid: AR25_RST_G5 (ADX_RISE=25.0, REGIME_SHIFT=true, GRACE=5)
/// 3-window walk-forward: train 2mo / test 1mo
///
/// Run: cargo run --release --features run82_2 -- --run82_2

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

// Best config from grid search
const BEST_CFG: (&str, f64, bool, usize) = ("AR25_RST_G5", 25.0, true, 5);

const WINDOWS: [(usize, usize, usize); 3] = [
    (0, 5760, 8640),      // train 0-5760, test 5760-8640
    (2880, 8640, 11520),  // train 2880-8640, test 8640-11520
    (5760, 11520, 14400), // train 5760-11520, test 11520-14400
];

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    adx: Vec<f64>,
    regime: Vec<i8>,
}

#[derive(Serialize)]
struct WfWin { window: usize, train_pnl_baseline: f64, train_pnl_best: f64, train_delta: f64,
    test_pnl_baseline: f64, test_pnl_best: f64, test_delta: f64, pass: bool }
#[derive(Serialize)]
struct Output { best_config: String, adx_rise: f64, regime_shift: bool, grace: usize, windows: Vec<WfWin> }

fn truerange(h: f64, l: f64, pc: f64) -> f64 {
    (h - l).max((h - pc).abs()).max((l - pc).abs())
}

fn load_15m(coin: &str) -> Option<CoinData15m> {
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
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
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

    let period = 14;
    let mut tr = vec![f64::NAN; n];
    let mut dm_plus = vec![f64::NAN; n];
    let mut dm_minus = vec![f64::NAN; n];
    for i in 1..n {
        tr[i] = truerange(highs[i], lows[i], closes[i-1]);
        let up_move = highs[i] - highs[i-1];
        let down_move = lows[i-1] - lows[i];
        if up_move > down_move && up_move > 0.0 { dm_plus[i] = up_move; dm_minus[i] = 0.0; }
        else if down_move > up_move && down_move > 0.0 { dm_plus[i] = 0.0; dm_minus[i] = down_move; }
        else { dm_plus[i] = 0.0; dm_minus[i] = 0.0; }
    }

    let mut adx = vec![f64::NAN; n];
    let mut dx = vec![f64::NAN; n];
    let mut smooth_tr = 0.0;
    let mut smooth_dm_plus = 0.0;
    let mut smooth_dm_minus = 0.0;
    for i in 1..=period {
        smooth_tr += tr[i];
        smooth_dm_plus += dm_plus[i];
        smooth_dm_minus += dm_minus[i];
    }
    if smooth_tr > 0.0 {
        let di_plus = (smooth_dm_plus / smooth_tr) * 100.0;
        let di_minus = (smooth_dm_minus / smooth_tr) * 100.0;
        let di_sum = di_plus + di_minus;
        dx[period] = if di_sum > 0.0 { (di_plus - di_minus).abs() / di_sum * 100.0 } else { 0.0 };
        adx[period] = dx[period];
    }
    for i in (period+1)..n {
        smooth_tr = smooth_tr - smooth_tr / period as f64 + tr[i];
        smooth_dm_plus = smooth_dm_plus - smooth_dm_plus / period as f64 + dm_plus[i];
        smooth_dm_minus = smooth_dm_minus - smooth_dm_minus / period as f64 + dm_minus[i];
        let di_plus = if smooth_tr > 0.0 { (smooth_dm_plus / smooth_tr) * 100.0 } else { 0.0 };
        let di_minus = if smooth_tr > 0.0 { (smooth_dm_minus / smooth_tr) * 100.0 } else { 0.0 };
        let di_sum = di_plus + di_minus;
        dx[i] = if di_sum > 0.0 { (di_plus - di_minus).abs() / di_sum * 100.0 } else { 0.0 };
        adx[i] = adx[i-1] - adx[i-1] / period as f64 + dx[i];
    }

    let mut regime = vec![0i8; n];
    for i in period..n {
        regime[i] = if adx[i] >= 25.0 { 1 } else { 0 };
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, adx, regime })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate_range(d: &CoinData15m, train_start: usize, train_end: usize, test_start: usize, test_end: usize, adx_rise: f64, regime_shift: bool, grace: usize) -> (f64, f64) {
    // Baseline: no decay exit
    let baseline_pnl = simulate_segment(d, train_start, train_end, 0.0, false, 0)
        + simulate_segment(d, test_start, test_end, 0.0, false, 0);
    // Best: with decay exit
    let best_pnl = simulate_segment(d, train_start, train_end, adx_rise, regime_shift, grace)
        + simulate_segment(d, test_start, test_end, adx_rise, regime_shift, grace);
    (baseline_pnl, best_pnl)
}

fn simulate_segment(d: &CoinData15m, start: usize, end: usize, adx_rise: f64, regime_shift: bool, grace: usize) -> f64 {
    let is_baseline = adx_rise == 0.0;
    let n = end.min(d.closes.len());
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None;
    let mut cooldown = 0usize;

    for i in start..n {
        if let Some((dir, entry, entry_bar, entry_adx)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed && !is_baseline {
                let bars_held = i - entry_bar;
                if bars_held >= grace {
                    if adx_rise > 0.0 && !d.adx[i].is_nan() && !entry_adx.is_nan() {
                        if d.adx[i] >= entry_adx + adx_rise {
                            exit_pct = pct; closed = true;
                        }
                    }
                    if !closed && regime_shift && !entry_adx.is_nan() && entry_adx < 25.0 && d.adx[i] >= 25.0 {
                        let current_z = d.zscore[i];
                        if (dir == 1 && current_z > 1.0 && d.regime[i] == 1) ||
                           (dir == -1 && current_z < -1.0 && d.regime[i] == 1) {
                            exit_pct = pct; closed = true;
                        }
                    }
                }
            }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
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
                        let entry_adx = if !d.adx[i].is_nan() { d.adx[i] } else { 20.0 };
                        pos = Some((dir, entry_price, i, entry_adx));
                    }
                }
            }
        }
    }
    bal - INITIAL_BAL
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN82.2 — Walk-Forward Validation: Regime Decay\n");
    eprintln!("Loading data...");
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let (cfg_label, adx_rise, regime_shift, grace) = BEST_CFG;
    eprintln!("Best config: {} (ADX_RISE={}, REGIME_SHIFT={}, GRACE={})\n", cfg_label, adx_rise, regime_shift, grace);

    let mut results: Vec<WfWin> = Vec::new();
    for (wi, &(train_s, train_e, test_e)) in WINDOWS.iter().enumerate() {
        let test_s = train_e;
        eprintln!("Window {}: train={}-{}, test={}-{}", wi+1, train_s, train_e, test_s, test_e);

        let mut train_baseline = 0.0;
        let mut train_best = 0.0;
        let mut test_baseline = 0.0;
        let mut test_best = 0.0;

        for d in &coin_data {
            let (tb, tp) = simulate_range(d, train_s, train_e, test_s, test_e, 0.0, false, 0);
            let (tbb, tpp) = simulate_range(d, train_s, train_e, test_s, test_e, adx_rise, regime_shift, grace);
            train_baseline += tb;
            train_best += tp;
            test_baseline += tbb;
            test_best += tpp;
        }

        let train_delta = train_best - train_baseline;
        let test_delta = test_best - test_baseline;
        let pass = test_delta > 0.0;
        eprintln!("  Train: baseline={:+.2}, best={:+.2}, delta={:+.2}", train_baseline, train_best, train_delta);
        eprintln!("  Test:  baseline={:+.2}, best={:+.2}, delta={:+.2}  --> {}", test_baseline, test_best, test_delta, if pass { "PASS" } else { "FAIL" });

        results.push(WfWin {
            window: wi + 1,
            train_pnl_baseline: train_baseline,
            train_pnl_best: train_best,
            train_delta,
            test_pnl_baseline: test_baseline,
            test_pnl_best: test_best,
            test_delta,
            pass,
        });
    }

    let passes = results.iter().filter(|r| r.pass).count();
    let avg_test_delta: f64 = results.iter().map(|r| r.test_delta).sum::<f64>() / 3.0;
    let is_positive = passes >= 2 && avg_test_delta > 0.0;

    println!("\n=== RUN82.2 Walk-Forward Summary ===");
    println!("Config: {}", cfg_label);
    println!("{:>3}  {:>12}  {:>12}  {:>10}  {:>12}  {:>12}  {:>10}  {:>5}",
        "Win", "Train(Base)", "Train(Best)", "Δ(train)", "Test(Base)", "Test(Best)", "Δ(test)", "Pass?");
    println!("{}", "-".repeat(80));
    for r in &results {
        println!("  {}  {:>+12.2}  {:>+12.2}  {:>+10.2}  {:>+12.2}  {:>+12.2}  {:>+10.2}  {:>5}",
            r.window, r.train_pnl_baseline, r.train_pnl_best, r.train_delta,
            r.test_pnl_baseline, r.test_pnl_best, r.test_delta, r.pass);
    }
    println!("{}", "=".repeat(80));
    println!("Passes: {}/3  Avg test Δ: {:+.2}", passes, avg_test_delta);
    println!("VERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let output = Output {
        best_config: cfg_label.to_string(),
        adx_rise,
        regime_shift,
        grace,
        windows: results,
    };
    std::fs::write("/home/scamarena/ProjectCoin/run82_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run82_2_results.json");
}
