/// RUN19 — Position Sizing Comparison
///
/// Hypothesis: Can Kelly/Half-Kelly sizing improve COINCLAW v13 portfolio returns
///             vs the implicit fixed-fraction currently in use?
///
/// Method:
///   - 18 coins, 15m 1-year data, COINCLAW v13 strategy per coin
///   - 50/50 chronological split
///   - Kelly fraction estimated on train half only (OOS)
///   - All 5 methods evaluated on test half
///   - Position sizing: fraction of equity allocated per trade
///   - Trade P&L % × fraction → equity change
///
/// Methods: Fixed 1% | Fixed 2% | Fixed 5% | Kelly (OOS) | Half-Kelly (OOS)

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::backtest::{backtest, STOP_LOSS};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::strategies::signals;

const FEE: f64 = 0.001; // 0.1% per side — applied to P&L
const ACCOUNT: f64 = 10_000.0;

// Max Kelly fraction cap — uncapped Kelly can recommend >100% which is ruin
const KELLY_CAP: f64 = 0.25;

// ── Position sizing fraction estimators ──────────────────────────────────────

fn kelly_fraction(pnls: &[f64]) -> f64 {
    if pnls.is_empty() { return 0.0; }
    let wins: Vec<f64> = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).map(|p| -p).collect();
    if wins.is_empty() || losses.is_empty() { return 0.0; }

    let p = wins.len() as f64 / pnls.len() as f64;  // win rate
    let q = 1.0 - p;
    let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
    let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;

    if avg_win <= 0.0 || avg_loss <= 0.0 { return 0.0; }
    let b = avg_win / avg_loss;  // win/loss ratio
    let kelly = (b * p - q) / b;
    kelly.max(0.0).min(KELLY_CAP)
}

// ── Sizing simulation ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SizingResult {
    fraction:     f64,
    total_return: f64,   // % over test period
    max_drawdown: f64,   // % peak-to-trough
    sharpe:       f64,   // trade-level, annualised
    calmar:       f64,   // return / max_dd
    n_trades:     usize,
    win_rate:     f64,
}

fn simulate_sizing(pnls: &[f64], fraction: f64) -> SizingResult {
    let n = pnls.len();
    if n == 0 {
        return SizingResult { fraction, total_return: 0.0, max_drawdown: 0.0,
                              sharpe: 0.0, calmar: 0.0, n_trades: 0, win_rate: 0.0 };
    }

    let mut equity = ACCOUNT;
    let mut peak = equity;
    let mut max_dd = 0.0f64;
    let mut equity_returns: Vec<f64> = Vec::with_capacity(n);

    for &pnl_pct in pnls {
        let prev = equity;
        // Apply fee: 0.1%/side on fraction of equity
        let fee_cost = fraction * equity * FEE * 2.0;
        let trade_pnl = fraction * equity * pnl_pct / 100.0 - fee_cost;
        equity += trade_pnl;
        equity = equity.max(0.001 * ACCOUNT); // ruin floor
        if equity > peak { peak = equity; }
        let dd = (peak - equity) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
        equity_returns.push((equity - prev) / prev);
    }

    let total_return = (equity - ACCOUNT) / ACCOUNT * 100.0;

    // Sharpe: annualise by sqrt(252 * 96) — 15m bars, 96 per day
    // But using trade-count annualisation is simpler and more standard here
    let mean_r = equity_returns.iter().sum::<f64>() / n as f64;
    let var_r = equity_returns.iter().map(|&r| (r - mean_r).powi(2)).sum::<f64>() / n as f64;
    let sharpe = if var_r > 0.0 { mean_r / var_r.sqrt() * (n as f64).sqrt() } else { 0.0 };

    let calmar = if max_dd > 0.0 { total_return / max_dd } else { total_return };

    let wins = pnls.iter().filter(|&&p| p > 0.0).count();
    let win_rate = wins as f64 / n as f64 * 100.0;

    SizingResult { fraction, total_return, max_drawdown: max_dd, sharpe, calmar, n_trades: n, win_rate }
}

// ── Per-coin processing ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CoinResult {
    coin:         String,
    strategy:     String,
    n_total:      usize,
    n_train:      usize,
    n_test:       usize,
    base_wr:      f64,
    kelly_frac:   f64,
    fixed1:       SizingResult,
    fixed2:       SizingResult,
    fixed5:       SizingResult,
    kelly:        SizingResult,
    half_kelly:   SizingResult,
}

