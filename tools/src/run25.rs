/// RUN25 — ML-Based Regime Detection (Simplified)
///
/// Tests whether the 4-regime BTC framework (BULL/BEAR × HIGH/LOW vol) can
/// predict when COINCLAW strategies work best on individual coins.
///
/// Fix over Python stub:
///   Python trains RF/XGB to predict regime labels — but labels are
///   DEFINED by SMA50>SMA200 + vol>median, so "ML" just rediscovers its
///   own input features. Accuracy vs baseline is meaningless. Dropped.
///
///   Corrected approach: directly measure COINCLAW per-coin strategy
///   performance across 4 BTC regimes. The actionable question is:
///   "Does any BTC regime produce WR ≥ 44%?" not "can ML predict
///    the regime label with >N% accuracy?"
///
/// Regimes (on BTC 15m data):
///   0 = BEAR_LOW_VOL   (SMA50 < SMA200 AND vol ≤ median)
///   1 = BEAR_HIGH_VOL  (SMA50 < SMA200 AND vol  > median)
///   2 = BULL_LOW_VOL   (SMA50 > SMA200 AND vol ≤ median)
///   3 = BULL_HIGH_VOL  (SMA50 > SMA200 AND vol  > median)
///
/// Vol threshold: global median of train-half 20-bar return std (no look-ahead).
/// Split: 67% train / 33% OOS test. Regimes derived on full data but only
/// the test-half regime labels are used for filtering.
///
/// Strategy per coin: COINCLAW v13 primary long (from COIN_STRATEGIES).
/// Trade sim: SL=0.3% fee=0.1%/side slip=0.05%/side.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::{sma, z_score};
use crate::loader::{load_ohlcv, Bar, COIN_STRATEGIES};
use crate::strategies::signals;

const SL:   f64 = 0.003;
const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;

const REGIME_NAMES: [&str; 4] = [
    "BEAR_LOW_VOL", "BEAR_HIGH_VOL", "BULL_LOW_VOL", "BULL_HIGH_VOL"
];

// ── Rolling 20-bar return std ──────────────────────────────────────────────────
fn return_std20(close: &[f64]) -> Vec<f64> {
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    for i in 20..n {
        let rets: Vec<f64> = (i-19..=i).map(|j| (close[j] - close[j-1]) / close[j-1]).collect();
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let var = rets.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
        out[i] = var.sqrt();
    }
    out
}

// ── Label BTC regimes ────────────────────────────────────────────────────────
/// Returns regime labels (0-3) for each bar. Vol threshold = median of train half.
fn label_regimes(bars: &[Bar], split: usize) -> Vec<Option<u8>> {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();

    let sma50  = sma(&close, 50);
    let sma200 = sma(&close, 200);
    let vol    = return_std20(&close);

    // Vol threshold: median of valid vol values in train half
    let train_vols: Vec<f64> = vol[..split].iter().cloned()
        .filter(|v| v.is_finite())
        .collect();
    let vol_threshold = if train_vols.is_empty() {
        0.01
    } else {
        let mut tv = train_vols.clone();
        tv.sort_by(|a, b| a.partial_cmp(b).unwrap());
        tv[tv.len() / 2]
    };

    let mut regimes = vec![None; n];
    for i in 200..n {
        if sma50[i].is_nan() || sma200[i].is_nan() || vol[i].is_nan() { continue; }
        let is_bull     = sma50[i] > sma200[i];
        let is_high_vol = vol[i] > vol_threshold;
        regimes[i] = Some(match (is_bull, is_high_vol) {
            (false, false) => 0, // BEAR_LOW_VOL
            (false, true)  => 1, // BEAR_HIGH_VOL
            (true,  false) => 2, // BULL_LOW_VOL
            (true,  true)  => 3, // BULL_HIGH_VOL
        });
    }
    regimes
}

