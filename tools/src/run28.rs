/// RUN28 — Momentum Persistence Classifier
///
/// Diagnoses WHY the breakout strategy works on NEAR/DASH/XLM but not on
/// other coins. Tests the hypothesis: "coins where hard moves don't continue
/// should be excluded from the strategy."
///
/// Method:
///   1. For every bar where breakout conditions fire (|ret16| ≥ thresh,
///      vol spike, ADX rising), record the event.
///   2. Measure the NEXT 4/8/16/32 bars: does price continue in the
///      same direction?
///   3. Compute continuation_rate = P(continues | event) per coin.
///   4. Compute baseline = P(continues | any random bar) — ~50%.
///   5. Edge = continuation_rate − baseline.
///   6. Rank coins by 16-bar edge. Coins with edge ≥ +5pp are
///      breakout-suitable; coins with edge ≤ 0 should be disabled.
///   7. Cross-reference with RUN27.1 WR/PF results to confirm.
///
/// Also reports:
///   - Event frequency (how often conditions fire per coin)
///   - Average forward return at each horizon (magnitude of persistence)
///   - Approximate t-stat (edge / (0.5/sqrt(n))) for significance
///
/// Three condition levels (loose/medium/strict) to see if stricter
/// conditions select better-persisting breakouts.
///
/// Fee/slip not needed — this is a forward-return measurement, not a sim.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::indicators::{rolling_mean, sma};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::run27::{adx14, bar16_return};

// Horizons to measure forward return (bars at 15m each)
const HORIZONS: [usize; 4] = [4, 8, 16, 32]; // 1h, 2h, 4h, 8h

// Condition levels: (move_thresh, vol_mult, adx_thresh, label)
const LEVELS: [(f64, f64, f64, &str); 3] = [
    (0.015, 1.5, 20.0, "loose"),    // ≥1.5%, 1.5×vol, ADX≥20
    (0.020, 2.0, 20.0, "medium"),   // ≥2.0%, 2.0×vol, ADX≥20  (universal from RUN27.1)
    (0.025, 2.0, 25.0, "strict"),   // ≥2.5%, 2.0×vol, ADX≥25
];

// RUN27.1 OOS results for cross-reference (long WR, short WR, long PF, short PF)
const RUN27_RESULTS: &[(&str, f64, f64, f64, f64)] = &[
    ("DASH", 37.1, 38.1, 1.48, 1.16),
    ("UNI",  20.0, 37.5, 0.41, 0.35),
    ("NEAR", 48.4, 46.2, 1.60, 1.84),
    ("ADA",  36.8, 40.0, 0.85, 0.95),
    ("LTC",  16.7, 40.0, 0.16, 0.70),
    ("SHIB", 42.1, 40.0, 1.42, 0.43),
    ("LINK", 16.7, 38.9, 0.12, 0.87),
    ("ETH",   0.0, 40.0, 0.00, 2.30),
    ("DOT",  18.2, 23.1, 0.64, 0.75),
    ("XRP",  36.4, 75.0, 0.28, 1.89),
    ("ATOM", 36.4, 50.0, 0.32, 0.14),
    ("SOL",  40.0, 41.2, 1.20, 0.62),
    ("DOGE", 62.5, 50.0, 0.93, 0.29),
    ("XLM",  50.0, 46.7, 1.68, 1.32),
    ("AVAX", 58.3, 20.0, 0.89, 0.04),
    ("ALGO", 33.3, 50.0, 0.63, 1.03),
    ("BNB",   0.0, 27.8, 0.00, 0.42),
    ("BTC",  44.4, 26.3, 0.58, 0.58),
];

struct PersistenceResult {
    // Per direction (0=long, 1=short)
    n_events:    [usize; 2],
    // continuation_rate[dir][horizon_idx]
    cont_rate:   [[f64; 4]; 2],
    // average forward return (%) at each horizon
    avg_fwd_ret: [[f64; 4]; 2],
    // baseline continuation rate (any random bar, long direction = close[i+H] > close[i])
    baseline:    [f64; 4],
}

