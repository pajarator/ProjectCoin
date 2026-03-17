/// RUN21 — Sentiment Regime Filter (BTC as Market Fear/Greed Proxy)
///
/// Original plan: Fear & Greed index from alternative.me (no cached data).
/// Reframed: BTC RSI(14) and BTC z-score(50) as market sentiment proxy.
/// F&G is ~40% derived from price momentum/RSI — BTC RSI is the cleaner version.
///
/// Hypothesis: COINCLAW mean reversion entries work better when the broader
/// crypto market (BTC) is in "fear" mode (oversold). "Buy the fear" — extreme
/// bearishness leads to overshooting + clean reversions. Greed mode = crowded
/// market where false dips keep continuing.
///
/// Regime definitions (BTC RSI14):
///   ExtremeFear  RSI < 30
///   Fear         RSI < 40  (includes ExtremeFear)
///   Neutral      RSI 40–60
///   Greed        RSI > 60  (includes ExtremeGreed)
///   ExtremeGreed RSI > 70
///
/// Also tested: BTC z-score(50) < -1.0 as alternative fear proxy.
///
/// Method:
///   - Load all 18 coins + BTC 15m 1-year data
///   - Compute BTC RSI14 and z-score(50) at each bar
///   - For each coin: run COINCLAW v13 strategy, apply BTC regime filter to entries
///   - Simulate trades: SL=0.3% fee=0.1%/side slip=0.05%/side
///   - 50/50 chrono split, evaluate OOS test half only

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::backtest::STOP_LOSS;
use crate::indicators::{rsi, z_score};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::strategies::signals;

const FEE:  f64 = 0.001;  // 0.1%/side
const SLIP: f64 = 0.0005; // 0.05%/side

// ── BTC sentiment regimes ─────────────────────────────────────────────────────

struct BtcSentiment {
    rsi14:   Vec<f64>,  // BTC RSI(14) per bar
    zscore50: Vec<f64>, // BTC z-score(50) per bar
    n:       usize,
}

impl BtcSentiment {
    fn load() -> Self {
        let bars = load_ohlcv("BTC");
        let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
        let n = close.len();
        let rsi14    = rsi(&close, 14);
        let zscore50 = z_score(&close, 50);
        BtcSentiment { rsi14, zscore50, n }
    }

    fn rsi_at(&self, i: usize) -> f64 { self.rsi14[i] }
    fn z_at(&self, i: usize)   -> f64 { self.zscore50[i] }
}

// ── Trade simulation ──────────────────────────────────────────────────────────

