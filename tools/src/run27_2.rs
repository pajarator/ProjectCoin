/// RUN27.2 — Walk-Forward Validation of Momentum Breakout
///
/// Validates that the positive results from RUN27.1 (NEAR long/short,
/// DASH long, XLM long) are not artefacts of the specific 67/33 split.
///
/// Method: 3 expanding windows, each with independent grid-search on
/// the train portion and evaluation on the corresponding test portion.
/// Window splits (approximate bar fractions):
///   W1: train[0..33%]  → test[33%..50%]   (≈4mo train, ≈2mo test)
///   W2: train[0..50%]  → test[50%..75%]   (≈6mo train, ≈3mo test)
///   W3: train[0..67%]  → test[67%..100%]  (≈8mo train, ≈4mo test, = RUN27.1)
///
/// For each window: best per-coin config + universal config are both tested.
/// Universal params (most-selected across RUN27.1 positives):
///   move=2.0%, vol=2.0×, ADX≥20, ATR×0.75, trail=1.0×ATR@0.8%
///
/// Degradation metric: IS WR → OOS WR across windows (should stay ≥40%).
/// Focus summary on RUN27.1 positive coins: NEAR, DASH, XLM.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::{atr, rsi, rolling_mean, sma};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::run27::{
    adx14, bar16_return, signals_long, signals_short, sim, Params,
    MOVE_THRESHOLDS, VOL_MULTS, ADX_THRESHOLDS, ATR_MULTS, TRAIL_CONFIGS,
};

// Universal params derived from RUN27.1 most-common positive config
const UNIV: Params = Params {
    move_thresh: 0.020,
    vol_mult:    2.0,
    adx_thresh:  20.0,
    atr_mult:    0.75,
    trail_atr:   1.0,
    trail_act:   0.008,
};

const WINDOWS: [(f64, f64, f64); 3] = [
    (0.0, 0.33, 0.50),  // W1
    (0.0, 0.50, 0.75),  // W2
    (0.0, 0.67, 1.00),  // W3 = RUN27.1
];

