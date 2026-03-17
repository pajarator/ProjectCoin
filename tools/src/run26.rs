/// RUN26 — ATR-Based Dynamic Stops Grid Search
///
/// Tests ATR-based stop losses and trailing stops against the current
/// fixed 0.3% SL. ATR stops adapt to current volatility; trailing stops
/// let winning trades run further — both can lower the effective breakeven WR.
///
/// Fix over Python stub:
///   Python used full-data "best strategy" selection (look-ahead bias)
///   and evaluated on full data. Corrected: COINCLAW v13 per-coin primary
///   strategy; grid search on train (67%); winner vs baseline on OOS (33%).
///
/// Grid:
///   ATR mult    : [0.5, 0.75, 1.0, 1.5, 2.0]
///   ATR period  : [7, 14, 21]
///   Trail pct   : [none, 0.3%, 0.5%, 0.75%, 1.0%]
///   Trail activation: [none/immediate, 0.3%, 0.5%, 0.8%]
///   → 255 combos per coin
///
/// ATR stop: stop_price = entry_price − ATR[entry_bar] × atr_mult
///   (ATR in price units; larger mult = wider, more forgiving stop)
/// Trailing stop: once price rises ≥ activation above entry, trail
///   at peak_price × (1 − trail_pct). Acts as floor that ratchets up.
///
/// Scoring on train: sharpe × sqrt(trades) (rank by risk-adjusted output)
/// Final comparison: train-best config vs fixed 0.3% baseline on OOS test.
/// Fee: 0.1%/side, slip: 0.05%/side.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::atr;
use crate::loader::{load_ohlcv, Bar, COIN_STRATEGIES};
use crate::strategies::signals;

const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;
const FIXED_SL: f64 = 0.003;

// Grid
const ATR_MULTS:  [f64; 5]   = [0.5, 0.75, 1.0, 1.5, 2.0];
const ATR_PERODS: [usize; 3] = [7, 14, 21];
// (trail_pct, trail_activation): 0.0 in trail_pct = no trailing
const TRAIL_CONFIGS: [(f64, f64); 17] = [
    (0.000, 0.000),  // no trailing (1 config × 15 ATR = 15)
    (0.003, 0.000), (0.003, 0.003), (0.003, 0.005), (0.003, 0.008),
    (0.005, 0.000), (0.005, 0.003), (0.005, 0.005), (0.005, 0.008),
    (0.0075,0.000), (0.0075,0.003), (0.0075,0.005), (0.0075,0.008),
    (0.010, 0.000), (0.010, 0.003), (0.010, 0.005), (0.010, 0.008),
];
// Total: 17 × 15 = 255 combos per coin