fn sim(
    close: &[f64],
    entry: &[bool],
    exit: &[bool],
    mask: &[bool],      // true = entry allowed at this bar
) -> Value {
    let n = close.len();
    let mut pnls = Vec::new();
    let mut in_pos = false;
    let mut entry_price = 0.0f64;
    let mut n_raw = 0usize;

    for i in 0..n {
        if entry[i] { n_raw += 1; }
        if in_pos {
            let pnl_frac = (close[i] - entry_price) / entry_price;
            if pnl_frac <= -STOP_LOSS {
                let ep = entry_price * (1.0 - STOP_LOSS * (1.0 + SLIP));
                pnls.push((ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            } else if exit[i] {
                let ep = close[i] * (1.0 - SLIP);
                pnls.push((ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            }
        } else if entry[i] && mask[i] {
            entry_price = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let ep = *close.last().unwrap();
        pnls.push((ep - entry_price) / entry_price * 100.0 - FEE * 2.0 * 100.0);
    }

    let n_trades = pnls.len();
    if n_trades == 0 {
        return json!({"n_trades": 0, "n_raw": n_raw, "win_rate": 0.0,
                      "total_pnl": 0.0, "profit_factor": 0.0, "max_dd": 0.0,
                      "pct_of_raw": 0.0});
    }
    let wins: Vec<f64> = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).map(|p| -p).collect();
    let win_rate = wins.len() as f64 / n_trades as f64 * 100.0;
    let gw: f64 = wins.iter().sum();
    let gl: f64 = losses.iter().sum();
    let pf = if gl > 0.0 { gw / gl } else { gw };

    let mut eq = 10_000.0f64;
    let mut peak = eq;
    let mut max_dd = 0.0f64;
    for &p in &pnls {
        eq *= 1.0 + p / 100.0;
        if eq > peak { peak = eq; }
        let dd = (peak - eq) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }
    let total_pnl = (eq - 10_000.0) / 10_000.0 * 100.0;

    json!({
        "n_trades": n_trades,
        "n_raw": n_raw,
        "win_rate": win_rate,
        "total_pnl": total_pnl,
        "profit_factor": pf,
        "max_dd": max_dd,
        "pct_of_raw": n_trades as f64 / n_raw.max(1) as f64 * 100.0,
    })
}

// ── Per-coin ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct CoinResult {
    coin:          String,
    strategy:      String,
    baseline:      Value,
    fear:          Value,  // RSI < 40
    extreme_fear:  Value,  // RSI < 30
    neutral:       Value,  // RSI 40–60
    greed:         Value,  // RSI > 60
    extreme_greed: Value,  // RSI > 70
    z_fear:        Value,  // z50 < -1.0
}

fn process_coin(coin: &str, strategy: &str, btc: &BtcSentiment) -> CoinResult {
    let bars  = load_ohlcv(coin);
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let n = close.len();
    assert_eq!(n, btc.n, "Bar count mismatch for {}", coin);

    let (entry, exit) = signals(&bars, strategy);

    // 50/50 split — test half only
    let split = n / 2;
    let close_t = &close[split..];
    let entry_t = &entry[split..];
    let exit_t  = &exit[split..];
    let nt = close_t.len();

    // Build BTC regime masks for test half
    let all_true: Vec<bool>  = vec![true; nt];
    let fear_mask: Vec<bool> = (split..n).map(|i| {
        let r = btc.rsi_at(i); !r.is_nan() && r < 40.0
    }).collect();
    let efear_mask: Vec<bool> = (split..n).map(|i| {
        let r = btc.rsi_at(i); !r.is_nan() && r < 30.0
    }).collect();
    let neutral_mask: Vec<bool> = (split..n).map(|i| {
        let r = btc.rsi_at(i); !r.is_nan() && r >= 40.0 && r <= 60.0
    }).collect();
    let greed_mask: Vec<bool> = (split..n).map(|i| {
        let r = btc.rsi_at(i); !r.is_nan() && r > 60.0
    }).collect();
    let egreed_mask: Vec<bool> = (split..n).map(|i| {
        let r = btc.rsi_at(i); !r.is_nan() && r > 70.0
    }).collect();
    let zfear_mask: Vec<bool> = (split..n).map(|i| {
        let z = btc.z_at(i); !z.is_nan() && z < -1.0
    }).collect();

    CoinResult {
        coin:          coin.to_string(),
        strategy:      strategy.to_string(),
        baseline:      sim(close_t, entry_t, exit_t, &all_true),
        fear:          sim(close_t, entry_t, exit_t, &fear_mask),
        extreme_fear:  sim(close_t, entry_t, exit_t, &efear_mask),
        neutral:       sim(close_t, entry_t, exit_t, &neutral_mask),
        greed:         sim(close_t, entry_t, exit_t, &greed_mask),
        extreme_greed: sim(close_t, entry_t, exit_t, &egreed_mask),
        z_fear:        sim(close_t, entry_t, exit_t, &zfear_mask),
    }
}

// ── Portfolio aggregation ─────────────────────────────────────────────────────

fn pavg(results: &[CoinResult], get: impl Fn(&CoinResult) -> &Value) -> (f64, f64, f64, f64) {
    let n = results.len() as f64;
    let wr   = results.iter().map(|r| get(r)["win_rate"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    let pnl  = results.iter().map(|r| get(r)["total_pnl"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    let dd   = results.iter().map(|r| get(r)["max_dd"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    let pct  = results.iter().map(|r| get(r)["pct_of_raw"].as_f64().unwrap_or(0.0)).sum::<f64>() / n;
    (wr, pnl, dd, pct)
}

// ── Main ──────────────────────────────────────────────────────────────────────

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN21 — Sentiment Regime Filter (BTC RSI as Fear/Greed Proxy)");
    println!("================================================================");
    println!("Note: No Fear & Greed cache. Using BTC RSI(14) as market sentiment proxy.");
    println!("Hypothesis: COINCLAW mean reversion entries succeed more in BTC 'fear' regimes.");
    println!("Data: 18 coins, 15m 1-year | COINCLAW v13 | OOS test half");
    println!("Regimes: BTC RSI<30 (ExtFear) | <40 (Fear) | 40-60 (Neutral) | >60 (Greed) | >70 (ExtGreed)");
    println!("         BTC z50<-1.0 (Z-Fear)");
    println!("Trade: SL=0.3% fee=0.1%/side slip=0.05%/side | Breakeven WR ~44%");
    println!();

    // Load BTC sentiment first (shared across all coins)
    print!("  Loading BTC sentiment... ");
    let btc = BtcSentiment::load();
    println!("OK ({} bars)", btc.n);

    // Log regime coverage on test half
    let split = btc.n / 2;
    let test_n = btc.n - split;
    let fear_bars  = (split..btc.n).filter(|&i| { let r = btc.rsi_at(i); !r.is_nan() && r < 40.0 }).count();
    let greed_bars = (split..btc.n).filter(|&i| { let r = btc.rsi_at(i); !r.is_nan() && r > 60.0 }).count();
    let efear_bars = (split..btc.n).filter(|&i| { let r = btc.rsi_at(i); !r.is_nan() && r < 30.0 }).count();
    let zfear_bars = (split..btc.n).filter(|&i| { let z = btc.z_at(i);   !z.is_nan() && z < -1.0 }).count();
    println!("  Test half BTC regime coverage ({} bars):", test_n);
    println!("    Fear (RSI<40): {:.1}%   ExtFear (RSI<30): {:.1}%",
        fear_bars as f64 / test_n as f64 * 100.0,
        efear_bars as f64 / test_n as f64 * 100.0);
    println!("    Greed (RSI>60): {:.1}%  Z-Fear (z<-1): {:.1}%",
        greed_bars as f64 / test_n as f64 * 100.0,
        zfear_bars as f64 / test_n as f64 * 100.0);
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.to_vec();

    // Cannot run in parallel — BtcSentiment is shared read-only, but rayon needs Send
    // Process sequentially (fast enough, pure computation)
    let mut results: Vec<CoinResult> = coins.iter()
        .filter(|_| !shutdown.load(Ordering::SeqCst))
        .map(|(coin, strat)| process_coin(coin, strat, &btc))
        .collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Shutdown before completion.");
        return;
    }

    results.sort_by(|a, b| a.coin.cmp(&b.coin));

    // ── Per-coin WR table ──────────────────────────────────────────────────────
    println!("  Per-coin WR% by BTC regime (OOS test half)");
    println!("  {:<6}  {:>5}  {:>7}  {:>7}  {:>7}  {:>6}  {:>7}  {:>7}",
        "Coin", "Base", "EFear", "Fear", "Neutral", "Greed", "EGreed", "Z-Fear");
    println!("  {}", "-".repeat(62));
    for r in &results {
        let wr = |v: &Value| v["win_rate"].as_f64().unwrap_or(0.0);
        println!("  {:<6}  {:>5.1}  {:>7.1}  {:>7.1}  {:>7.1}  {:>6.1}  {:>7.1}  {:>7.1}",
            r.coin,
            wr(&r.baseline), wr(&r.extreme_fear), wr(&r.fear),
            wr(&r.neutral), wr(&r.greed), wr(&r.extreme_greed), wr(&r.z_fear));
    }

    // ── Per-coin P&L table ─────────────────────────────────────────────────────
    println!();
    println!("  Per-coin Total P&L% by BTC regime (OOS test half)");
    println!("  {:<6}  {:>6}  {:>7}  {:>7}  {:>8}  {:>7}  {:>8}  {:>7}",
        "Coin", "Base", "EFear", "Fear", "Neutral", "Greed", "EGreed", "Z-Fear");
    println!("  {}", "-".repeat(64));
    for r in &results {
        let p = |v: &Value| v["total_pnl"].as_f64().unwrap_or(0.0);
        println!("  {:<6}  {:>6.1}  {:>7.1}  {:>7.1}  {:>8.1}  {:>7.1}  {:>8.1}  {:>7.1}",
            r.coin,
            p(&r.baseline), p(&r.extreme_fear), p(&r.fear),
            p(&r.neutral), p(&r.greed), p(&r.extreme_greed), p(&r.z_fear));
    }

    // ── Trade retention ────────────────────────────────────────────────────────
    println!();
    println!("  Avg trade retention (% of baseline signals in regime):");
    let (_, _, _, pf) = pavg(&results, |r| &r.fear);
    let (_, _, _, pe) = pavg(&results, |r| &r.extreme_fear);
    let (_, _, _, pn) = pavg(&results, |r| &r.neutral);
    let (_, _, _, pg) = pavg(&results, |r| &r.greed);
    let (_, _, _, peg) = pavg(&results, |r| &r.extreme_greed);
    let (_, _, _, pz) = pavg(&results, |r| &r.z_fear);
    println!("  Fear={:.1}%  ExtFear={:.1}%  Neutral={:.1}%  Greed={:.1}%  ExtGreed={:.1}%  Z-Fear={:.1}%",
        pf, pe, pn, pg, peg, pz);

    // ── Portfolio summary ──────────────────────────────────────────────────────
    let (bwr, bpnl, bdd, _) = pavg(&results, |r| &r.baseline);
    let (efw, efp, efd, _)  = pavg(&results, |r| &r.extreme_fear);
    let (fw,  fp,  fd,  _)  = pavg(&results, |r| &r.fear);
    let (nw,  np,  nd,  _)  = pavg(&results, |r| &r.neutral);
    let (gw,  gp,  gd,  _)  = pavg(&results, |r| &r.greed);
    let (egw, egp, egd, _)  = pavg(&results, |r| &r.extreme_greed);
    let (zw,  zp,  zd,  _)  = pavg(&results, |r| &r.z_fear);

    println!();
    println!("  Portfolio Summary (18-coin avg, OOS test half):");
    println!("  {:>12}  {:>7}  {:>9}  {:>10}",
        "Regime", "Avg WR%", "Avg P&L%", "Avg MaxDD%");
    println!("  {}", "-".repeat(44));
    for (name, wr, pnl, dd) in [
        ("Baseline",    bwr, bpnl, bdd),
        ("Ext Fear",    efw, efp,  efd),
        ("Fear",        fw,  fp,   fd),
        ("Neutral",     nw,  np,   nd),
        ("Greed",       gw,  gp,   gd),
        ("Ext Greed",   egw, egp,  egd),
        ("Z-Fear",      zw,  zp,   zd),
    ] {
        println!("  {:>12}  {:>7.2}  {:>9.2}  {:>10.2}", name, wr, pnl, dd);
    }

    // ── Conclusion ─────────────────────────────────────────────────────────────
    println!();
    println!("  Breakeven WR ≈ 44% — shown for reference");
    let best = [("ExtFear", efw, efp), ("Fear", fw, fp), ("Neutral", nw, np),
                ("Greed", gw, gp), ("ExtGreed", egw, egp), ("ZFear", zw, zp)]
        .iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
        .map(|(name, wr, pnl)| (*name, *wr, *pnl))
        .unwrap();
    println!("  Best regime: {} — WR={:.2}% P&L={:.2}%", best.0, best.1, best.2);
    if best.2 > bpnl + 1.0 {
        println!("  → Sentiment regime filter shows improvement (+{:.2}pp vs baseline)", best.2 - bpnl);
    } else {
        println!("  → No regime materially beats baseline");
    }

    // ── Save ───────────────────────────────────────────────────────────────────
    let per_coin: Vec<Value> = results.iter().map(|r| json!({
        "coin":          r.coin,
        "strategy":      r.strategy,
        "baseline":      r.baseline,
        "extreme_fear":  r.extreme_fear,
        "fear":          r.fear,
        "neutral":       r.neutral,
        "greed":         r.greed,
        "extreme_greed": r.extreme_greed,
        "z_fear":        r.z_fear,
    })).collect();

    let summary = json!({
        "portfolio": {
            "baseline":      {"avg_wr": bwr, "avg_pnl": bpnl, "avg_dd": bdd},
            "extreme_fear":  {"avg_wr": efw, "avg_pnl": efp,  "avg_dd": efd},
            "fear":          {"avg_wr": fw,  "avg_pnl": fp,   "avg_dd": fd},
            "neutral":       {"avg_wr": nw,  "avg_pnl": np,   "avg_dd": nd},
            "greed":         {"avg_wr": gw,  "avg_pnl": gp,   "avg_dd": gd},
            "extreme_greed": {"avg_wr": egw, "avg_pnl": egp,  "avg_dd": egd},
            "z_fear":        {"avg_wr": zw,  "avg_pnl": zp,   "avg_dd": zd},
        },
        "regime_coverage_pct": {
            "fear": pf, "extreme_fear": pe, "neutral": pn,
            "greed": pg, "extreme_greed": peg, "z_fear": pz,
        },
        "note": "BTC RSI(14) used as market sentiment proxy (no F&G cache available).",
    });

    let out = json!({ "per_coin": per_coin, "summary": summary });
    let out_path = "archive/RUN21/run21_results.json";
    std::fs::create_dir_all("archive/RUN21").ok();
    std::fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!();
    println!("  Results saved to {}", out_path);
}
