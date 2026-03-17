/// RUN20 — Momentum Crowding Filter
///
/// Original plan: test funding rate & OI as entry filters (requires external data).
/// Reframed: test OHLCV-derived momentum/crowding proxies — the same mechanism
/// (mean reversion works better when market is not "crowded" with late buyers)
/// without requiring derivatives data.
///
/// Hypothesis: COINCLAW vwap_rev / bb_bounce entries that follow strong 3-day
/// price run-ups fail more often (stop-losses) than entries after flat/down 3 days.
/// Filtering out "crowded" entries improves OOS WR and P&L.
///
/// Filters tested (all applied to COINCLAW entry signals before trade simulation):
///   Baseline    — all signals
///   Mom2pct     — skip entry if 3d return > +2%
///   Mom1pct     — skip entry if 3d return > +1%  (stricter)
///   Mom3pct     — skip entry if 3d return > +3%  (looser)
///   VolCrowd    — skip entry if 3d vol/15d vol ratio > 2.0 (heavy buying volume)
///   Combined    — Mom2pct AND VolCrowd
///   AntiMom     — skip entry if 3d return < -3%  (falling knife guard)
///
/// Trade params: SL=0.3% | fee=0.1%/side | slip=0.05%/side | no TP
/// Split: 50/50 chronological OOS

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::backtest::STOP_LOSS;
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::strategies::signals;

const FEE:      f64 = 0.001;  // 0.1%/side
const SLIP:     f64 = 0.0005; // 0.05%/side
const BARS_DAY: usize = 96;   // 15m bars per day

// ── Trade simulation with pre-filtered entry signals ─────────────────────────

fn simulate_filter(
    close: &[f64],
    vol: &[f64],
    entry_raw: &[bool],
    exit_raw: &[bool],
    mom_hi: Option<f64>,   // skip if 3d_return > mom_hi
    mom_lo: Option<f64>,   // skip if 3d_return < mom_lo
    vol_threshold: Option<f64>, // skip if 3d_vol_ratio > threshold
) -> (Vec<f64>, usize, usize) {
    // Precompute 3-day return and 3-day/15-day volume ratio
    let n = close.len();
    let lookback_3d  = BARS_DAY * 3;    // 288 bars
    let lookback_15d = BARS_DAY * 15;   // 1440 bars

    // Rolling volume averages (simple: trailing sum / window)
    let mut vol_sum_3d  = 0.0f64;
    let mut vol_sum_15d = 0.0f64;
    let mut vol_ratio   = vec![f64::NAN; n];

    for i in 0..n {
        vol_sum_3d  += vol[i];
        vol_sum_15d += vol[i];
        if i >= lookback_3d  { vol_sum_3d  -= vol[i - lookback_3d]; }
        if i >= lookback_15d { vol_sum_15d -= vol[i - lookback_15d]; }
        if i >= lookback_15d && vol_sum_15d > 0.0 {
            let avg_3d  = vol_sum_3d  / lookback_3d  as f64;
            let avg_15d = vol_sum_15d / lookback_15d as f64;
            vol_ratio[i] = avg_3d / avg_15d;
        }
    }

    let mut pnls = Vec::new();
    let mut in_pos = false;
    let mut entry_price = 0.0f64;
    let mut n_filtered = 0usize;
    let mut n_passed   = 0usize;

    for i in 0..n {
        if in_pos {
            let pnl_frac = (close[i] - entry_price) / entry_price;
            if pnl_frac <= -STOP_LOSS {
                let ep = entry_price * (1.0 - STOP_LOSS * (1.0 + SLIP));
                let pnl = (ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0;
                pnls.push(pnl);
                in_pos = false;
            } else if exit_raw[i] {
                let ep = close[i] * (1.0 - SLIP);
                let pnl = (ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0;
                pnls.push(pnl);
                in_pos = false;
            }
        } else if entry_raw[i] && i >= lookback_3d {
            // Compute 3-day return
            let ret_3d = (close[i] - close[i - lookback_3d]) / close[i - lookback_3d];

            // Apply filters
            let filtered = (mom_hi.map_or(false, |hi| ret_3d > hi))
                        || (mom_lo.map_or(false, |lo| ret_3d < lo))
                        || (vol_threshold.map_or(false, |th| !vol_ratio[i].is_nan() && vol_ratio[i] > th));

            if filtered {
                n_filtered += 1;
            } else {
                n_passed += 1;
                entry_price = close[i] * (1.0 + SLIP);
                in_pos = true;
            }
        }
    }

    if in_pos {
        let ep = *close.last().unwrap();
        let pnl = (ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0;
        pnls.push(pnl);
    }

    (pnls, n_passed, n_filtered)
}

fn summarise(pnls: &[f64], n_passed: usize, n_filtered: usize, n_raw: usize) -> Value {
    let n = pnls.len();
    if n == 0 {
        return json!({"n_trades": 0, "win_rate": 0.0, "total_pnl": 0.0,
                      "profit_factor": 0.0, "max_dd": 0.0,
                      "n_passed": n_passed, "n_filtered": n_filtered,
                      "pct_of_baseline": 0.0});
    }
    let wins: Vec<f64>  = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).map(|p| -p).collect();
    let win_rate = wins.len() as f64 / n as f64 * 100.0;
    let gross_win: f64  = wins.iter().sum();
    let gross_loss: f64 = losses.iter().sum();
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win };

    let mut equity = 10_000.0f64;
    let mut peak = equity;
    let mut max_dd = 0.0f64;
    for &p in pnls {
        equity *= 1.0 + p / 100.0;
        if equity > peak { peak = equity; }
        let dd = (peak - equity) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }
    let total_pnl = (equity - 10_000.0) / 10_000.0 * 100.0;
    let pct_of_baseline = n_passed as f64 / n_raw.max(1) as f64 * 100.0;

    json!({
        "n_trades": n,
        "win_rate": win_rate,
        "total_pnl": total_pnl,
        "profit_factor": pf,
        "max_dd": max_dd,
        "n_passed": n_passed,
        "n_filtered": n_filtered,
        "pct_of_baseline": pct_of_baseline,
    })
}

// ── Per-coin ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct CoinResult {
    coin:      String,
    strategy:  String,
    baseline:  Value,
    mom2:      Value,
    mom1:      Value,
    mom3:      Value,
    vol_crowd: Value,
    combined:  Value,
    anti_mom:  Value,
}

