/// RUN17.3 — COINCLAW Portfolio Monte Carlo Simulation
///
/// Builds a bar-aligned daily return series across all 18 COINCLAW coins,
/// then runs 100k block-bootstrap simulations to get portfolio-level
/// equity curve distributions. Block size = 20 bars (≈ 5 trading hours
/// on 15m data) to preserve intra-day autocorrelation.
///
/// Output: run17_3_results.json

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::backtest::{backtest, STOP_LOSS};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::mc::{block_bootstrap, percentiles};
use crate::strategies::signals;

const N_SIMS: usize = 100_000;
const BLOCK_SIZE: usize = 20; // 20 × 15m ≈ 5 hours

#[derive(Debug, Serialize)]
struct PortfolioStats {
    /// Final portfolio return percentiles across 100k simulations
    p5_return:  f64,
    p50_return: f64,
    p95_return: f64,
    mean_return: f64,
    /// Fraction of simulations that end positive
    prob_profit: f64,
    /// p5 / p50 / p95 of per-simulation max drawdown
    p5_max_dd:  f64,
    p50_max_dd: f64,
    p95_max_dd: f64,
}

#[derive(Debug, Serialize)]
struct CoinContribution {
    coin: String,
    strategy: String,
    n_trades: usize,
    total_pnl: f64,
    /// Number of bars with an open position (active coverage)
    active_bars: usize,
}

