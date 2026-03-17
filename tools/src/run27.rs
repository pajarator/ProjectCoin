/// RUN27 — Breakout Momentum Rider ("Hard Rally / Hard Crash")
///
/// Every prior RUN (RUN14–RUN26) tested variants of mean-reversion
/// (buy oversold, exit at mean) and found OOS WR ≤ 40% on the test half.
/// This RUN tests the opposite: ride the hard directional move.
///
/// HARD UP entry (long):
///   1. 16-bar compounded return > threshold  (≈ 4h candle hard up)
///   2. volume > rolling_mean(vol,20) × vol_mult  (volume spike)
///   3. ADX(14) > adx_thresh AND ADX[i] > ADX[i-3]  (trend strong + rising)
///   4. close > SMA(50)  (uptrend confirmed)
///   5. 50 ≤ RSI(14) ≤ 75  (momentum, not overbought)
///
/// HARD DOWN entry (short):
///   Mirror: 16-bar return < −threshold, close < SMA50, 25 ≤ RSI ≤ 50
///   (shorts simulated as long on inverted price)
///
/// Exit:
///   RSI > 78 (exhaustion) OR close < SMA(20) (trend break)
///
/// Stop: ATR(14) × atr_mult from entry (wider than 0.3% fixed — momentum
///       trades need room before continuation fires)
/// Trailing: optional — trail at peak − ATR × trail_mult after +act% profit
///
/// Grid (324 combos per coin):
///   move_thresh : [1.0%, 1.5%, 2.0%, 2.5%]
///   vol_mult    : [1.5, 2.0, 2.5]
///   adx_thresh  : [20, 25, 30]
///   atr_mult    : [0.75, 1.0, 1.5]
///   trail config: [none, trail=0.75×ATR@0.5%, trail=1.0×ATR@0.8%]
///
/// Split: 67% train / 33% OOS test  (consistent with all prior RUNs)
/// Fitness on train: Sharpe × sqrt(trades)
/// Fees: 0.1%/side, slip: 0.05%/side

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::{atr, ema, rsi, rolling_mean, sma};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};

const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;

// Grid
const MOVE_THRESHOLDS: [f64; 4]  = [0.010, 0.015, 0.020, 0.025];
const VOL_MULTS:       [f64; 3]  = [1.5, 2.0, 2.5];
const ADX_THRESHOLDS:  [f64; 3]  = [20.0, 25.0, 30.0];
const ATR_MULTS:       [f64; 3]  = [0.75, 1.0, 1.5];
// (trail_atr_mult, activation_pct): 0.0 trail = no trailing
const TRAIL_CONFIGS: [(f64, f64); 3] = [
    (0.0,  0.000),   // no trailing
    (0.75, 0.005),   // trail at peak − 0.75×ATR after +0.5% profit
    (1.0,  0.008),   // trail at peak − 1.0×ATR after +0.8% profit
];

// ── ADX(14) via Wilder's RMA ─────────────────────────────────────────────────
fn adx14(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let alpha = 1.0 / 14.0;
    let mut adx = vec![f64::NAN; n];
    let mut pdm_s = 0.0f64; let mut mdm_s = 0.0f64;
    let mut tr_s  = 0.0f64; let mut adx_s = 0.0f64;
    let mut adx_init = false;
    for i in 1..n {
        let up   = high[i] - high[i-1];
        let down = low[i-1]  - low[i];
        let pdm = if up > down && up > 0.0 { up } else { 0.0 };
        let mdm = if down > up && down > 0.0 { down } else { 0.0 };
        let tr  = (high[i] - low[i])
            .max((high[i] - close[i-1]).abs())
            .max((low[i]  - close[i-1]).abs());
        if i == 1 { pdm_s = pdm; mdm_s = mdm; tr_s = tr; }
        else {
            pdm_s += alpha * (pdm - pdm_s);
            mdm_s += alpha * (mdm - mdm_s);
            tr_s  += alpha * (tr  - tr_s);
        }
        if tr_s > 0.0 {
            let pdi = 100.0 * pdm_s / tr_s;
            let mdi = 100.0 * mdm_s / tr_s;
            let dx = if pdi + mdi > 0.0 { 100.0 * (pdi - mdi).abs() / (pdi + mdi) } else { 0.0 };
            if !adx_init { adx_s = dx; adx_init = true; }
            else { adx_s += alpha * (dx - adx_s); }
            if i >= 14 { adx[i] = adx_s; }
        }
    }
    adx
}