// ── Per-coin walk-forward ─────────────────────────────────────────────────────
fn process_coin(coin: &str) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

    let ret16  = bar16_return(&close);
    let vol_ma = rolling_mean(&vol, 20);
    let adx    = adx14(&high, &low, &close);
    let rsi14  = rsi(&close, 14);
    let sma20  = sma(&close, 20);
    let sma50  = sma(&close, 50);
    let atr14  = atr(&high, &low, &close, 14);

    let mut window_results = Vec::new();

    for (wi, &(_, train_end_frac, test_end_frac)) in WINDOWS.iter().enumerate() {
        let train_end = (n as f64 * train_end_frac) as usize;
        let test_end  = (n as f64 * test_end_frac)  as usize;

        // Slices
        let sl = |v: &Vec<f64>, a, b| v[a..b].to_vec();
        let tc = sl(&close, 0, train_end); let th = sl(&high, 0, train_end);
        let tl = sl(&low,   0, train_end); let tv = sl(&vol,  0, train_end);
        let tr16 = sl(&ret16, 0, train_end); let tvm = sl(&vol_ma, 0, train_end);
        let tadx = sl(&adx,  0, train_end); let trsi = sl(&rsi14,  0, train_end);
        let ts20 = sl(&sma20, 0, train_end); let ts50 = sl(&sma50, 0, train_end);
        let tatr = sl(&atr14, 0, train_end);

        let oc = sl(&close, train_end, test_end); let oh = sl(&high, train_end, test_end);
        let ol = sl(&low, train_end, test_end);   let ov = sl(&vol, train_end, test_end);
        let or16 = sl(&ret16, train_end, test_end); let ovm = sl(&vol_ma, train_end, test_end);
        let oadx = sl(&adx, train_end, test_end);   let orsi = sl(&rsi14, train_end, test_end);
        let os20 = sl(&sma20, train_end, test_end); let os50 = sl(&sma50, train_end, test_end);
        let oatr = sl(&atr14, train_end, test_end);

        // Find best long + short on train
        let eval_dir = |is_short: bool| -> Value {
            // Best-config evaluation
            let best_cfg = {
                let mut best_p = Params { move_thresh: MOVE_THRESHOLDS[2], vol_mult: VOL_MULTS[1],
                    adx_thresh: ADX_THRESHOLDS[0], atr_mult: ATR_MULTS[0],
                    trail_atr: TRAIL_CONFIGS[2].0, trail_act: TRAIL_CONFIGS[2].1 };
                let mut best_score = f64::NEG_INFINITY;
                let mut best_is_wr = 0.0; let mut best_is_t = 0usize;

                for &move_thresh in &MOVE_THRESHOLDS {
                    for &vol_mult in &VOL_MULTS {
                        for &adx_thresh in &ADX_THRESHOLDS {
                            for &atr_mult in &ATR_MULTS {
                                for &(trail_atr, trail_act) in &TRAIL_CONFIGS {
                                    let p = Params { move_thresh, vol_mult, adx_thresh,
                                                     atr_mult, trail_atr, trail_act };
                                    let (e, x) = if !is_short {
                                        signals_long(&tc,&th,&tl,&tv,&tr16,&tvm,&tadx,&trsi,&ts20,&ts50,&p)
                                    } else {
                                        signals_short(&tc,&th,&tl,&tv,&tr16,&tvm,&tadx,&trsi,&ts20,&ts50,&p)
                                    };
                                    let (nt,wr,_,_,_,_,sh) = sim(&tc,&tatr,&e,&x,atr_mult,trail_atr,trail_act,is_short);
                                    if nt < 5 { continue; }
                                    let score = sh * (nt as f64).sqrt();
                                    if score > best_score {
                                        best_score = score; best_p = Params { move_thresh, vol_mult,
                                            adx_thresh, atr_mult, trail_atr, trail_act };
                                        best_is_wr = wr; best_is_t = nt;
                                    }
                                }
                            }
                        }
                    }
                }
                (best_p, best_is_wr, best_is_t)
            };

            let (bp, bis_wr, bis_t) = best_cfg;

            // OOS: best config
            let (be, bx) = if !is_short {
                signals_long(&oc,&oh,&ol,&ov,&or16,&ovm,&oadx,&orsi,&os20,&os50,&bp)
            } else {
                signals_short(&oc,&oh,&ol,&ov,&or16,&ovm,&oadx,&orsi,&os20,&os50,&bp)
            };
            let (bnt,bwr,bpf,bpnl,_,_,_) = sim(&oc,&oatr,&be,&bx,bp.atr_mult,bp.trail_atr,bp.trail_act,is_short);

            // OOS: universal config
            let (ue, ux) = if !is_short {
                signals_long(&oc,&oh,&ol,&ov,&or16,&ovm,&oadx,&orsi,&os20,&os50,&UNIV)
            } else {
                signals_short(&oc,&oh,&ol,&ov,&or16,&ovm,&oadx,&orsi,&os20,&os50,&UNIV)
            };
            let (unt,uwr,upf,upnl,_,_,_) = sim(&oc,&oatr,&ue,&ux,UNIV.atr_mult,UNIV.trail_atr,UNIV.trail_act,is_short);

            json!({
                "is_wr": bis_wr, "is_t": bis_t,
                "best": { "t": bnt, "wr": bwr, "pf": bpf, "pnl": bpnl,
                    "degrad": bwr - bis_wr,
                    "config": { "move": bp.move_thresh*100.0, "vol": bp.vol_mult,
                        "adx": bp.adx_thresh, "atr": bp.atr_mult,
                        "trail": bp.trail_atr, "act": bp.trail_act*100.0 } },
                "univ": { "t": unt, "wr": uwr, "pf": upf, "pnl": upnl,
                    "degrad": uwr - bis_wr },
            })
        };

        window_results.push(json!({
            "window": wi + 1,
            "train_bars": train_end,
            "test_bars": test_end - train_end,
            "long":  eval_dir(false),
            "short": eval_dir(true),
        }));
    }

    json!({ "coin": coin, "windows": window_results })
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN27.2 — Walk-Forward Validation (Momentum Breakout)");
    println!("================================================================");
    println!("3 expanding windows: W1[0-33/50%] W2[0-50/75%] W3[0-67/100%]");
    println!("Per window: grid on train → best config + universal on OOS test");
    println!("Universal: move=2.0% vol=2.0x ADX≥20 ATR×0.75 trail=1.0×ATR@0.8%");
    println!("Focus: does NEAR/DASH/XLM edge hold across all 3 windows?");
    println!();

    let coins: Vec<&str> = COIN_STRATEGIES.iter().map(|(c, _)| *c).collect();
    let results: Vec<Value> = coins.par_iter().map(|&c| process_coin(c)).collect();

    // ── Print per-coin, per-window table ─────────────────────────────────────
    // Header
    println!("  LONG  (best-config OOS WR% per window + universal)");
    println!("  {:6}  {:>7}  {:>7}  {:>7}  |  {:>7}  {:>7}  {:>7}  |  {:>5}  {:>5}",
        "Coin", "W1 IS%", "W1 OOS%", "W1 deg",
                "W2 OOS%", "W3 OOS%",  "AvgOOS",
                "Univ W3", "Pass?");
    println!("  {}", "-".repeat(76));

    let positive_coins = ["NEAR", "DASH", "XLM"];

    let mut all_long_oos: Vec<(String, Vec<f64>, Vec<f64>)> = Vec::new(); // (coin, best_oos, univ_oos)

    for r in &results {
        let coin = r["coin"].as_str().unwrap_or("?");
        let ws = r["windows"].as_array().unwrap();

        let mut long_is:       Vec<f64> = Vec::new();
        let mut long_oos_best: Vec<f64> = Vec::new();
        let mut long_oos_univ: Vec<f64> = Vec::new();
        let mut long_t:        Vec<u64> = Vec::new();

        for w in ws {
            let l = &w["long"];
            long_is.push(l["is_wr"].as_f64().unwrap_or(0.0));
            long_oos_best.push(l["best"]["wr"].as_f64().unwrap_or(0.0));
            long_oos_univ.push(l["univ"]["wr"].as_f64().unwrap_or(0.0));
            long_t.push(l["best"]["t"].as_u64().unwrap_or(0));
        }

        let avg_oos = long_oos_best.iter().sum::<f64>() / 3.0;
        let w3_univ = long_oos_univ[2];
        let w3_t    = long_t[2];
        let pass    = avg_oos >= 40.0 && w3_t >= 20;
        let mark    = if positive_coins.contains(&coin) { "*" } else { " " };

        println!("  {:6}{} {:>7.1}  {:>7.1}  {:>+7.1}  |  {:>7.1}  {:>7.1}  |  {:>7.1}  |  {:>5.1}  {:>5}",
            coin, mark,
            long_is[0], long_oos_best[0], long_oos_best[0] - long_is[0],
            long_oos_best[1], long_oos_best[2],
            avg_oos,
            w3_univ,
            if pass { "YES" } else { "no" });

        all_long_oos.push((coin.to_string(), long_oos_best.clone(), long_oos_univ.clone()));
    }

    println!();
    println!("  SHORT (best-config OOS WR% per window + universal)");
    println!("  {:6}  {:>7}  {:>7}  {:>7}  |  {:>7}  {:>7}  |  {:>7}  |  {:>5}  {:>5}",
        "Coin", "W1 IS%", "W1 OOS%", "W1 deg",
                "W2 OOS%", "W3 OOS%",
                "AvgOOS",  "Univ W3", "Pass?");
    println!("  {}", "-".repeat(76));

    for r in &results {
        let coin = r["coin"].as_str().unwrap_or("?");
        let ws = r["windows"].as_array().unwrap();

        let mut short_is:       Vec<f64> = Vec::new();
        let mut short_oos_best: Vec<f64> = Vec::new();
        let mut short_oos_univ: Vec<f64> = Vec::new();
        let mut short_t:        Vec<u64> = Vec::new();

        for w in ws {
            let s = &w["short"];
            short_is.push(s["is_wr"].as_f64().unwrap_or(0.0));
            short_oos_best.push(s["best"]["wr"].as_f64().unwrap_or(0.0));
            short_oos_univ.push(s["univ"]["wr"].as_f64().unwrap_or(0.0));
            short_t.push(s["best"]["t"].as_u64().unwrap_or(0));
        }

        let avg_oos = short_oos_best.iter().sum::<f64>() / 3.0;
        let w3_univ = short_oos_univ[2];
        let w3_t    = short_t[2];
        let pass    = avg_oos >= 40.0 && w3_t >= 15;
        let mark    = if coin == "NEAR" { "*" } else { " " };

        println!("  {:6}{} {:>7.1}  {:>7.1}  {:>+7.1}  |  {:>7.1}  {:>7.1}  |  {:>7.1}  |  {:>5.1}  {:>5}",
            coin, mark,
            short_is[0], short_oos_best[0], short_oos_best[0] - short_is[0],
            short_oos_best[1], short_oos_best[2],
            avg_oos,
            w3_univ,
            if pass { "YES" } else { "no" });
    }

    // Save
    let out_dir = "archive/RUN27";
    let out_path = format!("{}/run27_2_results.json", out_dir);
    let output = serde_json::json!({ "run": "RUN27.2", "coins": results });
    std::fs::write(&out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("\n  Results saved to {}", out_path);
}