// ── Trade simulation ─────────────────────────────────────────────────────────
fn sim(close: &[f64], entry: &[bool], exit: &[bool]) -> (usize, f64, f64, f64) {
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;

    for i in 0..n {
        if in_pos {
            let ret = (close[i] - ep) / ep;
            let closed = if ret <= -SL {
                Some(-SL * (1.0 + SLIP) * 100.0 - FEE * 200.0)
            } else if exit[i] {
                Some((close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0)
            } else {
                None
            };
            if let Some(pnl) = closed {
                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                n_trades += 1;
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let pnl = (close[n-1] - ep) / ep * 100.0 - FEE * 200.0;
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        n_trades += 1;
    }

    if n_trades == 0 { return (0, 0.0, 0.0, 0.0); }
    let wr = wins as f64 / n_trades as f64 * 100.0;
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win };
    (n_trades, wr, pf, total_pnl)
}

/// Simulate on a masked subset of bars (regime-filtered)
fn sim_masked(close: &[f64], entry: &[bool], exit: &[bool], mask: &[bool]) -> (usize, f64, f64, f64) {
    // Collect only bars where mask is true, preserving the sequence
    // (we don't teleport into positions; use contiguous sequences within the mask)
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;

    for i in 0..n {
        if !mask[i] {
            // Regime change: close any open position at market
            if in_pos {
                let pnl = (close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0;
                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                n_trades += 1;
                in_pos = false;
            }
            continue;
        }
        if in_pos {
            let ret = (close[i] - ep) / ep;
            let closed = if ret <= -SL {
                Some(-SL * (1.0 + SLIP) * 100.0 - FEE * 200.0)
            } else if exit[i] {
                Some((close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0)
            } else {
                None
            };
            if let Some(pnl) = closed {
                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                n_trades += 1;
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let pnl = (close[n-1] - ep) / ep * 100.0 - FEE * 200.0;
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        n_trades += 1;
    }

    if n_trades == 0 { return (0, 0.0, 0.0, 0.0); }
    let wr = wins as f64 / n_trades as f64 * 100.0;
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win };
    (n_trades, wr, pf, total_pnl)
}

// ── Per-coin processing ───────────────────────────────────────────────────────
fn process_coin(coin: &str, strategy: &str, btc_regimes: &[Option<u8>]) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let split = n * 67 / 100;
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();

    let (entry_all, exit_all) = signals(&bars, strategy);

    // Test-half only
    let test_close  = &close[split..];
    let test_entry  = &entry_all[split..];
    let test_exit   = &exit_all[split..];
    let test_n      = test_close.len();

    // Align BTC regime labels to test half (regime array is same length as coin array)
    // If BTC and coin have different lengths, take from back to match test_n bars
    let btc_test_regimes: Vec<Option<u8>> = {
        let btc_len = btc_regimes.len();
        if btc_len >= test_n {
            btc_regimes[btc_len - test_n..].to_vec()
        } else {
            // BTC shorter — pad front with None
            let pad = test_n - btc_len;
            let mut v = vec![None; pad];
            v.extend_from_slice(btc_regimes);
            v
        }
    };

    // Baseline: all test bars
    let (bl_t, bl_wr, bl_pf, bl_pnl) = sim(test_close, test_entry, test_exit);

    // Per-regime
    let mut regime_results = Vec::new();
    let mut best_regime_wr = bl_wr;
    let mut best_regime_name = "baseline";

    for r in 0u8..4 {
        let mask: Vec<bool> = btc_test_regimes.iter().map(|opt| opt == &Some(r)).collect();
        let bar_count = mask.iter().filter(|&&b| b).count();
        let (rt, rwr, rpf, rpnl) = sim_masked(test_close, test_entry, test_exit, &mask);
        if rwr > best_regime_wr && rt >= 10 {
            best_regime_wr = rwr;
            best_regime_name = REGIME_NAMES[r as usize];
        }
        regime_results.push(json!({
            "regime": REGIME_NAMES[r as usize],
            "bar_pct": bar_count as f64 / test_n as f64 * 100.0,
            "t": rt, "wr": rwr, "pf": rpf, "pnl": rpnl
        }));
    }

    json!({
        "coin": coin,
        "strategy": strategy,
        "baseline": { "t": bl_t, "wr": bl_wr, "pf": bl_pf, "pnl": bl_pnl },
        "regimes": regime_results,
        "best_regime": best_regime_name,
        "best_regime_wr": best_regime_wr,
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN25 — ML-Based Regime Detection (Corrected)");
    println!("================================================================");
    println!("Fix: ML predicting its own labels is circular. Dropped RF/XGB.");
    println!("Direct test: COINCLAW per-coin WR across 4 BTC regimes.");
    println!("Regimes: BULL/BEAR × HIGH/LOW_VOL  (SMA50>SMA200, vol>median)");
    println!("Vol threshold: train-half median of 20-bar return std.");
    println!("Trade: SL=0.3% fee=0.1%/side slip=0.05%/side | Breakeven ≈ 44%");
    println!();

    // Load BTC data and compute regime labels
    let btc_bars = load_ohlcv("BTC");
    let btc_n    = btc_bars.len();
    let btc_split = btc_n * 67 / 100;
    let btc_regimes = label_regimes(&btc_bars, btc_split);

    // Print regime distribution over test half
    let btc_test_regimes = &btc_regimes[btc_split..];
    let test_n = btc_test_regimes.len();
    println!("  BTC regime distribution (OOS test half, {} bars):", test_n);
    for r in 0u8..4 {
        let cnt = btc_test_regimes.iter().filter(|&&o| o == Some(r)).count();
        println!("    {:14}: {} bars ({:.1}%)", REGIME_NAMES[r as usize], cnt, cnt as f64 / test_n as f64 * 100.0);
    }
    let none_cnt = btc_test_regimes.iter().filter(|&&o| o.is_none()).count();
    if none_cnt > 0 {
        println!("    (warmup/NaN): {} bars", none_cnt);
    }
    println!();

    // Process coins in parallel
    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.iter().map(|&(c, s)| (c, s)).collect();

    let results: Vec<Value> = coins.par_iter()
        .map(|&(coin, strat)| process_coin(coin, strat, &btc_regimes))
        .collect();

    // ── Print summary table ───────────────────────────────────────────────────
    println!("  {:6} {:>8}  {:6}  {:6}  {:6}  {:6}  | {:6}  {:6}  {:6}  {:6}  | {:6}  {:6}  {:6}  {:6}  | {:6}  {:6}  {:6}  {:6}  | {:14}",
        "Coin", "Base WR%",
        "t", "WR%", "PF", "PnL%",   // BEAR_LOW_VOL
        "t", "WR%", "PF", "PnL%",   // BEAR_HIGH_VOL
        "t", "WR%", "PF", "PnL%",   // BULL_LOW_VOL
        "t", "WR%", "PF", "PnL%",   // BULL_HIGH_VOL
        "BestRegime");
    println!("  {:6} {:>8}  {:->60}  | {:->28}  | {:->28}  | {:->28}  | {:->14}",
        "", "",
        "BEAR_LOW_VOL", "BEAR_HIGH_VOL", "BULL_LOW_VOL", "BULL_HIGH_VOL", "");

    let mut n_above_44 = 0usize;
    let mut total_regime_cells = 0usize;

    // Portfolio-level WR accumulators per regime
    let mut sum_wr = [0.0f64; 5]; // [baseline, R0, R1, R2, R3]
    let mut cnt_wr = [0usize; 5];

    for r in &results {
        if r.get("error").is_some() { continue; }
        let coin = r["coin"].as_str().unwrap_or("?");
        let bl_wr = r["baseline"]["wr"].as_f64().unwrap_or(0.0);
        let regs  = r["regimes"].as_array().unwrap();
        let best  = r["best_regime"].as_str().unwrap_or("baseline");

        sum_wr[0] += bl_wr; cnt_wr[0] += 1;

        let row: Vec<(u64, f64, f64, f64)> = regs.iter().map(|rv| {
            (rv["t"].as_u64().unwrap_or(0),
             rv["wr"].as_f64().unwrap_or(0.0),
             rv["pf"].as_f64().unwrap_or(0.0),
             rv["pnl"].as_f64().unwrap_or(0.0))
        }).collect();

        for (ri, &(rt, rwr, _, _)) in row.iter().enumerate() {
            sum_wr[ri+1] += rwr; cnt_wr[ri+1] += 1;
            if rwr > 44.0 && rt >= 10 { n_above_44 += 1; }
            if rt >= 10 { total_regime_cells += 1; }
        }

        println!("  {:6} {:>8.1}  {:>6}  {:>6.1}  {:>6.2}  {:>+6.1}  | {:>6}  {:>6.1}  {:>6.2}  {:>+6.1}  | {:>6}  {:>6.1}  {:>6.2}  {:>+6.1}  | {:>6}  {:>6.1}  {:>6.2}  {:>+6.1}  | {:>14}",
            coin, bl_wr,
            row[0].0, row[0].1, row[0].2, row[0].3,
            row[1].0, row[1].1, row[1].2, row[1].3,
            row[2].0, row[2].1, row[2].2, row[2].3,
            row[3].0, row[3].1, row[3].2, row[3].3,
            best);
    }

    println!();
    print!("  Avg WR%:       {:>6.1}  ", sum_wr[0] / cnt_wr[0].max(1) as f64);
    for i in 1..5 {
        print!("  {:>6.1}                       |", sum_wr[i] / cnt_wr[i].max(1) as f64);
    }
    println!();
    println!();
    println!("  WR > 44% with ≥10 trades: {}/{}", n_above_44, total_regime_cells);

    // Summary
    println!();
    println!("  Portfolio avg WR by regime:");
    let regime_labels = ["Baseline", "BEAR_LOW_VOL", "BEAR_HIGH_VOL", "BULL_LOW_VOL", "BULL_HIGH_VOL"];
    for i in 0..5 {
        if cnt_wr[i] > 0 {
            println!("    {:>14}: {:.2}%", regime_labels[i], sum_wr[i] / cnt_wr[i] as f64);
        }
    }

    // Save JSON
    let out_dir = "archive/RUN25";
    std::fs::create_dir_all(out_dir).ok();
    let out_path = format!("{}/run25_results.json", out_dir);
    let output = serde_json::json!({
        "run": "RUN25",
        "description": "BTC 4-regime framework: BULL/BEAR × HIGH/LOW_VOL vs COINCLAW per-coin WR",
        "coins": results,
        "summary": {
            "avg_wr_baseline":      sum_wr[0] / cnt_wr[0].max(1) as f64,
            "avg_wr_bear_low_vol":  sum_wr[1] / cnt_wr[1].max(1) as f64,
            "avg_wr_bear_high_vol": sum_wr[2] / cnt_wr[2].max(1) as f64,
            "avg_wr_bull_low_vol":  sum_wr[3] / cnt_wr[3].max(1) as f64,
            "avg_wr_bull_high_vol": sum_wr[4] / cnt_wr[4].max(1) as f64,
            "wr_above_44_of_total": format!("{}/{}", n_above_44, total_regime_cells),
        }
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("\n  Results saved to {}", out_path);
}