// ── Signal generators ────────────────────────────────────────────────────────

/// 16-bar compounded return at bar i: close[i]/close[i-16] − 1
fn bar16_return(close: &[f64]) -> Vec<f64> {
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    for i in 16..n {
        if close[i-16] > 0.0 { out[i] = close[i] / close[i-16] - 1.0; }
    }
    out
}

struct Params {
    move_thresh: f64,
    vol_mult:    f64,
    adx_thresh:  f64,
    atr_mult:    f64,
    trail_atr:   f64,
    trail_act:   f64,
}

fn signals_long(
    close: &[f64], high: &[f64], low: &[f64], vol: &[f64],
    ret16: &[f64], vol_ma: &[f64], adx: &[f64], rsi14: &[f64],
    sma20: &[f64], sma50: &[f64],
    p: &Params,
) -> (Vec<bool>, Vec<bool>) {
    let n = close.len();
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 50..n {
        if ret16[i].is_nan() || vol_ma[i].is_nan() || adx[i].is_nan()
            || rsi14[i].is_nan() || sma20[i].is_nan() || sma50[i].is_nan()
        { continue; }
        // Entry: hard up breakout
        let adx_rising = i >= 3 && !adx[i-3].is_nan() && adx[i] > adx[i-3];
        entry[i] = ret16[i] >= p.move_thresh
            && vol[i] >= vol_ma[i] * p.vol_mult
            && adx[i] >= p.adx_thresh
            && adx_rising
            && close[i] > sma50[i]
            && rsi14[i] >= 50.0 && rsi14[i] <= 75.0;
        // Exit: exhaustion or trend break
        exit[i] = rsi14[i] > 78.0 || close[i] < sma20[i];
    }
    (entry, exit)
}

fn signals_short(
    close: &[f64], high: &[f64], low: &[f64], vol: &[f64],
    ret16: &[f64], vol_ma: &[f64], adx: &[f64], rsi14: &[f64],
    sma20: &[f64], sma50: &[f64],
    p: &Params,
) -> (Vec<bool>, Vec<bool>) {
    let n = close.len();
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 50..n {
        if ret16[i].is_nan() || vol_ma[i].is_nan() || adx[i].is_nan()
            || rsi14[i].is_nan() || sma20[i].is_nan() || sma50[i].is_nan()
        { continue; }
        let adx_rising = i >= 3 && !adx[i-3].is_nan() && adx[i] > adx[i-3];
        entry[i] = ret16[i] <= -p.move_thresh
            && vol[i] >= vol_ma[i] * p.vol_mult
            && adx[i] >= p.adx_thresh
            && adx_rising
            && close[i] < sma50[i]
            && rsi14[i] >= 25.0 && rsi14[i] <= 50.0;
        exit[i] = rsi14[i] < 22.0 || close[i] > sma20[i];
    }
    (entry, exit)
}

