/// RUN24 — Ensemble Strategy Framework
///
/// Tests whether voting ensembles of top-3 COINCLAW strategies per coin
/// outperform the best single strategy on OOS data.
///
/// Fix over Python stub:
///   Python found top-3 strategies on FULL data (look-ahead bias).
///   Corrected: top-3 ranked by train PF (67%) only; tested on OOS (33%).
///
/// Strategies pooled (5 total): vwap_rev, bb_bounce, adr_rev, dual_rsi, mean_rev
///
/// Ensemble modes tested:
///   best_single  — top-1 by train PF
///   equal_vote   — entry if ≥2 of top-3 fire (majority, threshold=0.5)
///   pf_vote      — weighted by train PF, threshold=0.5
///   intersection — entry only if all top-3 fire (AND, strict)
///   union        — entry if any top-3 fires (OR, permissive)
///
/// Trade sim: SL=0.3% fee=0.1%/side slip=0.05%/side
/// Split: 67% train / 33% OOS test

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::loader::{load_ohlcv, Bar, COIN_STRATEGIES};
use crate::strategies::signals;

const SL:   f64 = 0.003;
const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;

const POOL: [&str; 5] = ["vwap_rev", "bb_bounce", "adr_rev", "dual_rsi", "mean_rev"];

// ── Trade simulation ──────────────────────────────────────────────────────────
fn sim(close: &[f64], entry: &[bool], exit: &[bool]) -> (usize, f64, f64, f64) {
    // (n_trades, win_rate%, profit_factor, total_pnl%)
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
                let pnl = -SL * (1.0 + SLIP) * 100.0 - FEE * 200.0;
                Some(pnl)
            } else if exit[i] {
                let pnl = (close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0;
                Some(pnl)
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

// ── Ensemble signal generators ────────────────────────────────────────────────

/// Equal-weight majority vote: entry if ≥ ceil(n/2) strategies fire
fn ensemble_equal(signals_list: &[(Vec<bool>, Vec<bool>)], n: usize, threshold: f64)
    -> (Vec<bool>, Vec<bool>)
{
    let k = signals_list.len();
    let w = 1.0 / k as f64;
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 0..n {
        let es: f64 = signals_list.iter().map(|(e, _)| if e[i] { w } else { 0.0 }).sum();
        let xs: f64 = signals_list.iter().map(|(_, x)| if x[i] { w } else { 0.0 }).sum();
        entry[i] = es >= threshold;
        exit[i]  = xs >= threshold;
    }
    (entry, exit)
}

/// PF-weighted vote: entry if weighted_sum ≥ threshold (0.5)
fn ensemble_pf_vote(signals_list: &[(Vec<bool>, Vec<bool>)], weights: &[f64], n: usize)
    -> (Vec<bool>, Vec<bool>)
{
    let total_w: f64 = weights.iter().sum();
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 0..n {
        let es: f64 = signals_list.iter().zip(weights)
            .map(|((e, _), &w)| if e[i] { w } else { 0.0 }).sum::<f64>() / total_w;
        let xs: f64 = signals_list.iter().zip(weights)
            .map(|((_, x), &w)| if x[i] { w } else { 0.0 }).sum::<f64>() / total_w;
        entry[i] = es >= 0.5;
        exit[i]  = xs >= 0.5;
    }
    (entry, exit)
}

/// Intersection: entry only if ALL strategies agree
fn ensemble_and(signals_list: &[(Vec<bool>, Vec<bool>)], n: usize) -> (Vec<bool>, Vec<bool>) {
    let mut entry = vec![true; n];
    let mut exit  = vec![true; n];
    for (e, x) in signals_list {
        for i in 0..n { entry[i] = entry[i] && e[i]; exit[i] = exit[i] && x[i]; }
    }
    (entry, exit)
}

/// Union: entry if ANY strategy fires
fn ensemble_or(signals_list: &[(Vec<bool>, Vec<bool>)], n: usize) -> (Vec<bool>, Vec<bool>) {
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for (e, x) in signals_list {
        for i in 0..n { entry[i] = entry[i] || e[i]; exit[i] = exit[i] || x[i]; }
    }
    (entry, exit)
}

// ── Per-coin processing ───────────────────────────────────────────────────────

fn process_coin(coin: &str) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let split = n * 67 / 100;

    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();

    // Generate signals for all 5 pooled strategies on FULL data
    // (signals use rolling lookback; we index-slice for train/test eval)
    let all_sigs: Vec<(Vec<bool>, Vec<bool>)> = POOL.iter()
        .map(|&s| signals(&bars, s))
        .collect();

    // Rank strategies by PF on TRAIN half (no look-ahead)
    let mut train_scores: Vec<(usize, f64, usize)> = all_sigs.iter().enumerate()
        .map(|(i, (e, x))| {
            let (nt, _, pf, _) = sim(&close[..split], &e[..split], &x[..split]);
            (i, pf, nt)
        })
        .filter(|(_, _, nt)| *nt >= 5)
        .collect();
    train_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Take top-3 (or fewer if not enough valid strategies)
    let top3: Vec<(usize, f64)> = train_scores.iter().take(3).map(|&(i, pf, _)| (i, pf)).collect();

    if top3.is_empty() {
        return json!({ "coin": coin, "error": "no valid train strategies" });
    }

    let top3_sigs: Vec<&(Vec<bool>, Vec<bool>)> = top3.iter().map(|(i, _)| &all_sigs[*i]).collect();
    let top3_names: Vec<&str> = top3.iter().map(|(i, _)| POOL[*i]).collect();
    let top3_pfs: Vec<f64>    = top3.iter().map(|(_, pf)| *pf).collect();

    let test_close = &close[split..];
    let test_n = test_close.len();

    // Slice test-half signals
    let top3_test: Vec<(Vec<bool>, Vec<bool>)> = top3_sigs.iter().map(|(e, x)| {
        (e[split..].to_vec(), x[split..].to_vec())
    }).collect();

    // best_single: top-1
    let (bs_t, bs_wr, bs_pf, bs_pnl) = {
        let (e, x) = &top3_test[0];
        sim(test_close, e, x)
    };

    // equal_vote majority (threshold 0.5 = ≥ 2 of 3 for 3 strategies)
    let (ev_t, ev_wr, ev_pf, ev_pnl) = {
        let (e, x) = ensemble_equal(&top3_test, test_n, 0.5);
        sim(test_close, &e, &x)
    };

    // pf_vote: weighted by train PF
    let (pv_t, pv_wr, pv_pf, pv_pnl) = {
        let (e, x) = ensemble_pf_vote(&top3_test, &top3_pfs, test_n);
        sim(test_close, &e, &x)
    };

    // intersection: all must agree
    let (ai_t, ai_wr, ai_pf, ai_pnl) = {
        let (e, x) = ensemble_and(&top3_test, test_n);
        sim(test_close, &e, &x)
    };

    // union: any fires
    let (au_t, au_wr, au_pf, au_pnl) = {
        let (e, x) = ensemble_or(&top3_test, test_n);
        sim(test_close, &e, &x)
    };

    // determine winner
    let candidates = [
        ("best_single", bs_pf, bs_t),
        ("equal_vote",  ev_pf, ev_t),
        ("pf_vote",     pv_pf, pv_t),
        ("intersection",ai_pf, ai_t),
        ("union",       au_pf, au_t),
    ];
    let best = candidates.iter()
        .filter(|&&(_, _, t)| t >= 5)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|&(name, _, _)| name)
        .unwrap_or("best_single");

    json!({
        "coin": coin,
        "top3_strategies": top3_names,
        "top3_train_pf": top3_pfs,
        "best_single":   { "t": bs_t, "wr": bs_wr, "pf": bs_pf, "pnl": bs_pnl },
        "equal_vote":    { "t": ev_t, "wr": ev_wr, "pf": ev_pf, "pnl": ev_pnl },
        "pf_vote":       { "t": pv_t, "wr": pv_wr, "pf": pv_pf, "pnl": pv_pnl },
        "intersection":  { "t": ai_t, "wr": ai_wr, "pf": ai_pf, "pnl": ai_pnl },
        "union":         { "t": au_t, "wr": au_wr, "pf": au_pf, "pnl": au_pnl },
        "best_method": best,
        "ensemble_beats_single": best != "best_single",
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN24 — Ensemble Strategy Framework");
    println!("================================================================");
    println!("Fix: top-3 strategies ranked on TRAIN (67%) only — no look-ahead.");
    println!("Pool: vwap_rev | bb_bounce | adr_rev | dual_rsi | mean_rev");
    println!("Modes: best_single | equal_vote | pf_vote | intersection | union");
    println!("Trade: SL=0.3% fee=0.1%/side slip=0.05%/side | Breakeven WR ≈ 44%");
    println!();

    let coins: Vec<&str> = COIN_STRATEGIES.iter().map(|(c, _)| *c).collect();

    let results: Vec<Value> = coins.par_iter()
        .map(|&coin| process_coin(coin))
        .collect();

    // Print results table
    println!("  {:6}  {:>3}  {:>5}  {:>5}  {:>7}  | {:>3}  {:>5}  {:>5}  {:>7}  | {:>3}  {:>5}  {:>5}  {:>7}  | {:>12}",
        "Coin", "t", "WR%", "PF", "P&L%",
        "t", "WR%", "PF", "P&L%",
        "t", "WR%", "PF", "P&L%",
        "Best");
    println!("  {:6}  {:>17}  | {:>17} EqVote | {:>17} PFVote | {:>12}",
        "", "BestSingle", "", "", "");
    println!("  {}", "-".repeat(110));

    let mut wins_single = 0usize;
    let mut wins_equal  = 0usize;
    let mut wins_pf     = 0usize;
    let mut wins_and    = 0usize;
    let mut wins_or     = 0usize;
    let mut n_valid = 0usize;

    let mut sum_bs_pf = 0.0f64;
    let mut sum_ev_pf = 0.0f64;
    let mut sum_pv_pf = 0.0f64;

    for r in &results {
        if r.get("error").is_some() { continue; }
        let coin = r["coin"].as_str().unwrap_or("?");
        let bs = &r["best_single"];
        let ev = &r["equal_vote"];
        let pv = &r["pf_vote"];
        let best = r["best_method"].as_str().unwrap_or("?");

        let bs_t   = bs["t"].as_u64().unwrap_or(0);
        let bs_wr  = bs["wr"].as_f64().unwrap_or(0.0);
        let bs_pf  = bs["pf"].as_f64().unwrap_or(0.0);
        let bs_pnl = bs["pnl"].as_f64().unwrap_or(0.0);
        let ev_t   = ev["t"].as_u64().unwrap_or(0);
        let ev_wr  = ev["wr"].as_f64().unwrap_or(0.0);
        let ev_pf  = ev["pf"].as_f64().unwrap_or(0.0);
        let ev_pnl = ev["pnl"].as_f64().unwrap_or(0.0);
        let pv_t   = pv["t"].as_u64().unwrap_or(0);
        let pv_wr  = pv["wr"].as_f64().unwrap_or(0.0);
        let pv_pf  = pv["pf"].as_f64().unwrap_or(0.0);
        let pv_pnl = pv["pnl"].as_f64().unwrap_or(0.0);

        println!("  {:6}  {:>3}  {:>5.1}  {:>5.2}  {:>+7.1}  | {:>3}  {:>5.1}  {:>5.2}  {:>+7.1}  | {:>3}  {:>5.1}  {:>5.2}  {:>+7.1}  | {:>12}",
            coin,
            bs_t, bs_wr, bs_pf, bs_pnl,
            ev_t, ev_wr, ev_pf, ev_pnl,
            pv_t, pv_wr, pv_pf, pv_pnl,
            best);

        sum_bs_pf += bs_pf;
        sum_ev_pf += ev_pf;
        sum_pv_pf += pv_pf;
        n_valid += 1;

        match best {
            "best_single"  => wins_single += 1,
            "equal_vote"   => wins_equal  += 1,
            "pf_vote"      => wins_pf     += 1,
            "intersection" => wins_and    += 1,
            "union"        => wins_or     += 1,
            _ => {}
        }
    }

    println!();
    if n_valid > 0 {
        println!("  Avg PF — BestSingle: {:.3}  EqualVote: {:.3}  PFVote: {:.3}",
            sum_bs_pf / n_valid as f64,
            sum_ev_pf / n_valid as f64,
            sum_pv_pf / n_valid as f64);
        println!("  Method wins (of {}): best_single={} equal_vote={} pf_vote={} intersection={} union={}",
            n_valid, wins_single, wins_equal, wins_pf, wins_and, wins_or);
        println!("  Ensemble beats best_single: {}/{}", n_valid - wins_single, n_valid);
    }

    // OOS WR summary
    println!();
    println!("  OOS WR% (breakeven = 44%):");
    println!("  {:6}  {:>12}  {:>12}  {:>12}  {:>12}  {:>12}",
        "Coin", "BestSingle", "EqVote", "PFVote", "AND", "OR");
    println!("  {}", "-".repeat(78));
    let mut n_above_44 = 0usize;
    let mut total_cells = 0usize;
    for r in &results {
        if r.get("error").is_some() { continue; }
        let coin = r["coin"].as_str().unwrap_or("?");
        let get_wr = |key: &str| r[key]["wr"].as_f64().unwrap_or(0.0);
        let get_t  = |key: &str| r[key]["t"].as_u64().unwrap_or(0);
        let bs_wr = get_wr("best_single");
        let ev_wr = get_wr("equal_vote");
        let pv_wr = get_wr("pf_vote");
        let ai_wr = get_wr("intersection");
        let au_wr = get_wr("union");
        println!("  {:6}  {:>12.1}  {:>12.1}  {:>12.1}  {:>12.1}  {:>12.1}",
            coin, bs_wr, ev_wr, pv_wr, ai_wr, au_wr);
        for (&wr, mode) in [bs_wr, ev_wr, pv_wr, ai_wr, au_wr].iter()
            .zip(["best_single","equal_vote","pf_vote","intersection","union"])
        {
            let t = get_t(mode);
            if wr > 44.0 && t >= 10 { n_above_44 += 1; }
            if t >= 10 { total_cells += 1; }
        }
    }
    println!();
    println!("  WR > 44% with ≥10 trades: {}/{}", n_above_44, total_cells);

    // Save JSON
    let out_dir = "archive/RUN24";
    std::fs::create_dir_all(out_dir).ok();
    let out_path = format!("{}/run24_results.json", out_dir);
    let output = serde_json::json!({
        "run": "RUN24",
        "description": "Ensemble strategy framework — voting combinations of top-3 per coin",
        "coins": results,
        "summary": {
            "n_valid": n_valid,
            "avg_pf_best_single": sum_bs_pf / n_valid.max(1) as f64,
            "avg_pf_equal_vote":  sum_ev_pf / n_valid.max(1) as f64,
            "avg_pf_pf_vote":     sum_pv_pf / n_valid.max(1) as f64,
            "method_wins": {
                "best_single": wins_single,
                "equal_vote":  wins_equal,
                "pf_vote":     wins_pf,
                "intersection": wins_and,
                "union":       wins_or,
            },
            "ensemble_beats_single": n_valid - wins_single,
            "wr_above_44_of_total": format!("{}/{}", n_above_44, total_cells),
        }
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("\n  Results saved to {}", out_path);
}