// ── Simulation with ATR stop + optional trailing ───────────────────────────
/// Returns (n_trades, wr%, pf, total_pnl%, avg_win%, avg_loss%, max_dd%)
fn sim_atr(
    close: &[f64],
    atr_vals: &[f64],
    entry: &[bool],
    exit: &[bool],
    atr_mult: f64,
    trail_pct: f64,      // 0.0 = no trailing
    trail_act: f64,      // activation threshold above entry (e.g. 0.005 = 0.5%)
) -> (usize, f64, f64, f64, f64, f64, f64) {
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;
    let mut atr_sl = 0.0f64;    // absolute stop price
    let mut peak = 0.0f64;
    let mut trail_active = false;
    let mut trail_stop = 0.0f64;

    // For drawdown
    let mut equity = 0.0f64;
    let mut eq_peak = 0.0f64;
    let mut max_dd = 0.0f64;

    for i in 0..n {
        if in_pos {
            // Update peak for trailing
            if close[i] > peak { peak = close[i]; }

            // Activate trailing once profit exceeds activation threshold
            if trail_pct > 0.0 && !trail_active {
                if trail_act == 0.0 || close[i] >= ep * (1.0 + trail_act) {
                    trail_active = true;
                }
            }
            if trail_active {
                trail_stop = peak * (1.0 - trail_pct);
            }

            // Effective stop = max(atr_sl, trail_stop if active)
            let stop = if trail_active { atr_sl.max(trail_stop) } else { atr_sl };

            let closed = if close[i] <= stop {
                // Stop hit: exit at stop price (pessimistic)
                let exit_price = stop * (1.0 - SLIP);
                Some((exit_price - ep) / ep * 100.0 - FEE * 200.0)
            } else if exit[i] {
                let exit_price = close[i] * (1.0 - SLIP);
                Some((exit_price - ep) / ep * 100.0 - FEE * 200.0)
            } else {
                None
            };

            if let Some(pnl) = closed {
                equity += pnl;
                if equity > eq_peak { eq_peak = equity; }
                let dd = eq_peak - equity;
                if dd > max_dd { max_dd = dd; }

                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                n_trades += 1;
                in_pos = false;
                trail_active = false;
                trail_stop = 0.0;
            }
        } else if entry[i] {
            if atr_vals[i].is_nan() || atr_vals[i] <= 0.0 { continue; }
            ep = close[i] * (1.0 + SLIP);
            atr_sl = ep - atr_vals[i] * atr_mult;
            // Safety: ATR stop can't be tighter than 0.1% below entry
            atr_sl = atr_sl.min(ep * (1.0 - 0.001));
            peak = ep;
            trail_active = false;
            trail_stop = 0.0;
            in_pos = true;
        }
    }

    // Close any open position
    if in_pos {
        let pnl = (close[n-1] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0;
        equity += pnl;
        if equity > eq_peak { eq_peak = equity; }
        let dd = eq_peak - equity;
        if dd > max_dd { max_dd = dd; }
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        n_trades += 1;
    }

    if n_trades == 0 { return (0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0); }
    let wr = wins as f64 / n_trades as f64 * 100.0;
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win + 1.0 };
    let avg_win  = if wins > 0 { gross_win / wins as f64 } else { 0.0 };
    let avg_loss = if n_trades > wins { gross_loss / (n_trades - wins) as f64 } else { 0.0 };
    (n_trades, wr, pf, total_pnl, avg_win, avg_loss, max_dd)
}

