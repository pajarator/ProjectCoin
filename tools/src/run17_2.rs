/// RUN17.2 — COINCLAW Primary Strategies MC Validation
///
/// For each of the 18 COINCLAW coins, backtests the primary long strategy
/// then runs 100k MC simulations to determine if the strategy's P&L is
/// statistically robust or a lucky streak.
///
/// Uses Rayon for full parallelism (18 coins simultaneously).
/// Output: run17_2_results.json

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::backtest::{backtest, STOP_LOSS};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::mc::{monte_carlo, McResult};
use crate::strategies::signals;

const N_SIMS: usize = 100_000;
const MIN_TRADES: usize = 30;

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    strategy: String,
    actual: ActualStats,
    mc: McResult,
    flagged: bool,        // 5th-percentile PF < 1.0
    fragile: bool,        // prob_profit < 0.80
}

#[derive(Debug, Serialize)]
struct ActualStats {
    n_trades: usize,
    win_rate: f64,
    profit_factor: f64,
    sharpe: f64,
    max_drawdown: f64,
    total_pnl: f64,
}

#[derive(Debug, Serialize)]
struct Summary {
    total_coins: usize,
    flagged_count: usize,       // 5th-pct PF < 1.0
    fragile_count: usize,       // prob_profit < 80%
    robust_count: usize,        // not flagged AND not fragile
    avg_prob_profit: f64,
    avg_actual_pf: f64,
    flagged_coins: Vec<String>,
    fragile_coins: Vec<String>,
    robust_coins: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Output {
    n_simulations: usize,
    per_coin: Vec<CoinResult>,
    summary: Summary,
}

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN17.2 — COINCLAW Primary Strategies Monte Carlo Validation");
    println!("================================================================");
    println!("Coins: 18 | Simulations: {} | Strategy: primary long per coin", N_SIMS);
    println!("Running all coins in parallel on {} CPUs...", rayon::current_num_threads());
    println!();

    // Check for existing results to skip
    let out_path = "run17_2_results.json";
    if std::path::Path::new(out_path).exists() {
        println!("Results already exist at {}. Delete to re-run.", out_path);
        return;
    }

    let per_coin: Vec<CoinResult> = COIN_STRATEGIES
        .par_iter()
        .filter_map(|&(coin, strat)| {
            if shutdown.load(Ordering::SeqCst) { return None; }

            let bars = load_ohlcv(coin);
            let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
            let (entry, exit) = signals(&bars, strat);
            let (pnls, stats) = backtest(&close, &entry, &exit, STOP_LOSS);

            if pnls.len() < MIN_TRADES {
                eprintln!("  SKIP {}/{}: only {} trades", coin, strat, pnls.len());
                return None;
            }

            let mc = monte_carlo(&pnls, N_SIMS);
            let flagged = mc.p5_pf < 1.0;
            let fragile = mc.prob_profit < 0.80;

            let verdict = if flagged { "FLAGGED" } else if fragile { "FRAGILE" } else { "ROBUST" };
            println!(
                "  {:6} {:10}  trades={:4}  WR={:5.1}%  PF={:.2}  p5_PF={:.2}  prob_profit={:.1}%  [{}]",
                coin, strat, stats.n_trades, stats.win_rate, stats.profit_factor,
                mc.p5_pf, mc.prob_profit * 100.0, verdict
            );

            Some(CoinResult {
                coin: coin.to_string(),
                strategy: strat.to_string(),
                actual: ActualStats {
                    n_trades: stats.n_trades,
                    win_rate: stats.win_rate,
                    profit_factor: stats.profit_factor,
                    sharpe: stats.sharpe,
                    max_drawdown: stats.max_drawdown,
                    total_pnl: stats.total_pnl,
                },
                mc,
                flagged,
                fragile,
            })
        })
        .collect();

    // Aggregate summary
    let flagged: Vec<String> = per_coin.iter().filter(|r| r.flagged).map(|r| r.coin.clone()).collect();
    let fragile: Vec<String> = per_coin.iter().filter(|r| !r.flagged && r.fragile).map(|r| r.coin.clone()).collect();
    let robust:  Vec<String> = per_coin.iter().filter(|r| !r.flagged && !r.fragile).map(|r| r.coin.clone()).collect();
    let avg_prob = per_coin.iter().map(|r| r.mc.prob_profit).sum::<f64>() / per_coin.len() as f64;
    let avg_pf   = per_coin.iter().map(|r| r.actual.profit_factor).sum::<f64>() / per_coin.len() as f64;

    let summary = Summary {
        total_coins: per_coin.len(),
        flagged_count: flagged.len(),
        fragile_count: fragile.len(),
        robust_count:  robust.len(),
        avg_prob_profit: avg_prob,
        avg_actual_pf: avg_pf,
        flagged_coins: flagged,
        fragile_coins: fragile,
        robust_coins:  robust,
    };

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY");
    println!("  Total coins:     {}", summary.total_coins);
    println!("  Robust (p5>1.0, prob>80%): {}", summary.robust_count);
    println!("  Fragile (prob_profit 60-80%): {}", summary.fragile_count);
    println!("  Flagged (p5_PF < 1.0):  {}", summary.flagged_count);
    println!("  Avg prob_profit:    {:.1}%", summary.avg_prob_profit * 100.0);
    println!("  Avg actual PF:      {:.2}", summary.avg_actual_pf);
    if !summary.flagged_coins.is_empty() {
        println!("  Flagged: {:?}", summary.flagged_coins);
    }
    if !summary.fragile_coins.is_empty() {
        println!("  Fragile: {:?}", summary.fragile_coins);
    }
    println!("  Robust: {:?}", summary.robust_coins);

    let output = Output { n_simulations: N_SIMS, per_coin, summary };
    let json = serde_json::to_string_pretty(&output).expect("JSON serialise");
    std::fs::write(out_path, json).expect("write results");
    println!();
    println!("Results saved to {}", out_path);
}