fn process_coin(coin: &str, strategy: &str) -> CoinResult {
    let bars = load_ohlcv(coin);
    let (entry, exit) = signals(&bars, strategy);
    let (pnls, stats) = backtest(
        &bars.iter().map(|b| b.c).collect::<Vec<_>>(),
        &entry,
        &exit,
        STOP_LOSS,
    );

    let n_total = pnls.len();
    let split = n_total / 2;
    let train = &pnls[..split];
    let test  = &pnls[split..];

    // Estimate Kelly on train only (OOS methodology)
    let kelly_frac = kelly_fraction(train);
    let hk_frac    = kelly_frac / 2.0;

    let fixed1     = simulate_sizing(test, 0.01);
    let fixed2     = simulate_sizing(test, 0.02);
    let fixed5     = simulate_sizing(test, 0.05);
    let kelly      = simulate_sizing(test, kelly_frac);
    let half_kelly = simulate_sizing(test, hk_frac);

    CoinResult {
        coin:      coin.to_string(),
        strategy:  strategy.to_string(),
        n_total,
        n_train:   split,
        n_test:    test.len(),
        base_wr:   stats.win_rate,
        kelly_frac,
        fixed1, fixed2, fixed5, kelly, half_kelly,
    }
}

// ── Portfolio aggregation ─────────────────────────────────────────────────────

fn portfolio_stats(results: &[CoinResult], get: impl Fn(&CoinResult) -> &SizingResult) -> (f64, f64, f64) {
    // Equal-weight portfolio: average terminal return, root-mean-square max_dd, avg Calmar
    let n = results.len() as f64;
    let avg_ret = results.iter().map(|r| get(r).total_return).sum::<f64>() / n;
    let avg_dd  = results.iter().map(|r| get(r).max_drawdown).sum::<f64>() / n;
    let avg_cal = results.iter().map(|r| get(r).calmar).sum::<f64>() / n;
    (avg_ret, avg_dd, avg_cal)
}