/// Baseline sim with fixed 0.3% SL (existing COINCLAW behaviour)
fn sim_fixed_sl(close: &[f64], entry: &[bool], exit: &[bool]) -> (usize, f64, f64, f64, f64, f64) {
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;

    let mut equity = 0.0f64;
    let mut eq_peak = 0.0f64;
    let mut max_dd = 0.0f64;

    for i in 0..n {
        if in_pos {
            let ret = (close[i] - ep) / ep;
            let closed = if ret <= -FIXED_SL {
                Some(-FIXED_SL * (1.0 + SLIP) * 100.0 - FEE * 200.0)
            } else if exit[i] {
                Some((close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0)
            } else {
                None
            };
            if let Some(pnl) = closed {
                equity += pnl;
                if equity > eq_peak { eq_peak = equity; }
                let dd = eq_peak - equity;
                if dd > max_dd { max_dd = dd; }
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
        let pnl = (close[n-1] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0;
        equity += pnl;
        if equity > eq_peak { eq_peak = equity; }
        let dd = eq_peak - equity;
        if dd > max_dd { max_dd = dd; }
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        n_trades += 1;
    }
    if n_trades == 0 { return (0, 0.0, 0.0, 0.0, 0.0, 0.0); }
    let wr = wins as f64 / n_trades as f64 * 100.0;
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win + 1.0 };
    let avg_win  = if wins > 0 { gross_win / wins as f64 } else { 0.0 };
    let avg_loss = if n_trades > wins { gross_loss / (n_trades - wins) as f64 } else { 0.0 };
    (n_trades, wr, pf, total_pnl, avg_win, avg_loss)
}

// ── Per-coin processing ───────────────────────────────────────────────────────
fn process_coin(coin: &str, strategy: &str) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let split = n * 67 / 100;
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();

    let (entry_all, exit_all) = signals(&bars, strategy);

    // Pre-compute ATR for all three periods
    let atrs: [Vec<f64>; 3] = [
        atr(&high, &low, &close, ATR_PERODS[0]),
        atr(&high, &low, &close, ATR_PERODS[1]),
        atr(&high, &low, &close, ATR_PERODS[2]),
    ];

    // ── Train: grid search ────────────────────────────────────────────────────
    let train_close = &close[..split];
    let train_entry = &entry_all[..split];
    let train_exit  = &exit_all[..split];

    struct BestConfig {
        atr_mult:   f64,
        atr_period: usize,
        trail_pct:  f64,
        trail_act:  f64,
        score:      f64,
        wr:         f64,
        pf:         f64,
    }

    let mut best: Option<BestConfig> = None;

    for &atr_mult in &ATR_MULTS {
        for (pi, &atr_period) in ATR_PERODS.iter().enumerate() {
            let train_atr = &atrs[pi][..split];
            for &(trail_pct, trail_act) in &TRAIL_CONFIGS {
                let (nt, wr, pf, pnl, avg_win, _avg_loss, max_dd) =
                    sim_atr(train_close, train_atr, train_entry, train_exit,
                            atr_mult, trail_pct, trail_act);
                if nt < 5 { continue; }
                // Score: sharpe-proxy × sqrt(trades)
                // Estimate sharpe as pnl / max_dd (simplified)
                let score = pf * (nt as f64).sqrt() / max_dd.max(1.0);
                if best.as_ref().map_or(true, |b| score > b.score) {
                    best = Some(BestConfig {
                        atr_mult, atr_period, trail_pct, trail_act,
                        score, wr, pf
                    });
                }
            }
        }
    }

    // ── Test: baseline vs best config ────────────────────────────────────────
    let test_close = &close[split..];
    let test_entry = &entry_all[split..];
    let test_exit  = &exit_all[split..];

    let (bl_t, bl_wr, bl_pf, bl_pnl, bl_avgw, bl_avgl) =
        sim_fixed_sl(test_close, test_entry, test_exit);

    let bl_breakeven_wr = if bl_avgw + bl_avgl > 0.0 {
        bl_avgl / (bl_avgw + bl_avgl) * 100.0
    } else { 44.0 };

    let (best_t, best_wr, best_pf, best_pnl, best_avgw, best_avgl, best_dd);
    let (best_mult, best_period, best_tpct, best_tact, train_wr, train_pf);

    if let Some(ref bc) = best {
        let pi = ATR_PERODS.iter().position(|&p| p == bc.atr_period).unwrap();
        let test_atr = &atrs[pi][split..];
        let res = sim_atr(test_close, test_atr, test_entry, test_exit,
                          bc.atr_mult, bc.trail_pct, bc.trail_act);
        best_t    = res.0; best_wr  = res.1; best_pf  = res.2;
        best_pnl  = res.3; best_avgw= res.4; best_avgl= res.5;
        best_dd   = res.6;
        best_mult   = bc.atr_mult;
        best_period = bc.atr_period;
        best_tpct   = bc.trail_pct;
        best_tact   = bc.trail_act;
        train_wr    = bc.wr;
        train_pf    = bc.pf;
    } else {
        return json!({ "coin": coin, "error": "no valid config on train" });
    }

    let best_breakeven_wr = if best_avgw + best_avgl > 0.0 {
        best_avgl / (best_avgw + best_avgl) * 100.0
    } else { 44.0 };

    let pf_delta = best_pf - bl_pf;
    let wr_delta = best_wr - bl_wr;

    json!({
        "coin": coin,
        "strategy": strategy,
        "baseline": {
            "t": bl_t, "wr": bl_wr, "pf": bl_pf, "pnl": bl_pnl,
            "avg_win": bl_avgw, "avg_loss": bl_avgl,
            "breakeven_wr": bl_breakeven_wr,
        },
        "best_config": {
            "atr_mult": best_mult, "atr_period": best_period,
            "trail_pct": best_tpct, "trail_act": best_tact,
            "train_wr": train_wr, "train_pf": train_pf,
        },
        "best_oos": {
            "t": best_t, "wr": best_wr, "pf": best_pf, "pnl": best_pnl,
            "avg_win": best_avgw, "avg_loss": best_avgl,
            "breakeven_wr": best_breakeven_wr,
            "max_dd": best_dd,
        },
        "pf_delta": pf_delta,
        "wr_delta": wr_delta,
        "wr_above_44": best_wr >= 44.0 && best_t >= 10,
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN26 — ATR-Based Dynamic Stops Grid Search");
    println!("================================================================");
    println!("Fix: COINCLAW v13 per-coin strategy; grid on train (67%); OOS eval (33%).");
    println!("Grid: 5 ATR mults × 3 ATR periods × 17 trail configs = 255 combos/coin");
    println!("ATR stop: stop = entry − ATR[entry] × mult");
    println!("Trailing: trail from peak once profit ≥ activation threshold");
    println!("Baseline: fixed SL=0.3% (current COINCLAW)");
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.iter().map(|&(c, s)| (c, s)).collect();

    let results: Vec<Value> = coins.par_iter()
        .map(|&(coin, strat)| process_coin(coin, strat))
        .collect();

    // ── Print table ───────────────────────────────────────────────────────────
    println!("  {:6}  {:>5}  {:>5}  {:>5}  {:>6}  {:>5}  {:>5}  |  {:>5}  {:>5}  {:>5}  {:>6}  {:>5}  {:>5}  |  {:>+6}  {:>+6}  |  {:>22}",
        "Coin",
        "Bt", "BWR%", "BPF", "BP&L%", "BEwr", "AvgW",
        "At", "AWR%", "APF", "AP&L%", "AEwr", "AvgW",
        "dPF", "dWR",
        "BestConfig");
    println!("  {}  {}  {}  {}",
        "-".repeat(48), "-".repeat(43), "-".repeat(15), "-".repeat(22));

    let mut n_above_44 = 0usize;
    let mut n_pf_improved = 0usize;
    let mut n_valid = 0usize;
    let mut sum_bl_wr = 0.0f64;
    let mut sum_best_wr = 0.0f64;
    let mut sum_bl_pf = 0.0f64;
    let mut sum_best_pf = 0.0f64;
    let mut sum_bl_breakeven = 0.0f64;
    let mut sum_best_breakeven = 0.0f64;

    for r in &results {
        if r.get("error").is_some() { continue; }
        let coin = r["coin"].as_str().unwrap_or("?");
        let bl   = &r["baseline"];
        let bo   = &r["best_oos"];
        let bc   = &r["best_config"];

        let bl_t    = bl["t"].as_u64().unwrap_or(0);
        let bl_wr   = bl["wr"].as_f64().unwrap_or(0.0);
        let bl_pf   = bl["pf"].as_f64().unwrap_or(0.0);
        let bl_pnl  = bl["pnl"].as_f64().unwrap_or(0.0);
        let bl_bewr = bl["breakeven_wr"].as_f64().unwrap_or(44.0);
        let bl_avgw = bl["avg_win"].as_f64().unwrap_or(0.0);

        let bo_t    = bo["t"].as_u64().unwrap_or(0);
        let bo_wr   = bo["wr"].as_f64().unwrap_or(0.0);
        let bo_pf   = bo["pf"].as_f64().unwrap_or(0.0);
        let bo_pnl  = bo["pnl"].as_f64().unwrap_or(0.0);
        let bo_bewr = bo["breakeven_wr"].as_f64().unwrap_or(44.0);
        let bo_avgw = bo["avg_win"].as_f64().unwrap_or(0.0);

        let dpf = r["pf_delta"].as_f64().unwrap_or(0.0);
        let dwr = r["wr_delta"].as_f64().unwrap_or(0.0);

        let mult   = bc["atr_mult"].as_f64().unwrap_or(0.0);
        let period = bc["atr_period"].as_u64().unwrap_or(0);
        let tpct   = bc["trail_pct"].as_f64().unwrap_or(0.0);
        let tact   = bc["trail_act"].as_f64().unwrap_or(0.0);

        let config_str = if tpct == 0.0 {
            format!("ATR{:.2}×p{}", mult, period)
        } else {
            format!("ATR{:.2}×p{} trail{:.1}%@{:.1}%", mult, period, tpct*100.0, tact*100.0)
        };

        println!("  {:6}  {:>5}  {:>5.1}  {:>5.2}  {:>+6.1}  {:>5.1}  {:>5.3}  |  {:>5}  {:>5.1}  {:>5.2}  {:>+6.1}  {:>5.1}  {:>5.3}  |  {:>+6.3}  {:>+6.1}  |  {:>22}",
            coin,
            bl_t, bl_wr, bl_pf, bl_pnl, bl_bewr, bl_avgw,
            bo_t, bo_wr, bo_pf, bo_pnl, bo_bewr, bo_avgw,
            dpf, dwr,
            config_str);

        n_valid += 1;
        sum_bl_wr      += bl_wr;   sum_best_wr      += bo_wr;
        sum_bl_pf      += bl_pf;   sum_best_pf      += bo_pf;
        sum_bl_breakeven += bl_bewr; sum_best_breakeven += bo_bewr;

        if bo_wr >= 44.0 && bo_t >= 10 { n_above_44 += 1; }
        if dpf > 0.0 { n_pf_improved += 1; }
    }

    println!();
    if n_valid > 0 {
        let n = n_valid as f64;
        println!("  Avg baseline:  WR={:.1}%  PF={:.3}  BreakevenWR={:.1}%",
            sum_bl_wr/n, sum_bl_pf/n, sum_bl_breakeven/n);
        println!("  Avg best OOS:  WR={:.1}%  PF={:.3}  BreakevenWR={:.1}%",
            sum_best_wr/n, sum_best_pf/n, sum_best_breakeven/n);
        println!("  PF improved: {}/{}   WR > 44% with ≥10t: {}/{}",
            n_pf_improved, n_valid, n_above_44, n_valid);
    }

    // Find best breakeven_wr overall (key metric — lower = easier to profit)
    println!();
    println!("  Breakeven WR comparison (lower = easier to profit):");
    println!("  {:6}  {:>12}  {:>12}  {:>12}",
        "Coin", "Base BEwr%", "Best BEwr%", "Delta");
    for r in &results {
        if r.get("error").is_some() { continue; }
        let coin  = r["coin"].as_str().unwrap_or("?");
        let bl_be = r["baseline"]["breakeven_wr"].as_f64().unwrap_or(44.0);
        let bo_be = r["best_oos"]["breakeven_wr"].as_f64().unwrap_or(44.0);
        println!("  {:6}  {:>12.1}  {:>12.1}  {:>+12.1}", coin, bl_be, bo_be, bo_be - bl_be);
    }

    // Save
    let out_dir = "archive/RUN26";
    std::fs::create_dir_all(out_dir).ok();
    let out_path = format!("{}/run26_results.json", out_dir);
    let output = serde_json::json!({
        "run": "RUN26",
        "description": "ATR-based dynamic stops vs fixed 0.3% SL",
        "coins": results,
        "summary": {
            "n_valid": n_valid,
            "avg_baseline_wr":  sum_bl_wr   / n_valid.max(1) as f64,
            "avg_best_wr":      sum_best_wr / n_valid.max(1) as f64,
            "avg_baseline_pf":  sum_bl_pf   / n_valid.max(1) as f64,
            "avg_best_pf":      sum_best_pf / n_valid.max(1) as f64,
            "avg_baseline_breakeven_wr": sum_bl_breakeven  / n_valid.max(1) as f64,
            "avg_best_breakeven_wr":     sum_best_breakeven/ n_valid.max(1) as f64,
            "pf_improved": n_pf_improved,
            "wr_above_44": n_above_44,
        }
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("\n  Results saved to {}", out_path);
}