#[derive(Debug, Serialize)]
struct Output {
    n_simulations: usize,
    block_size: usize,
    n_coins: usize,
    /// Length of the underlying daily-return series (bars)
    series_length: usize,
    portfolio: PortfolioStats,
    per_coin: Vec<CoinContribution>,
}

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN17.3 — COINCLAW Portfolio Monte Carlo Simulation");
    println!("================================================================");
    println!(
        "Coins: 18 | Simulations: {} | Block size: {} bars (15m)",
        N_SIMS, BLOCK_SIZE
    );
    println!();

    let out_path = "run17_3_results.json";
    if std::path::Path::new(out_path).exists() {
        println!("Results already exist at {}. Delete to re-run.", out_path);
        return;
    }

    // ── Step 1: load all coins, run backtests, build per-bar P&L arrays ──────
    // Each coin produces a Vec<f64> of length = bars.len() with pnl[i] = percent
    // gain/loss on bar i if we were in a trade, else 0.0.
    // We run this in parallel, then align all series to the same length.

    struct CoinData {
        coin: &'static str,
        strategy: &'static str,
        bar_pnl: Vec<f64>,     // per-bar fractional P&L (0 when flat)
        n_trades: usize,
        total_pnl: f64,
        active_bars: usize,
        n_bars: usize,
    }

    let coin_data: Vec<CoinData> = COIN_STRATEGIES
        .par_iter()
        .filter_map(|&(coin, strat)| {
            if shutdown.load(Ordering::SeqCst) { return None; }

            let bars = load_ohlcv(coin);
            let n_bars = bars.len();
            let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
            let (entry, exit) = signals(&bars, strat);
            let (pnls, stats) = backtest(&close, &entry, &exit, STOP_LOSS);

            if pnls.is_empty() {
                eprintln!("  SKIP {}: no trades", coin);
                return None;
            }

            // Build bar-aligned P&L: replay the backtest to mark each bar
            // with the fractional return while a position is open.
            let mut bar_pnl = vec![0.0f64; n_bars];
            let mut in_pos = false;
            let mut entry_price = 0.0f64;
            const SLIP: f64 = 0.0005;

            for i in 0..n_bars {
                if in_pos {
                    let pnl_frac = (close[i] - entry_price) / entry_price;
                    if pnl_frac <= -STOP_LOSS {
                        let ep = entry_price * (1.0 - STOP_LOSS * (1.0 + SLIP));
                        bar_pnl[i] = (ep - entry_price) / entry_price;
                        in_pos = false;
                    } else if exit[i] {
                        let ep = close[i] * (1.0 - SLIP);
                        bar_pnl[i] = (ep - entry_price) / entry_price;
                        in_pos = false;
                    } else {
                        // Mark-to-market: unrealised return for this bar
                        bar_pnl[i] = (close[i] - close[i.saturating_sub(1)]) / close[i.saturating_sub(1)];
                    }
                } else if entry[i] {
                    entry_price = close[i] * (1.0 + SLIP);
                    in_pos = true;
                    // First bar in trade: entry slippage cost
                    bar_pnl[i] = -SLIP;
                }
            }

            let active_bars = bar_pnl.iter().filter(|&&p| p != 0.0).count();

            eprintln!(
                "  {:6} {:10}  trades={:4}  total_pnl={:+.1}%  active_bars={}",
                coin, strat, stats.n_trades, stats.total_pnl, active_bars
            );

            Some(CoinData {
                coin,
                strategy: strat,
                bar_pnl,
                n_trades: stats.n_trades,
                total_pnl: stats.total_pnl,
                active_bars,
                n_bars,
            })
        })
        .collect();

    if coin_data.is_empty() {
        eprintln!("No coins loaded — aborting.");
        return;
    }

    // ── Step 2: align all series to shortest length, build portfolio returns ─
    let min_len = coin_data.iter().map(|c| c.n_bars).min().unwrap_or(0);
    let n_coins = coin_data.len() as f64;

    println!("Loaded {} coins, aligning to {} bars.", coin_data.len(), min_len);

    // Portfolio return at bar i = average of all coin bar P&Ls
    let mut portfolio_returns = vec![0.0f64; min_len];
    for cd in &coin_data {
        for i in 0..min_len {
            portfolio_returns[i] += cd.bar_pnl[i] / n_coins;
        }
    }

    println!(
        "Portfolio daily series built. Running {} block-bootstrap simulations (block={} bars)...",
        N_SIMS, BLOCK_SIZE
    );
    println!();

    // ── Step 3: block bootstrap ───────────────────────────────────────────────
    let sim_returns = block_bootstrap(&portfolio_returns, N_SIMS, BLOCK_SIZE);

    // ── Step 4: compute portfolio stats from simulation results ───────────────
    let total = sim_returns.len() as f64;
    let prob_profit = sim_returns.iter().filter(|&&r| r > 0.0).count() as f64 / total;

    let (p5_ret, p50_ret, p95_ret, mean_ret) = percentiles(sim_returns.clone());

    // Compute max drawdown for each simulation path
    // block_bootstrap returns only final return, not equity curve.
    // We recompute the DD from a re-run using the same seeds but tracking
    // the equity path. For efficiency, sample 10k paths for DD distribution.
    let dd_samples = {
        use rand::prelude::*;
        use rand::rngs::SmallRng;
        let n_dd = N_SIMS.min(10_000);
        let n = portfolio_returns.len();
        let n_blocks = (n / BLOCK_SIZE) + 1;

        (0..n_dd)
            .into_par_iter()
            .map(|seed| {
                let mut rng = SmallRng::seed_from_u64(seed as u64 + 88888);
                let mut path = vec![0.0f64; n];
                let mut pos = 0;
                while pos < n {
                    let max_start = n.saturating_sub(BLOCK_SIZE).max(1);
                    let block_start = rng.gen_range(0..max_start);
                    let block_end = (block_start + BLOCK_SIZE).min(n);
                    let take = (n - pos).min(block_end - block_start);
                    path[pos..pos + take]
                        .copy_from_slice(&portfolio_returns[block_start..block_start + take]);
                    pos += take;
                }
                // Max drawdown of this path
                let mut equity = 1.0f64;
                let mut peak = 1.0f64;
                let mut max_dd = 0.0f64;
                for &r in &path {
                    equity *= 1.0 + r;
                    if equity > peak { peak = equity; }
                    let dd = (peak - equity) / peak;
                    if dd > max_dd { max_dd = dd; }
                }
                max_dd * 100.0
            })
            .collect::<Vec<f64>>()
    };

    let (p5_dd, p50_dd, p95_dd, _) = percentiles(dd_samples);

    let portfolio = PortfolioStats {
        p5_return:   p5_ret  * 100.0,
        p50_return:  p50_ret * 100.0,
        p95_return:  p95_ret * 100.0,
        mean_return: mean_ret * 100.0,
        prob_profit,
        p5_max_dd:  p5_dd,
        p50_max_dd: p50_dd,
        p95_max_dd: p95_dd,
    };

    // ── Step 5: print summary ─────────────────────────────────────────────────
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PORTFOLIO MONTE CARLO RESULTS ({} simulations)", N_SIMS);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Return distribution (% over full data window):");
    println!("    p5:   {:+.1}%", portfolio.p5_return);
    println!("    p50:  {:+.1}%", portfolio.p50_return);
    println!("    p95:  {:+.1}%", portfolio.p95_return);
    println!("    mean: {:+.1}%", portfolio.mean_return);
    println!("  Probability of profit: {:.1}%", portfolio.prob_profit * 100.0);
    println!();
    println!("  Max Drawdown distribution:");
    println!("    p5:  {:+.1}%  (best-case DD)", portfolio.p5_max_dd);
    println!("    p50: {:+.1}%  (typical DD)", portfolio.p50_max_dd);
    println!("    p95: {:+.1}%  (worst-case DD)", portfolio.p95_max_dd);

    // ── Step 6: build per-coin contribution table ─────────────────────────────
    let per_coin: Vec<CoinContribution> = coin_data
        .iter()
        .map(|cd| CoinContribution {
            coin: cd.coin.to_string(),
            strategy: cd.strategy.to_string(),
            n_trades: cd.n_trades,
            total_pnl: cd.total_pnl,
            active_bars: cd.active_bars,
        })
        .collect();

    println!();
    println!("  Per-coin contributions:");
    for c in &per_coin {
        println!(
            "    {:6} {:10}  trades={:4}  pnl={:+.1}%  active_bars={}",
            c.coin, c.strategy, c.n_trades, c.total_pnl, c.active_bars
        );
    }

    let output = Output {
        n_simulations: N_SIMS,
        block_size: BLOCK_SIZE,
        n_coins: per_coin.len(),
        series_length: min_len,
        portfolio,
        per_coin,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON serialise");
    std::fs::write(out_path, json).expect("write results");
    println!();
    println!("Results saved to {}", out_path);
}
