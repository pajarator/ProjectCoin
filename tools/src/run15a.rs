/// RUN15a (COINCLAW-corrected) — Bayesian Entry Gate vs Primary Strategies
///
/// Tests whether a Bayesian probability gate on trade entries can improve
/// COINCLAW's primary long strategies. Per-coin model: train on first 50%
/// of bars, test on last 50% (strict out-of-sample).
///
/// Three modes on test data:
///   binary   — COINCLAW primary entry signals unchanged (baseline)
///   bay60    — primary entry AND P(win | z_bin, rsi_bin) > 60%
///   bay55    — primary entry AND P(win | z_bin, rsi_bin) > 55%
///
/// Fixes vs original RUN15a:
///   ✓ Uses actual per-coin COINCLAW strategies (vwap_rev / bb_bounce / etc.)
///   ✓ Trade outcomes from real backtest (stop-loss + signal exit), not 4-bar lookahead
///   ✓ No artificial leverage or fixed trade_size
///   ✓ Bayesian trained on actual trade outcomes, not next-bar direction
///   ✓ VWAP omitted — Bayesian uses z20+rsi14 which are the dominant predictors
///   ✓ Dead code removed (vwap < vwap bug, etc.)
///
/// Output: run15a_results.json

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::backtest::{backtest, BacktestStats, SLIPPAGE, STOP_LOSS};
use crate::indicators::{rsi, z_score};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::strategies::signals;

// Thresholds to gate on
const THRESH_60: f64 = 0.60;
const THRESH_55: f64 = 0.55;
// Minimum observations in a cell before we trust the posterior (not just the prior)
const MIN_OBS: f64 = 5.0;

// ── Bayesian Beta model per (z_bin, rsi_bin) cell ───────────────────────────

#[derive(Debug, Clone)]
struct BetaCell {
    alpha: f64, // wins + 1  (Beta(1,1) prior = uniform)
    beta:  f64, // losses + 1
}

impl BetaCell {
    fn new() -> Self { Self { alpha: 1.0, beta: 1.0 } }

    fn observe(&mut self, win: bool) {
        if win { self.alpha += 1.0; } else { self.beta += 1.0; }
    }

    /// Posterior mean. Returns prior (0.5) until MIN_OBS trades observed.
    fn prob_win(&self) -> f64 {
        let obs = self.alpha + self.beta - 2.0; // subtract the prior counts
        if obs < MIN_OBS { return 0.5; }
        self.alpha / (self.alpha + self.beta)
    }
}

struct BayesModel {
    cells: HashMap<(i32, i32), BetaCell>,
}

impl BayesModel {
    fn new() -> Self { Self { cells: HashMap::new() } }

    fn bin_z(z: f64) -> i32 { (z / 0.5).floor() as i32 }
    fn bin_rsi(r: f64) -> i32 { (r / 10.0).floor() as i32 }

    fn learn(&mut self, z: f64, r: f64, win: bool) {
        if z.is_nan() || r.is_nan() { return; }
        self.cells
            .entry((Self::bin_z(z), Self::bin_rsi(r)))
            .or_insert_with(BetaCell::new)
            .observe(win);
    }

    fn prob_win(&self, z: f64, r: f64) -> f64 {
        if z.is_nan() || r.is_nan() { return 0.5; }
        self.cells
            .get(&(Self::bin_z(z), Self::bin_rsi(r)))
            .map(|c| c.prob_win())
            .unwrap_or(0.5)
    }
}