// ── Trade simulation with ATR stop + optional trailing ────────────────────────
/// Returns (n_trades, wr%, pf, total_pnl%, avg_win%, avg_loss%, sharpe_proxy)
fn sim(
    close: &[f64],
    atr_vals: &[f64],
    entry: &[bool],
    exit: &[bool],
    atr_mult: f64,
    trail_atr: f64,
    trail_act: f64,
    is_short: bool,
) -> (usize, f64, f64, f64, f64, f64, f64) {
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;
    let mut sl_price = 0.0f64;
    let mut peak = 0.0f64;   // best-in-direction price
    let mut trail_active = false;
    let mut trail_stop = 0.0f64;

    let mut pnls: Vec<f64> = Vec::new();

    for i in 0..n {
        if in_pos {
            // Update peak price in direction of trade
            if !is_short { if close[i] > peak { peak = close[i]; } }
            else         { if close[i] < peak { peak = close[i]; } }

            // Activate trailing
            if trail_atr > 0.0 && !trail_active {
                let profit = if !is_short { (close[i] - ep) / ep } else { (ep - close[i]) / ep };
                if profit >= trail_act {
                    trail_active = true;
                }
            }
            if trail_active && atr_vals[i].is_finite() {
                trail_stop = if !is_short {
                    peak - atr_vals[i] * trail_atr
                } else {
                    peak + atr_vals[i] * trail_atr
                };
            }

            // Check stop: ATR-based (fixed at entry) or trailing (whichever is tighter / better)
            let effective_stop = if trail_active {
                if !is_short { sl_price.max(trail_stop) } else { sl_price.min(trail_stop) }
            } else {
                sl_price
            };

            let stopped = if !is_short { close[i] <= effective_stop }
                          else          { close[i] >= effective_stop };
            let exited  = exit[i];

            let closed_pnl = if stopped {
                let exit_px = effective_stop * if !is_short { 1.0 - SLIP } else { 1.0 + SLIP };
                Some(if !is_short { (exit_px - ep) / ep * 100.0 - FEE * 200.0 }
                     else         { (ep - exit_px) / ep * 100.0 - FEE * 200.0 })
            } else if exited {
                let exit_px = close[i] * if !is_short { 1.0 - SLIP } else { 1.0 + SLIP };
                Some(if !is_short { (exit_px - ep) / ep * 100.0 - FEE * 200.0 }
                     else         { (ep - exit_px) / ep * 100.0 - FEE * 200.0 })
            } else {
                None
            };

            if let Some(pnl) = closed_pnl {
                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                pnls.push(pnl);
                n_trades += 1;
                in_pos = false;
                trail_active = false;
            }
        } else if entry[i] && atr_vals[i].is_finite() && atr_vals[i] > 0.0 {
            ep = close[i] * if !is_short { 1.0 + SLIP } else { 1.0 - SLIP };
            sl_price = if !is_short { ep - atr_vals[i] * atr_mult }
                       else         { ep + atr_vals[i] * atr_mult };
            // Safety floor: stop no tighter than 0.1%
            if !is_short { sl_price = sl_price.min(ep * 0.999); }
            else         { sl_price = sl_price.max(ep * 1.001); }
            peak = ep;
            trail_active = false;
            in_pos = true;
        }
    }
    // Close open position at last bar
    if in_pos {
        let exit_px = close[n-1] * if !is_short { 1.0 - SLIP } else { 1.0 + SLIP };
        let pnl = if !is_short { (exit_px - ep) / ep * 100.0 - FEE * 200.0 }
                  else         { (ep - exit_px) / ep * 100.0 - FEE * 200.0 };
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        pnls.push(pnl);
        n_trades += 1;
    }

    if n_trades == 0 { return (0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0); }
    let wr   = wins as f64 / n_trades as f64 * 100.0;
    let pf   = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win + 1.0 };
    let avgw = if wins > 0 { gross_win / wins as f64 } else { 0.0 };
    let avgl = if n_trades > wins { gross_loss / (n_trades - wins) as f64 } else { 0.0 };
    // Sharpe proxy: mean / std of per-trade pnl
    let mean = total_pnl / n_trades as f64;
    let var  = pnls.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / n_trades as f64;
    let sharpe = if var > 0.0 { mean / var.sqrt() } else { 0.0 };
    (n_trades, wr, pf, total_pnl, avgw, avgl, sharpe)
}