fn analyse_coin(coin: &str, move_thresh: f64, vol_mult: f64, adx_thresh: f64)
    -> PersistenceResult
{
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

    let ret16  = bar16_return(&close);
    let vol_ma = rolling_mean(&vol, 20);
    let adx    = adx14(&high, &low, &close);
    // SMA50 context filter (same as RUN27.1)
    let sma50  = sma(&close, 50);

    // Max horizon determines how far from end we can measure
    let max_h = *HORIZONS.iter().max().unwrap();

    // ── Baseline: P(close[i+H] > close[i]) over all valid bars ──
    let mut base_cont = [0usize; 4];
    let mut base_n    = [0usize; 4];
    for i in 50..n.saturating_sub(max_h) {
        for (hi, &h) in HORIZONS.iter().enumerate() {
            if close[i] > 0.0 {
                base_n[hi] += 1;
                if close[i + h] > close[i] { base_cont[hi] += 1; }
            }
        }
    }
    let baseline: [f64; 4] = std::array::from_fn(|hi| {
        if base_n[hi] > 0 { base_cont[hi] as f64 / base_n[hi] as f64 * 100.0 } else { 50.0 }
    });

    // ── Event scan ────────────────────────────────────────────────────────────
    // For each direction: count continuations at each horizon
    let mut n_events = [0usize; 2];
    let mut cont_sum = [[0usize; 4]; 2];
    let mut fwd_sum  = [[0.0f64; 4]; 2];

    for i in 50..n.saturating_sub(max_h) {
        if vol_ma[i].is_nan() || adx[i].is_nan() || sma50[i].is_nan() { continue; }
        if close[i] <= 0.0 { continue; }

        let vol_ok  = vol[i] >= vol_ma[i] * vol_mult;
        let adx_ok  = adx[i] >= adx_thresh && i >= 3 && adx[i] > adx[i - 3];
        if !vol_ok || !adx_ok { continue; }

        // LONG breakout: large positive 16-bar return + close > SMA50
        if ret16[i] >= move_thresh && close[i] > sma50[i] {
            n_events[0] += 1;
            for (hi, &h) in HORIZONS.iter().enumerate() {
                let fwd = (close[i + h] - close[i]) / close[i] * 100.0;
                fwd_sum[0][hi]  += fwd;
                if fwd > 0.0 { cont_sum[0][hi] += 1; }
            }
        }

        // SHORT breakout: large negative 16-bar return + close < SMA50
        if ret16[i] <= -move_thresh && close[i] < sma50[i] {
            n_events[1] += 1;
            for (hi, &h) in HORIZONS.iter().enumerate() {
                let fwd = (close[i] - close[i + h]) / close[i] * 100.0; // positive = price fell
                fwd_sum[1][hi]  += fwd;
                if fwd > 0.0 { cont_sum[1][hi] += 1; }
            }
        }
    }

    let cont_rate: [[f64; 4]; 2] = std::array::from_fn(|dir| {
        std::array::from_fn(|hi| {
            if n_events[dir] > 0 {
                cont_sum[dir][hi] as f64 / n_events[dir] as f64 * 100.0
            } else { 50.0 }
        })
    });

    let avg_fwd_ret: [[f64; 4]; 2] = std::array::from_fn(|dir| {
        std::array::from_fn(|hi| {
            if n_events[dir] > 0 {
                fwd_sum[dir][hi] / n_events[dir] as f64
            } else { 0.0 }
        })
    });

    PersistenceResult { n_events, cont_rate, avg_fwd_ret, baseline }
}