// ── Trade-entry collector ────────────────────────────────────────────────────
/// Mirrors the exact backtest logic in backtest.rs and returns (entry_bar, pnl)
/// for each completed trade, so we can correlate entry conditions with outcomes.
fn collect_trades(
    close: &[f64],
    entry: &[bool],
    exit: &[bool],
) -> Vec<(usize, f64)> {
    let n = close.len();
    let mut out = Vec::new();
    let mut in_pos = false;
    let mut entry_price = 0.0f64;
    let mut entry_bar = 0usize;

    for i in 0..n {
        if in_pos {
            let pnl_frac = (close[i] - entry_price) / entry_price;
            if pnl_frac <= -STOP_LOSS {
                let ep = entry_price * (1.0 - STOP_LOSS * (1.0 + SLIPPAGE));
                let pnl = (ep - entry_price) / entry_price * 100.0;
                out.push((entry_bar, pnl));
                in_pos = false;
            } else if exit[i] {
                let ep = close[i] * (1.0 - SLIPPAGE);
                let pnl = (ep - entry_price) / entry_price * 100.0;
                out.push((entry_bar, pnl));
                in_pos = false;
            }
        } else if entry[i] {
            entry_price = close[i] * (1.0 + SLIPPAGE);
            entry_bar = i;
            in_pos = true;
        }
    }
    // open position at end
    if in_pos {
        let ep = *close.last().unwrap();
        let pnl = (ep - entry_price) / entry_price * 100.0;
        out.push((entry_bar, pnl));
    }
    out
}

// ── Result structs ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
struct StratResult {
    n_trades: usize,
    win_rate: f64,
    profit_factor: f64,
    sharpe: f64,
    max_drawdown: f64,
    total_pnl: f64,
}

impl From<BacktestStats> for StratResult {
    fn from(s: BacktestStats) -> Self {
        Self {
            n_trades: s.n_trades,
            win_rate: s.win_rate,
            profit_factor: s.profit_factor,
            sharpe: s.sharpe,
            max_drawdown: s.max_drawdown,
            total_pnl: s.total_pnl,
        }
    }
}

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    strategy: String,
    train_trades: usize,
    /// cells with ≥ MIN_OBS observations
    model_cells: usize,
    binary: StratResult,
    bay60: StratResult,  // gate: P(win) > 60%
    bay55: StratResult,  // gate: P(win) > 55%
}

#[derive(Debug, Serialize)]
struct Summary {
    n_coins: usize,
    // Average metrics across all coins
    binary_avg_wr:    f64,
    bay60_avg_wr:     f64,
    bay55_avg_wr:     f64,
    binary_avg_pf:    f64,
    bay60_avg_pf:     f64,
    bay55_avg_pf:     f64,
    // Sum of per-coin total_pnl (not compounded)
    binary_sum_pnl:   f64,
    bay60_sum_pnl:    f64,
    bay55_sum_pnl:    f64,
    // Trade counts on test data
    binary_total_trades: usize,
    bay60_total_trades:  usize,
    bay55_total_trades:  usize,
    // Per-coin win counts
    coins_bay60_beats_binary_wr: usize,
    coins_bay60_beats_binary_pf: usize,
    coins_bay55_beats_binary_wr: usize,
    coins_bay55_beats_binary_pf: usize,
}

#[derive(Debug, Serialize)]
struct Output {
    experiment: String,
    train_test_split: String,
    per_coin: Vec<CoinResult>,
    summary: Summary,
}

// ── Main ─────────────────────────────────────────────────────────────────────

pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN15a — Bayesian Entry Gate vs COINCLAW Primary Strategies");
    println!("================================================================");
    println!("Train: first 50% of bars | Test: last 50% (OOS)");
    println!("Gates: P(win|z,rsi) > {:.0}%  and  > {:.0}%",
             THRESH_60 * 100.0, THRESH_55 * 100.0);
    println!("Running 18 coins in parallel...");
    println!();

    let out_path = "run15a_results.json";
    if std::path::Path::new(out_path).exists() {
        println!("Results already exist at {}. Delete to re-run.", out_path);
        return;
    }

    let per_coin: Vec<CoinResult> = COIN_STRATEGIES
        .par_iter()
        .filter_map(|&(coin, strat)| {
            if shutdown.load(Ordering::SeqCst) { return None; }

            let bars = load_ohlcv(coin);
            let n = bars.len();
            if n < 200 { return None; }

            let split = n / 2;

            // Full-series indicators (bar-aligned)
            let close_full: Vec<f64> = bars.iter().map(|b| b.c).collect();
            let z20_full   = z_score(&close_full, 20);
            let rsi14_full = rsi(&close_full, 14);

            // Full-series signals (sliced to train / test)
            let (entry_full, exit_full) = signals(&bars, strat);

            let train_entry = &entry_full[..split];
            let train_exit  = &exit_full[..split];
            let test_entry  = &entry_full[split..];
            let test_exit   = &exit_full[split..];

            let train_close: Vec<f64> = close_full[..split].to_vec();
            let test_close:  Vec<f64> = close_full[split..].to_vec();

            // ── Train: collect trades and observe outcomes ─────────────────
            let train_trades = collect_trades(&train_close, train_entry, train_exit);

            let mut model = BayesModel::new();
            for &(bar_idx, pnl) in &train_trades {
                model.learn(z20_full[bar_idx], rsi14_full[bar_idx], pnl > 0.0);
            }

            let model_cells = model.cells.values()
                .filter(|c| c.alpha + c.beta - 2.0 >= MIN_OBS)
                .count();

            // ── Test: build gated entry masks ─────────────────────────────
            let test_len = n - split;
            let mut bay60_entry = vec![false; test_len];
            let mut bay55_entry = vec![false; test_len];

            for i in 0..test_len {
                if test_entry[i] {
                    let global = split + i;
                    let p = model.prob_win(z20_full[global], rsi14_full[global]);
                    if p > THRESH_60 { bay60_entry[i] = true; }
                    if p > THRESH_55 { bay55_entry[i] = true; }
                }
            }

            // ── Backtest all three on test data ───────────────────────────
            let (_, bin_s)  = backtest(&test_close, test_entry,  test_exit, STOP_LOSS);
            let (_, b60_s)  = backtest(&test_close, &bay60_entry, test_exit, STOP_LOSS);
            let (_, b55_s)  = backtest(&test_close, &bay55_entry, test_exit, STOP_LOSS);

            println!(
                "  {:6} {:10}  train={:4}t  model={:3}cells  \
                 | bin  {:4.1}%WR PF={:.2} ({:4}t) \
                 | b60  {:4.1}%WR PF={:.2} ({:4}t) \
                 | b55  {:4.1}%WR PF={:.2} ({:4}t)",
                coin, strat, train_trades.len(), model_cells,
                bin_s.win_rate, bin_s.profit_factor, bin_s.n_trades,
                b60_s.win_rate, b60_s.profit_factor, b60_s.n_trades,
                b55_s.win_rate, b55_s.profit_factor, b55_s.n_trades,
            );

            Some(CoinResult {
                coin: coin.to_string(),
                strategy: strat.to_string(),
                train_trades: train_trades.len(),
                model_cells,
                binary: bin_s.into(),
                bay60:  b60_s.into(),
                bay55:  b55_s.into(),
            })
        })
        .collect();

    // ── Aggregate ─────────────────────────────────────────────────────────────
    let n = per_coin.len() as f64;

    macro_rules! avg { ($field:expr) => { $field.iter().sum::<f64>() / n } }
    macro_rules! sum_f { ($field:expr) => { $field.iter().sum::<f64>() } }
    macro_rules! sum_u { ($field:expr) => { $field.iter().sum::<usize>() } }

    let binary_avg_wr  = avg!(per_coin.iter().map(|r| r.binary.win_rate).collect::<Vec<_>>());
    let bay60_avg_wr   = avg!(per_coin.iter().map(|r| r.bay60.win_rate).collect::<Vec<_>>());
    let bay55_avg_wr   = avg!(per_coin.iter().map(|r| r.bay55.win_rate).collect::<Vec<_>>());
    let binary_avg_pf  = avg!(per_coin.iter().map(|r| r.binary.profit_factor).collect::<Vec<_>>());
    let bay60_avg_pf   = avg!(per_coin.iter().map(|r| r.bay60.profit_factor).collect::<Vec<_>>());
    let bay55_avg_pf   = avg!(per_coin.iter().map(|r| r.bay55.profit_factor).collect::<Vec<_>>());
    let binary_sum_pnl = sum_f!(per_coin.iter().map(|r| r.binary.total_pnl).collect::<Vec<_>>());
    let bay60_sum_pnl  = sum_f!(per_coin.iter().map(|r| r.bay60.total_pnl).collect::<Vec<_>>());
    let bay55_sum_pnl  = sum_f!(per_coin.iter().map(|r| r.bay55.total_pnl).collect::<Vec<_>>());
    let binary_total_trades = sum_u!(per_coin.iter().map(|r| r.binary.n_trades).collect::<Vec<_>>());
    let bay60_total_trades  = sum_u!(per_coin.iter().map(|r| r.bay60.n_trades).collect::<Vec<_>>());
    let bay55_total_trades  = sum_u!(per_coin.iter().map(|r| r.bay55.n_trades).collect::<Vec<_>>());

    let coins_bay60_beats_binary_wr = per_coin.iter().filter(|r| r.bay60.win_rate  > r.binary.win_rate).count();
    let coins_bay60_beats_binary_pf = per_coin.iter().filter(|r| r.bay60.profit_factor > r.binary.profit_factor).count();
    let coins_bay55_beats_binary_wr = per_coin.iter().filter(|r| r.bay55.win_rate  > r.binary.win_rate).count();
    let coins_bay55_beats_binary_pf = per_coin.iter().filter(|r| r.bay55.profit_factor > r.binary.profit_factor).count();

    let summary = Summary {
        n_coins: per_coin.len(),
        binary_avg_wr, bay60_avg_wr, bay55_avg_wr,
        binary_avg_pf, bay60_avg_pf, bay55_avg_pf,
        binary_sum_pnl, bay60_sum_pnl, bay55_sum_pnl,
        binary_total_trades, bay60_total_trades, bay55_total_trades,
        coins_bay60_beats_binary_wr,
        coins_bay60_beats_binary_pf,
        coins_bay55_beats_binary_wr,
        coins_bay55_beats_binary_pf,
    };

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY (test period, out-of-sample)");
    println!("  {:10}  Avg WR: {:4.1}%  Avg PF: {:.2}  Sum PnL: {:+.0}%  Trades: {}",
             "Binary",  binary_avg_wr, binary_avg_pf, binary_sum_pnl, binary_total_trades);
    println!("  {:10}  Avg WR: {:4.1}%  Avg PF: {:.2}  Sum PnL: {:+.0}%  Trades: {}  ({:.0}% of binary)",
             "Bay>60%", bay60_avg_wr, bay60_avg_pf, bay60_sum_pnl, bay60_total_trades,
             bay60_total_trades as f64 / binary_total_trades as f64 * 100.0);
    println!("  {:10}  Avg WR: {:4.1}%  Avg PF: {:.2}  Sum PnL: {:+.0}%  Trades: {}  ({:.0}% of binary)",
             "Bay>55%", bay55_avg_wr, bay55_avg_pf, bay55_sum_pnl, bay55_total_trades,
             bay55_total_trades as f64 / binary_total_trades as f64 * 100.0);
    println!();
    println!("  Coins where Bay>60% beats binary (WR): {}/{}", coins_bay60_beats_binary_wr, per_coin.len());
    println!("  Coins where Bay>60% beats binary (PF): {}/{}", coins_bay60_beats_binary_pf, per_coin.len());
    println!("  Coins where Bay>55% beats binary (WR): {}/{}", coins_bay55_beats_binary_wr, per_coin.len());
    println!("  Coins where Bay>55% beats binary (PF): {}/{}", coins_bay55_beats_binary_pf, per_coin.len());

    let output = Output {
        experiment: "Bayesian Entry Gate vs COINCLAW Primary Strategies (corrected)".to_string(),
        train_test_split: "50/50 — first 6 months train, last 6 months OOS test".to_string(),
        per_coin,
        summary,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON");
    std::fs::write(out_path, json).expect("write results");
    println!();
    println!("Results saved to {}", out_path);
}