// ── Per-coin processing ───────────────────────────────────────────────────────
fn process_coin(coin: &str) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let split = n * 67 / 100;

    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

    // Pre-compute indicators (whole series; we slice for train/test)
    let ret16  = bar16_return(&close);
    let vol_ma = rolling_mean(&vol, 20);
    let adx    = adx14(&high, &low, &close);
    let rsi14  = rsi(&close, 14);
    let sma20  = sma(&close, 20);
    let sma50  = sma(&close, 50);
    let atr14  = atr(&high, &low, &close, 14);

    // ── Grid search on TRAIN half ─────────────────────────────────────────────
    struct Best { score: f64, params: Params, is_short: bool, wr: f64, pf: f64 }

    let mut best_long:  Option<Best> = None;
    let mut best_short: Option<Best> = None;

    let tc = &close[..split]; let th = &high[..split]; let tl = &low[..split];
    let tv = &vol[..split];
    let tr16 = &ret16[..split]; let tvm = &vol_ma[..split];
    let tadx = &adx[..split];  let trsi = &rsi14[..split];
    let ts20 = &sma20[..split]; let ts50 = &sma50[..split];
    let tatr = &atr14[..split];

    for &move_thresh in &MOVE_THRESHOLDS {
        for &vol_mult in &VOL_MULTS {
            for &adx_thresh in &ADX_THRESHOLDS {
                for &atr_mult in &ATR_MULTS {
                    for &(trail_atr, trail_act) in &TRAIL_CONFIGS {
                        let p = Params { move_thresh, vol_mult, adx_thresh,
                                         atr_mult, trail_atr, trail_act };

                        // Long
                        let (el, xl) = signals_long(tc, th, tl, tv, tr16, tvm,
                                                     tadx, trsi, ts20, ts50, &p);
                        let (ntl, wrl, pfl, _, _, _, shl) =
                            sim(tc, tatr, &el, &xl, atr_mult, trail_atr, trail_act, false);
                        if ntl >= 5 {
                            let score = shl * (ntl as f64).sqrt();
                            if best_long.as_ref().map_or(true, |b| score > b.score) {
                                best_long = Some(Best {
                                    score,
                                    params: Params { move_thresh, vol_mult, adx_thresh,
                                                     atr_mult, trail_atr, trail_act },
                                    is_short: false, wr: wrl, pf: pfl,
                                });
                            }
                        }

                        // Short
                        let (es, xs) = signals_short(tc, th, tl, tv, tr16, tvm,
                                                      tadx, trsi, ts20, ts50, &p);
                        let (nts, wrs, pfs, _, _, _, shs) =
                            sim(tc, tatr, &es, &xs, atr_mult, trail_atr, trail_act, true);
                        if nts >= 5 {
                            let score = shs * (nts as f64).sqrt();
                            if best_short.as_ref().map_or(true, |b| score > b.score) {
                                best_short = Some(Best {
                                    score,
                                    params: Params { move_thresh, vol_mult, adx_thresh,
                                                     atr_mult, trail_atr, trail_act },
                                    is_short: true, wr: wrs, pf: pfs,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // ── OOS evaluation ────────────────────────────────────────────────────────
    let oc = &close[split..]; let oh = &high[split..]; let ol = &low[split..];
    let ov = &vol[split..];
    let or16 = &ret16[split..]; let ovm = &vol_ma[split..];
    let oadx = &adx[split..];   let orsi = &rsi14[split..];
    let os20 = &sma20[split..]; let os50 = &sma50[split..];
    let oatr = &atr14[split..];

    let eval_dir = |best: &Option<Best>, is_short: bool| -> Value {
        let b = match best { Some(b) => b, None => return json!({"t":0,"error":"no train signal"}) };
        let p = &b.params;
        let (entry, exit) = if !is_short {
            signals_long(oc, oh, ol, ov, or16, ovm, oadx, orsi, os20, os50, p)
        } else {
            signals_short(oc, oh, ol, ov, or16, ovm, oadx, orsi, os20, os50, p)
        };
        let (nt, wr, pf, pnl, avgw, avgl, _) =
            sim(oc, oatr, &entry, &exit, p.atr_mult, p.trail_atr, p.trail_act, is_short);
        let bewr = if avgw + avgl > 0.0 { avgl / (avgw + avgl) * 100.0 } else { 50.0 };
        json!({
            "t": nt, "wr": wr, "pf": pf, "pnl": pnl,
            "avg_win": avgw, "avg_loss": avgl, "breakeven_wr": bewr,
            "config": {
                "move_thresh_pct": p.move_thresh * 100.0,
                "vol_mult": p.vol_mult,
                "adx_thresh": p.adx_thresh,
                "atr_mult": p.atr_mult,
                "trail_atr": p.trail_atr,
                "trail_act_pct": p.trail_act * 100.0,
            },
            "train_wr": b.wr, "train_pf": b.pf,
        })
    };

    let long_oos  = eval_dir(&best_long,  false);
    let short_oos = eval_dir(&best_short, true);

    // Combined: entry fires if either long or short fires (sequential)
    let combined_t   = long_oos["t"].as_u64().unwrap_or(0) + short_oos["t"].as_u64().unwrap_or(0);
    let combined_pnl = long_oos["pnl"].as_f64().unwrap_or(0.0) + short_oos["pnl"].as_f64().unwrap_or(0.0);

    let positive_long  = long_oos["wr"].as_f64().unwrap_or(0.0) >= 44.0
                       && long_oos["t"].as_u64().unwrap_or(0) >= 20;
    let positive_short = short_oos["wr"].as_f64().unwrap_or(0.0) >= 44.0
                       && short_oos["t"].as_u64().unwrap_or(0) >= 20;
    let positive_rr_long  = long_oos["avg_win"].as_f64().unwrap_or(0.0) /
                             long_oos["avg_loss"].as_f64().unwrap_or(1.0) >= 1.5
                            && long_oos["pf"].as_f64().unwrap_or(0.0) >= 1.2
                            && long_oos["t"].as_u64().unwrap_or(0) >= 20;

    json!({
        "coin": coin,
        "long":  long_oos,
        "short": short_oos,
        "combined_t": combined_t,
        "combined_pnl": combined_pnl,
        "positive_long": positive_long || positive_rr_long,
        "positive_short": positive_short,
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN27 — Breakout Momentum Rider (Hard Rally / Hard Crash)");
    println!("================================================================");
    println!("Hypothesis: ride hard directional moves instead of fading them.");
    println!("Entry: 16-bar return > thresh + vol spike + ADX rising + SMA50 filter");
    println!("Exit: RSI exhaustion (>78/<22) OR SMA20 cross");
    println!("Stop: ATR(14) × mult | Trail: optional ATR-based after activation");
    println!("Grid: 4×3×3×3×3 = 324 combos | train 67% | OOS 33%");
    println!("Success: WR≥44% ≥20t  OR  avg_win/avg_loss≥1.5 + PF≥1.2 ≥20t");
    println!();

    let coins: Vec<&str> = COIN_STRATEGIES.iter().map(|(c, _)| *c).collect();

    let results: Vec<Value> = coins.par_iter()
        .map(|&coin| process_coin(coin))
        .collect();

    // ── Print results ─────────────────────────────────────────────────────────
    println!("  {:6}  {:>4}  {:>5}  {:>5}  {:>6}  {:>5}  {:>5}  {:>5}  |  {:>4}  {:>5}  {:>5}  {:>6}  {:>5}  {:>5}  {:>5}  |  {:>4}",
        "Coin",
        "Lt",  "LWR%", "LPF", "LP&L%", "LBEw", "LAW", "LAL",
        "St",  "SWR%", "SPF", "SP&L%", "SBEw", "SAW", "SAL",
        "+?");
    println!("  {}  {}  {}",
        "-".repeat(50), "-".repeat(51), "-".repeat(4));

    let mut n_positive_long  = 0usize;
    let mut n_positive_short = 0usize;
    let mut sum_l_wr = 0.0f64; let mut sum_l_pf = 0.0f64;
    let mut sum_s_wr = 0.0f64; let mut sum_s_pf = 0.0f64;
    let mut n_valid = 0usize;

    for r in &results {
        let coin = r["coin"].as_str().unwrap_or("?");
        let l = &r["long"]; let s = &r["short"];

        let lt   = l["t"].as_u64().unwrap_or(0);
        let lwr  = l["wr"].as_f64().unwrap_or(0.0);
        let lpf  = l["pf"].as_f64().unwrap_or(0.0);
        let lpnl = l["pnl"].as_f64().unwrap_or(0.0);
        let lbew = l["breakeven_wr"].as_f64().unwrap_or(50.0);
        let law  = l["avg_win"].as_f64().unwrap_or(0.0);
        let lal  = l["avg_loss"].as_f64().unwrap_or(0.0);

        let st   = s["t"].as_u64().unwrap_or(0);
        let swr  = s["wr"].as_f64().unwrap_or(0.0);
        let spf  = s["pf"].as_f64().unwrap_or(0.0);
        let spnl = s["pnl"].as_f64().unwrap_or(0.0);
        let sbew = s["breakeven_wr"].as_f64().unwrap_or(50.0);
        let saw  = s["avg_win"].as_f64().unwrap_or(0.0);
        let sal  = s["avg_loss"].as_f64().unwrap_or(0.0);

        let pos_l = r["positive_long"].as_bool().unwrap_or(false);
        let pos_s = r["positive_short"].as_bool().unwrap_or(false);
        let flag  = if pos_l || pos_s { "YES" } else { "no" };

        println!("  {:6}  {:>4}  {:>5.1}  {:>5.2}  {:>+6.1}  {:>5.1}  {:>5.3}  {:>5.3}  |  {:>4}  {:>5.1}  {:>5.2}  {:>+6.1}  {:>5.1}  {:>5.3}  {:>5.3}  |  {:>4}",
            coin,
            lt, lwr, lpf, lpnl, lbew, law, lal,
            st, swr, spf, spnl, sbew, saw, sal,
            flag);

        if lt >= 5 || st >= 5 { n_valid += 1; }
        if pos_l { n_positive_long  += 1; }
        if pos_s { n_positive_short += 1; }
        if lt > 0 { sum_l_wr += lwr; sum_l_pf += lpf; }
        if st > 0 { sum_s_wr += swr; sum_s_pf += spf; }
    }

    let n = coins.len() as f64;
    println!();
    println!("  Avg LONG:  WR={:.1}%  PF={:.3}", sum_l_wr/n, sum_l_pf/n);
    println!("  Avg SHORT: WR={:.1}%  PF={:.3}", sum_s_wr/n, sum_s_pf/n);
    println!("  POSITIVE — long: {}/{}  short: {}/{}", n_positive_long, 18, n_positive_short, 18);

    // Config summary for positives
    println!();
    let total_pos = n_positive_long + n_positive_short;
    if total_pos > 0 {
        println!("  POSITIVE coin details:");
        for r in &results {
            let pos_l = r["positive_long"].as_bool().unwrap_or(false);
            let pos_s = r["positive_short"].as_bool().unwrap_or(false);
            if !pos_l && !pos_s { continue; }
            let coin = r["coin"].as_str().unwrap_or("?");
            if pos_l {
                let l = &r["long"];
                let c = &l["config"];
                println!("    {} LONG  WR={:.1}% PF={:.2} t={} | move={:.1}% vol={:.1}x ADX≥{} ATR×{:.2} trail={}×ATR@{:.1}%",
                    coin,
                    l["wr"].as_f64().unwrap_or(0.0),
                    l["pf"].as_f64().unwrap_or(0.0),
                    l["t"].as_u64().unwrap_or(0),
                    c["move_thresh_pct"].as_f64().unwrap_or(0.0),
                    c["vol_mult"].as_f64().unwrap_or(0.0),
                    c["adx_thresh"].as_f64().unwrap_or(0.0),
                    c["atr_mult"].as_f64().unwrap_or(0.0),
                    c["trail_atr"].as_f64().unwrap_or(0.0),
                    c["trail_act_pct"].as_f64().unwrap_or(0.0));
            }
            if pos_s {
                let s = &r["short"];
                let c = &s["config"];
                println!("    {} SHORT WR={:.1}% PF={:.2} t={} | move={:.1}% vol={:.1}x ADX≥{} ATR×{:.2} trail={}×ATR@{:.1}%",
                    coin,
                    s["wr"].as_f64().unwrap_or(0.0),
                    s["pf"].as_f64().unwrap_or(0.0),
                    s["t"].as_u64().unwrap_or(0),
                    c["move_thresh_pct"].as_f64().unwrap_or(0.0),
                    c["vol_mult"].as_f64().unwrap_or(0.0),
                    c["adx_thresh"].as_f64().unwrap_or(0.0),
                    c["atr_mult"].as_f64().unwrap_or(0.0),
                    c["trail_atr"].as_f64().unwrap_or(0.0),
                    c["trail_act_pct"].as_f64().unwrap_or(0.0));
            }
        }
    } else {
        println!("  No coins met the success criteria (WR≥44%+20t or RR≥1.5+PF≥1.2+20t).");
    }

    // Save
    let out_dir = "archive/RUN27";
    std::fs::create_dir_all(out_dir).ok();
    let out_path = format!("{}/run27_results.json", out_dir);
    let output = serde_json::json!({
        "run": "RUN27",
        "description": "Breakout momentum rider — hard rally/crash signal",
        "coins": results,
        "summary": {
            "avg_long_wr":  sum_l_wr / n,
            "avg_long_pf":  sum_l_pf / n,
            "avg_short_wr": sum_s_wr / n,
            "avg_short_pf": sum_s_pf / n,
            "positive_long":  n_positive_long,
            "positive_short": n_positive_short,
        }
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("\n  Results saved to {}", out_path);
}