fn process_coin(coin: &str, strategy: &str) -> CoinResult {
    let bars = load_ohlcv(coin);
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();
    let (entry, exit) = signals(&bars, strategy);
    let n = close.len();

    // 50/50 split — only evaluate test half
    let split = n / 2;
    let close_t = &close[split..];
    let vol_t   = &vol[split..];
    let entry_t = &entry[split..];
    let exit_t  = &exit[split..];

    // Baseline entry count (for pct_of_baseline)
    let n_raw: usize = entry_t.iter().filter(|&&e| e).count();

    let run = |mom_hi: Option<f64>, mom_lo: Option<f64>, vol_thr: Option<f64>| {
        simulate_filter(close_t, vol_t, entry_t, exit_t, mom_hi, mom_lo, vol_thr)
    };

    let (b_pnls, b_pass, b_filt) = run(None, None, None);
    let (m2_pnls, m2_pass, m2_filt) = run(Some(0.02), None, None);
    let (m1_pnls, m1_pass, m1_filt) = run(Some(0.01), None, None);
    let (m3_pnls, m3_pass, m3_filt) = run(Some(0.03), None, None);
    let (vc_pnls, vc_pass, vc_filt) = run(None, None, Some(2.0));
    let (cb_pnls, cb_pass, cb_filt) = run(Some(0.02), None, Some(2.0));
    let (am_pnls, am_pass, am_filt) = run(None, Some(-0.03), None);

    CoinResult {
        coin:      coin.to_string(),
        strategy:  strategy.to_string(),
        baseline:  summarise(&b_pnls,  b_pass,  b_filt,  n_raw),
        mom2:      summarise(&m2_pnls, m2_pass, m2_filt, n_raw),
        mom1:      summarise(&m1_pnls, m1_pass, m1_filt, n_raw),
        mom3:      summarise(&m3_pnls, m3_pass, m3_filt, n_raw),
        vol_crowd: summarise(&vc_pnls, vc_pass, vc_filt, n_raw),
        combined:  summarise(&cb_pnls, cb_pass, cb_filt, n_raw),
        anti_mom:  summarise(&am_pnls, am_pass, am_filt, n_raw),
    }
}