// ── Entry point ────────────────────────────────────────────────────────────────
pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN28 — Momentum Persistence Classifier");
    println!("================================================================");
    println!("Question: after a qualifying hard move, does price CONTINUE or REVERSE?");
    println!("Metric: continuation_rate = P(price moves further in same direction)");
    println!("Baseline: ~50% (random bar). Edge = cont_rate − baseline.");
    println!("Focus horizon: 16 bars (4h equivalent at 15m)");
    println!();

    let coins: Vec<&str> = COIN_STRATEGIES.iter().map(|(c, _)| *c).collect();

    // Run all 3 condition levels
    let mut all_results: Vec<Value> = Vec::new();

    for &(move_thresh, vol_mult, adx_thresh, label) in &LEVELS {
        println!("────────────────────────────────────────────────────────────────");
        println!("Condition level: {} (move≥{:.1}%, vol≥{:.1}×, ADX≥{:.0})",
            label, move_thresh * 100.0, vol_mult, adx_thresh);
        println!("────────────────────────────────────────────────────────────────");
        println!("  {:6}  {:>4} {:>4}  {:>6} {:>6} {:>6} {:>6}  {:>6} {:>6} {:>6} {:>6}  {:>6} {:>6}",
            "Coin", "L.n", "S.n",
            "L@4h", "L@8h", "L@16h", "L@32h",
            "S@4h", "S@8h", "S@16h", "S@32h",
            "27WR_L", "27WR_S");
        println!("  {:6}  {:>4} {:>4}  {:>6} {:>6} {:>6} {:>6}  {:>6} {:>6} {:>6} {:>6}  {:>6} {:>6}",
            "", "", "",
            "edge%", "edge%", "edge%", "edge%",
            "edge%", "edge%", "edge%", "edge%",
            "WR27", "WR27");
        println!("  {}", "-".repeat(90));

        let results: Vec<(String, PersistenceResult)> = coins.par_iter()
            .map(|&c| (c.to_string(), analyse_coin(c, move_thresh, vol_mult, adx_thresh)))
            .collect();

        // Sort by 16-bar long edge (index 2 = 16-bar horizon)
        let mut sorted = results;
        sorted.sort_by(|a, b| {
            let ea = a.1.cont_rate[0][2] - a.1.baseline[2];
            let eb = b.1.cont_rate[0][2] - b.1.baseline[2];
            eb.partial_cmp(&ea).unwrap()
        });

        let mut coin_json: Vec<Value> = Vec::new();

        for (coin, pr) in &sorted {
            let r27 = RUN27_RESULTS.iter().find(|(c, ..)| *c == coin.as_str());
            let (r27_lwr, r27_swr) = r27.map(|x| (x.1, x.2)).unwrap_or((0.0, 0.0));

            // Edge = cont_rate − baseline
            let le: Vec<f64> = (0..4).map(|hi| pr.cont_rate[0][hi] - pr.baseline[hi]).collect();
            let se: Vec<f64> = (0..4).map(|hi| pr.cont_rate[1][hi] - pr.baseline[hi]).collect();

            let mark_l = if le[2] >= 5.0 { "+" } else if le[2] <= 0.0 { "-" } else { " " };
            let mark_s = if se[2] >= 5.0 { "+" } else if se[2] <= 0.0 { "-" } else { " " };
            let _ = (mark_l, mark_s);

            println!("  {:6}  {:>4} {:>4}  {:>+6.1} {:>+6.1} {:>+6.1} {:>+6.1}  {:>+6.1} {:>+6.1} {:>+6.1} {:>+6.1}  {:>6.1} {:>6.1}",
                coin,
                pr.n_events[0], pr.n_events[1],
                le[0], le[1], le[2], le[3],
                se[0], se[1], se[2], se[3],
                r27_lwr, r27_swr);

            // t-stat at 16-bar horizon (binomial test vs 50%)
            let tstat_l = if pr.n_events[0] > 0 {
                (pr.cont_rate[0][2] / 100.0 - pr.baseline[2] / 100.0)
                    / (0.5 / (pr.n_events[0] as f64).sqrt())
            } else { 0.0 };
            let tstat_s = if pr.n_events[1] > 0 {
                (pr.cont_rate[1][2] / 100.0 - pr.baseline[2] / 100.0)
                    / (0.5 / (pr.n_events[1] as f64).sqrt())
            } else { 0.0 };

            coin_json.push(json!({
                "coin": coin,
                "n_events_long": pr.n_events[0],
                "n_events_short": pr.n_events[1],
                "baseline": pr.baseline,
                "long_cont_rate": pr.cont_rate[0],
                "short_cont_rate": pr.cont_rate[1],
                "long_edge_16": le[2],
                "short_edge_16": se[2],
                "long_tstat_16": tstat_l,
                "short_tstat_16": tstat_s,
                "long_avg_fwd_ret_16": pr.avg_fwd_ret[0][2],
                "short_avg_fwd_ret_16": pr.avg_fwd_ret[1][2],
                "run27_long_wr": r27_lwr,
                "run27_short_wr": r27_swr,
            }));
        }

        println!();
        println!("  Baseline 16-bar continuation rate: ~{:.1}%",
            sorted.first().map(|x| x.1.baseline[2]).unwrap_or(50.0));
        println!("  Breakout-suitable (long edge ≥ +5pp): {}",
            sorted.iter()
                .filter(|(_, pr)| pr.cont_rate[0][2] - pr.baseline[2] >= 5.0
                                   && pr.n_events[0] >= 10)
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(", "));
        println!("  Anti-momentum long (edge ≤ 0): {}",
            sorted.iter()
                .filter(|(_, pr)| pr.cont_rate[0][2] - pr.baseline[2] <= 0.0
                                   && pr.n_events[0] >= 5)
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(", "));
        println!();

        all_results.push(json!({
            "level": label,
            "move_thresh_pct": move_thresh * 100.0,
            "vol_mult": vol_mult,
            "adx_thresh": adx_thresh,
            "coins": coin_json,
        }));
    }

    // ── Average forward return table (medium level, 16-bar) ──────────────────
    println!("════════════════════════════════════════════════════════════════");
    println!("Average forward return after breakout event (medium params, 16 bars)");
    println!("Positive = price continues in breakout direction");
    println!("  {:6}  {:>5} {:>8}  {:>5} {:>8}  {:>7} {:>7}",
        "Coin", "L.n", "L.fwd16%", "S.n", "S.fwd16%", "27WR_L", "27WR_S");
    println!("  {}", "-".repeat(60));

    let medium_results: Vec<(String, PersistenceResult)> = coins.par_iter()
        .map(|&c| (c.to_string(), analyse_coin(c, 0.020, 2.0, 20.0)))
        .collect();

    let mut med_sorted = medium_results;
    med_sorted.sort_by(|a, b| b.1.avg_fwd_ret[0][2].partial_cmp(&a.1.avg_fwd_ret[0][2]).unwrap());

    for (coin, pr) in &med_sorted {
        let r27 = RUN27_RESULTS.iter().find(|(c, ..)| *c == coin.as_str());
        let (r27_lwr, r27_swr) = r27.map(|x| (x.1, x.2)).unwrap_or((0.0, 0.0));
        println!("  {:6}  {:>5} {:>+8.3}  {:>5} {:>+8.3}  {:>7.1} {:>7.1}",
            coin,
            pr.n_events[0], pr.avg_fwd_ret[0][2],
            pr.n_events[1], pr.avg_fwd_ret[1][2],
            r27_lwr, r27_swr);
    }

    println!();
    println!("Correlation insight: coins with higher avg_fwd_ret[16] should");
    println!("match coins with higher RUN27.1 WR — confirming persistence drives edge.");

    // Save
    let out_path = "archive/RUN28/run28_results.json";
    std::fs::write(out_path,
        serde_json::to_string_pretty(&json!({ "run": "RUN28", "levels": all_results })).unwrap()
    ).unwrap();
    println!("\n  Results saved to {}", out_path);
}