// ── Main ──────────────────────────────────────────────────────────────────────

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN19 — Position Sizing Comparison (OOS Kelly vs Fixed Fraction)");
    println!("================================================================");
    println!("Data: 18 coins, 15m 1-year | Strategy: COINCLAW v13 per coin");
    println!("Split: 50/50 chronological | Kelly fit on train half only");
    println!("Methods: Fixed1% | Fixed2% | Fixed5% | Kelly(OOS) | HalfKelly(OOS)");
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

    // ── Per-coin table ────────────────────────────────────────────────────────
    println!("  {:<6}  {:<10}  {:>6}  {:>5}  {:>5}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}",
        "Coin", "Strategy", "Trades", "WR%", "Kelly", "F1%Ret", "F2%Ret", "F5%Ret", "KelRet", "HKRet");
    println!("  {}", "-".repeat(88));

    let mut sorted = results.clone();
    sorted.sort_by(|a, b| a.coin.cmp(&b.coin));

    for r in &sorted {
        println!("  {:<6}  {:<10}  {:>6}  {:>5.1}  {:>5.3}  {:>8.1}  {:>8.1}  {:>8.1}  {:>8.1}  {:>8.1}",
            r.coin, r.strategy, r.n_test, r.base_wr,
            r.kelly_frac,
            r.fixed1.total_return,
            r.fixed2.total_return,
            r.fixed5.total_return,
            r.kelly.total_return,
            r.half_kelly.total_return,
        );
    }

    // ── Max drawdown table ────────────────────────────────────────────────────
    println!();
    println!("  Max Drawdown by Method (OOS test half):");
    println!("  {:<6}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}",
        "Coin", "Fixed1%", "Fixed2%", "Fixed5%", "Kelly", "HalfKel");
    println!("  {}", "-".repeat(52));
    for r in &sorted {
        println!("  {:<6}  {:>8.1}  {:>8.1}  {:>8.1}  {:>8.1}  {:>8.1}",
            r.coin,
            r.fixed1.max_drawdown, r.fixed2.max_drawdown, r.fixed5.max_drawdown,
            r.kelly.max_drawdown, r.half_kelly.max_drawdown,
        );
    }

    // ── Portfolio summary ─────────────────────────────────────────────────────
    let (r1, d1, c1) = portfolio_stats(&results, |r| &r.fixed1);
    let (r2, d2, c2) = portfolio_stats(&results, |r| &r.fixed2);
    let (r5, d5, c5) = portfolio_stats(&results, |r| &r.fixed5);
    let (rk, dk, ck) = portfolio_stats(&results, |r| &r.kelly);
    let (rh, dh, ch) = portfolio_stats(&results, |r| &r.half_kelly);

    println!();
    println!("  Portfolio Summary (18-coin equal-weight average, OOS test half):");
    println!("  {:>12}  {:>10}  {:>10}  {:>10}",
        "Method", "Avg Ret%", "Avg MaxDD%", "Avg Calmar");
    println!("  {}", "-".repeat(48));
    for (name, ret, dd, cal) in [
        ("Fixed 1%",   r1, d1, c1),
        ("Fixed 2%",   r2, d2, c2),
        ("Fixed 5%",   r5, d5, c5),
        ("Kelly OOS",  rk, dk, ck),
        ("HalfKel OOS",rh, dh, ch),
    ] {
        println!("  {:>12}  {:>10.2}  {:>10.2}  {:>10.3}", name, ret, dd, cal);
    }

    // ── Kelly fraction summary ────────────────────────────────────────────────
    println!();
    println!("  Kelly fractions (estimated on train half):");
    let mut kf: Vec<f64> = sorted.iter().map(|r| r.kelly_frac).collect();
    kf.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_k = kf[kf.len() / 2];
    let mean_k = kf.iter().sum::<f64>() / kf.len() as f64;
    println!("  Min={:.3}  Median={:.3}  Mean={:.3}  Max={:.3}",
        kf.first().unwrap(), median_k, mean_k, kf.last().unwrap());
    println!("  (capped at {:.0}%)", KELLY_CAP * 100.0);

    // ── Best method ───────────────────────────────────────────────────────────
    let method_wins: [(&str, f64); 5] = [
        ("fixed_1pct",  r1 / d1.max(0.01)),
        ("fixed_2pct",  r2 / d2.max(0.01)),
        ("fixed_5pct",  r5 / d5.max(0.01)),
        ("kelly_oos",   rk / dk.max(0.01)),
        ("halfkelly",   rh / dh.max(0.01)),
    ];
    let best = method_wins.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    println!();
    println!("  Best risk-adjusted method: {} (return/dd ratio = {:.3})", best.0, best.1);

    // ── Save results ──────────────────────────────────────────────────────────
    let per_coin: Vec<Value> = sorted.iter().map(|r| {
        let sz = |s: &SizingResult| json!({
            "fraction": s.fraction,
            "total_return_pct": s.total_return,
            "max_drawdown_pct": s.max_drawdown,
            "sharpe": s.sharpe,
            "calmar": s.calmar,
            "n_trades": s.n_trades,
            "win_rate": s.win_rate,
        });
        json!({
            "coin": r.coin,
            "strategy": r.strategy,
            "n_total_trades": r.n_total,
            "n_train": r.n_train,
            "n_test": r.n_test,
            "full_wr": r.base_wr,
            "kelly_fraction_from_train": r.kelly_frac,
            "fixed_1pct": sz(&r.fixed1),
            "fixed_2pct": sz(&r.fixed2),
            "fixed_5pct": sz(&r.fixed5),
            "kelly_oos":  sz(&r.kelly),
            "halfkelly":  sz(&r.half_kelly),
        })
    }).collect();

    let summary = json!({
        "portfolio": {
            "fixed_1pct":  {"avg_return": r1, "avg_dd": d1, "avg_calmar": c1},
            "fixed_2pct":  {"avg_return": r2, "avg_dd": d2, "avg_calmar": c2},
            "fixed_5pct":  {"avg_return": r5, "avg_dd": d5, "avg_calmar": c5},
            "kelly_oos":   {"avg_return": rk, "avg_dd": dk, "avg_calmar": ck},
            "halfkelly":   {"avg_return": rh, "avg_dd": dh, "avg_calmar": ch},
        },
        "kelly_fractions": {
            "min":    kf.first().unwrap(),
            "median": median_k,
            "mean":   mean_k,
            "max":    kf.last().unwrap(),
            "cap":    KELLY_CAP,
        },
        "best_risk_adjusted": best.0,
    });

    let out = json!({ "per_coin": per_coin, "summary": summary });
    let out_path = "archive/RUN19/run19_results.json";
    std::fs::create_dir_all("archive/RUN19").ok();
    std::fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!();
    println!("  Results saved to {}", out_path);
}