// ── Portfolio aggregation ─────────────────────────────────────────────────────

fn port_avg(results: &[CoinResult], get: impl Fn(&CoinResult) -> &Value) -> (f64, f64, f64) {
    let n = results.len() as f64;
    let wr  = results.iter().map(|r| get(r)["win_rate"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    let pnl = results.iter().map(|r| get(r)["total_pnl"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    let dd  = results.iter().map(|r| get(r)["max_dd"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    (wr, pnl, dd)
}

fn trades_pct(results: &[CoinResult], get: impl Fn(&CoinResult) -> &Value) -> f64 {
    let n = results.len() as f64;
    results.iter().map(|r| get(r)["pct_of_baseline"].as_f64().unwrap_or(0.0)).sum::<f64>() / n
}

// ── Main ──────────────────────────────────────────────────────────────────────

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN20 — Momentum Crowding Filter");
    println!("================================================================");
    println!("Note: Originally planned as funding-rate/OI filter (no derivatives");
    println!("      data in cache). Reframed as OHLCV momentum/crowding proxy.");
    println!("Hypothesis: entries after strong 3d run-ups fail more often");
    println!("Data: 18 coins, 15m 1-year | COINCLAW v13 strategies | OOS test half");
    println!("Filters: Baseline | Mom>2% | Mom>1% | Mom>3% | VolCrowd | Combined | AntiMom<-3%");
    println!("Trade: SL=0.3% fee=0.1%/side slip=0.05%/side | Breakeven WR ~44%");
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.to_vec();

    let results: Vec<CoinResult> = coins
        .par_iter()
        .filter(|_| !shutdown.load(Ordering::SeqCst))
        .map(|(coin, strat)| process_coin(coin, strat))
        .collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Shutdown before completion.");
        return;
    }

    let mut sorted = results;
    sorted.sort_by(|a, b| a.coin.cmp(&b.coin));

    // ── Per-coin WR table ──────────────────────────────────────────────────────
    println!("  Per-coin WR%  (OOS test half, with fees)");
    println!("  {:<6}  {:<10}  {:>5}  {:>6}  {:>6}  {:>6}  {:>7}  {:>8}  {:>8}",
        "Coin", "Strategy", "Base", "Mom2%", "Mom1%", "Mom3%", "VolCrwd", "Combined", "AntiMom");
    println!("  {}", "-".repeat(76));
    for r in &sorted {
        let wr = |v: &Value| v["win_rate"].as_f64().unwrap_or(0.0);
        println!("  {:<6}  {:<10}  {:>5.1}  {:>6.1}  {:>6.1}  {:>6.1}  {:>7.1}  {:>8.1}  {:>8.1}",
            r.coin, r.strategy,
            wr(&r.baseline), wr(&r.mom2), wr(&r.mom1), wr(&r.mom3),
            wr(&r.vol_crowd), wr(&r.combined), wr(&r.anti_mom));
    }

    // ── Per-coin P&L table ─────────────────────────────────────────────────────
    println!();
    println!("  Per-coin Total P&L%  (OOS test half)");
    println!("  {:<6}  {:>7}  {:>7}  {:>7}  {:>7}  {:>8}  {:>9}  {:>9}",
        "Coin", "Base", "Mom2%", "Mom1%", "Mom3%", "VolCrwd", "Combined", "AntiMom");
    println!("  {}", "-".repeat(68));
    for r in &sorted {
        let pnl = |v: &Value| v["total_pnl"].as_f64().unwrap_or(0.0);
        println!("  {:<6}  {:>7.1}  {:>7.1}  {:>7.1}  {:>7.1}  {:>8.1}  {:>9.1}  {:>9.1}",
            r.coin,
            pnl(&r.baseline), pnl(&r.mom2), pnl(&r.mom1), pnl(&r.mom3),
            pnl(&r.vol_crowd), pnl(&r.combined), pnl(&r.anti_mom));
    }

    // ── Trade retention table ──────────────────────────────────────────────────
    println!();
    println!("  Trade retention (avg % of baseline signals kept):");
    let tpct = |get: fn(&CoinResult) -> &Value| trades_pct(&sorted, get);
    println!("  Mom2%={:.1}%  Mom1%={:.1}%  Mom3%={:.1}%  VolCrowd={:.1}%  Combined={:.1}%  AntiMom={:.1}%",
        tpct(|r| &r.mom2),
        tpct(|r| &r.mom1),
        tpct(|r| &r.mom3),
        tpct(|r| &r.vol_crowd),
        tpct(|r| &r.combined),
        tpct(|r| &r.anti_mom),
    );

    // ── Portfolio summary ──────────────────────────────────────────────────────
    let (bwr, bpnl, bdd) = port_avg(&sorted, |r| &r.baseline);
    let (w2, p2, d2)     = port_avg(&sorted, |r| &r.mom2);
    let (w1, p1, d1)     = port_avg(&sorted, |r| &r.mom1);
    let (w3, p3, d3)     = port_avg(&sorted, |r| &r.mom3);
    let (wv, pv, dv)     = port_avg(&sorted, |r| &r.vol_crowd);
    let (wc, pc, dc)     = port_avg(&sorted, |r| &r.combined);
    let (wa, pa, da)     = port_avg(&sorted, |r| &r.anti_mom);

    println!();
    println!("  Portfolio Summary (18-coin equal-weight average, OOS test half):");
    println!("  {:>12}  {:>7}  {:>8}  {:>9}",
        "Filter", "Avg WR%", "Avg PnL%", "Avg MaxDD%");
    println!("  {}", "-".repeat(42));
    for (name, wr, pnl, dd) in [
        ("Baseline",  bwr, bpnl, bdd),
        ("Mom > +2%", w2,  p2,   d2),
        ("Mom > +1%", w1,  p1,   d1),
        ("Mom > +3%", w3,  p3,   d3),
        ("VolCrowd",  wv,  pv,   dv),
        ("Combined",  wc,  pc,   dc),
        ("AntiMom<-3%",wa, pa,   da),
    ] {
        println!("  {:>12}  {:>7.2}  {:>8.2}  {:>9.2}", name, wr, pnl, dd);
    }

    // ── Conclusion ─────────────────────────────────────────────────────────────
    let best_pnl = [("Mom>2%", p2), ("Mom>1%", p1), ("Mom>3%", p3),
                    ("VolCrwd", pv), ("Combined", pc), ("AntiMom", pa)]
        .iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(name, pnl)| (*name, *pnl))
        .unwrap();

    println!();
    if best_pnl.1 > bpnl + 0.5 {
        println!("  Best filter: {} (+{:.2}% vs baseline {:.2}%)", best_pnl.0, best_pnl.1 - bpnl, bpnl);
    } else {
        println!("  No filter materially beats baseline ({:.2}%)", bpnl);
    }
    println!("  Breakeven WR ≈ 44% — all filters must clear this to be viable");

    // ── Save results ───────────────────────────────────────────────────────────
    let per_coin: Vec<Value> = sorted.iter().map(|r| json!({
        "coin": r.coin,
        "strategy": r.strategy,
        "baseline":  r.baseline,
        "mom2pct":   r.mom2,
        "mom1pct":   r.mom1,
        "mom3pct":   r.mom3,
        "vol_crowd": r.vol_crowd,
        "combined":  r.combined,
        "anti_mom":  r.anti_mom,
    })).collect();

    let summary = json!({
        "portfolio": {
            "baseline":  {"avg_wr": bwr, "avg_pnl": bpnl, "avg_dd": bdd},
            "mom2pct":   {"avg_wr": w2,  "avg_pnl": p2,   "avg_dd": d2},
            "mom1pct":   {"avg_wr": w1,  "avg_pnl": p1,   "avg_dd": d1},
            "mom3pct":   {"avg_wr": w3,  "avg_pnl": p3,   "avg_dd": d3},
            "vol_crowd": {"avg_wr": wv,  "avg_pnl": pv,   "avg_dd": dv},
            "combined":  {"avg_wr": wc,  "avg_pnl": pc,   "avg_dd": dc},
            "anti_mom":  {"avg_wr": wa,  "avg_pnl": pa,   "avg_dd": da},
        },
        "note": "Derivatives data not available. Test uses OHLCV momentum proxy for funding-rate crowding.",
    });

    let out = json!({ "per_coin": per_coin, "summary": summary });
    let out_path = "archive/RUN20/run20_results.json";
    std::fs::create_dir_all("archive/RUN20").ok();
    std::fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!();
    println!("  Results saved to {}", out_path);
}
