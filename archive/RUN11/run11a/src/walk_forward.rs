//! RUN11a.2 — Walk-Forward Validation
//! 3-window walk-forward (train 2mo, test 1mo) for RUN11a.1 winners:
//!   - stoch_rsi (threshold=10,20 × z_filter=-0.5,-1.0)
//!   - effort_result (vol_mult=1.5,2.0 × z_filter=-1.0,-1.5)
//!   - inside_bar (breakout=0.001,0.003 × z_filter=-0.5,-1.0)
//! Focused on coins that showed promise: DASH, DOT, DOGE, ADA, ATOM, UNI, LTC

use run11a_lib::indicators::{self, Indicators};
use run11a_lib::strategies::{self, Candles, StratConfig};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run11a_2_results.json";

const FEE_RATE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;
const COST: f64 = FEE_RATE + SLIPPAGE;

// Exit modes to test
#[derive(Debug, Clone, Copy)]
struct ExitMode {
    name: &'static str,
    sl: f64,
    max_hold: usize,
    min_hold: usize,
}

const EXIT_MODES: &[ExitMode] = &[
    ExitMode { name: "signal_only", sl: 0.0, max_hold: 30, min_hold: 2 },
    ExitMode { name: "signal_10", sl: 0.010, max_hold: 30, min_hold: 2 },
    ExitMode { name: "signal_05", sl: 0.005, max_hold: 30, min_hold: 2 },
];

// Coins that showed promise in RUN11a.1 (corrected implementations)
const TARGET_COINS: &[&str] = &[
    "SHIB_USDT", "XLM_USDT", "DASH_USDT", "ALGO_USDT",
    "ADA_USDT", "DOT_USDT", "XRP_USDT",
];

struct CoinData {
    name: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WFResult {
    coin: String,
    strategy: String,
    params: String,
    exit_mode: String,
    // In-sample (train)
    is_trades: i32,
    is_win_rate: f64,
    is_pf: f64,
    is_pnl: f64,
    // Out-of-sample (test)
    oos_trades: i32,
    oos_win_rate: f64,
    oos_pf: f64,
    oos_pnl: f64,
    // Degradation
    wr_degrad_pct: f64,
    pf_degrad_pct: f64,
    window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WFOutput {
    run: String,
    total_windows: usize,
    results: Vec<WFResult>,
}

fn load_csv(path: &str) -> Option<CoinData> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .ok()?;

    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();

    for result in rdr.records() {
        let record = match result { Ok(r) => r, Err(_) => continue };
        let o: f64 = record.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let h: f64 = record.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let l: f64 = record.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let c: f64 = record.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let v: f64 = record.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        if o > 0.0 && h > 0.0 && l > 0.0 && c > 0.0 {
            open.push(o); high.push(h); low.push(l); close.push(c); volume.push(v);
        }
    }

    let coin_name = std::path::Path::new(path)
        .file_name()?.to_str()?
        .replace("_15m_5months.csv", "").replace('_', "/");

    Some(CoinData { name: coin_name, open, high, low, close, volume })
}

/// Backtest on a slice of data. Returns (trades, wins, pf, pnl_pct).
fn backtest_slice(
    open: &[f64], high: &[f64], low: &[f64], close: &[f64], volume: &[f64],
    cfg: &StratConfig, exit: &ExitMode,
) -> (i32, f64, f64, f64) {
    let n = close.len();
    if n < 200 { return (0, 0.0, 0.0, 0.0); }

    let ind = indicators::compute_all(open, high, low, close, volume);
    let candles = Candles { open, high, low, close, volume };
    let round_trip_cost = 2.0 * COST;
    let position_size = 0.10;
    let cooldown_period: usize = 5;

    let mut balance = 10000.0;
    let mut in_position = false;
    let mut entry_price = 0.0;
    let mut candles_held: usize = 0;
    let mut cooldown: usize = 0;
    let mut win_pcts: Vec<f64> = Vec::new();
    let mut loss_pcts: Vec<f64> = Vec::new();

    for i in 1..n {
        if !in_position {
            if cooldown > 0 { cooldown -= 1; continue; }
            if strategies::check_entry(&candles, &ind, i, cfg) {
                entry_price = close[i];
                in_position = true;
                candles_held = 0;
            }
        } else {
            candles_held += 1;
            let raw_pnl = (close[i] - entry_price) / entry_price;

            // SL
            if exit.sl > 0.0 && raw_pnl <= -exit.sl {
                let net = -exit.sl - round_trip_cost;
                balance += balance * position_size * net;
                loss_pcts.push(net * 100.0);
                in_position = false;
                cooldown = cooldown_period;
                continue;
            }

            // Signal exits
            if candles_held >= exit.min_hold {
                let mut do_exit = false;
                if !ind.sma20[i].is_nan() && close[i] > ind.sma20[i] {
                    if i > 0 && !ind.sma20[i-1].is_nan() && close[i-1] <= ind.sma20[i-1] {
                        do_exit = true;
                    }
                }
                if !do_exit && !ind.z_score[i].is_nan() && ind.z_score[i] > 0.5 {
                    do_exit = true;
                }
                if do_exit {
                    let net = raw_pnl - round_trip_cost;
                    balance += balance * position_size * net;
                    if net > 0.0 { win_pcts.push(net * 100.0); }
                    else { loss_pcts.push(net * 100.0); }
                    in_position = false;
                    cooldown = cooldown_period;
                    continue;
                }
            }

            // Max hold
            if exit.max_hold > 0 && candles_held >= exit.max_hold {
                let net = raw_pnl - round_trip_cost;
                balance += balance * position_size * net;
                if net > 0.0 { win_pcts.push(net * 100.0); }
                else { loss_pcts.push(net * 100.0); }
                in_position = false;
                cooldown = cooldown_period;
            }
        }
    }

    if in_position {
        let raw_pnl = (close[n-1] - entry_price) / entry_price;
        let net = raw_pnl - round_trip_cost;
        balance += balance * position_size * net;
        if net > 0.0 { win_pcts.push(net * 100.0); }
        else { loss_pcts.push(net * 100.0); }
    }

    let total = (win_pcts.len() + loss_pcts.len()) as i32;
    let wins = win_pcts.len() as i32;
    let wr = if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 };
    let tw: f64 = win_pcts.iter().sum();
    let tl: f64 = loss_pcts.iter().map(|x| x.abs()).sum();
    let pf = if tl > 0.0 { tw / tl } else { 0.0 };
    let pnl = (balance - 10000.0) / 10000.0 * 100.0;

    (total, wr, pf, pnl)
}

/// Generate the focused strategy configs for walk-forward.
fn wf_configs() -> Vec<StratConfig> {
    let mut cfgs = Vec::new();

    // effort_result (corrected VSA): vol_mult × max_spread_ratio
    // Best from RUN11a.1: vm=1.2, sr=0.6-0.7, z=-0.5 to -1.0
    for &vm in &[1.2, 1.5, 2.0] {
        for &sr in &[0.5, 0.6, 0.7] {
            for &z in &[0.0, -0.5, -1.0, -1.5] {
                cfgs.push(StratConfig { name: "effort_result", p1: vm, p2: sr, p3: 0.0, z_filter: z });
            }
        }
    }

    // inside_bar (ADA winner in RUN11a.1)
    for &b in &[0.001, 0.003] {
        for &z in &[0.0, -0.5, -1.0, -1.5] {
            cfgs.push(StratConfig { name: "inside_bar", p1: b, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // Also test displacement and marubozu (had near-misses)
    for &m in &[1.5, 2.0, 2.5] {
        for &z in &[-0.5, -1.0] {
            cfgs.push(StratConfig { name: "displacement", p1: m, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }
    for &b in &[0.85, 0.95] {
        for &z in &[-0.5, -1.0] {
            cfgs.push(StratConfig { name: "marubozu", p1: b, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // CMF reversal
    for &t in &[-0.15, -0.25] {
        for &z in &[-1.0, -1.5] {
            cfgs.push(StratConfig { name: "cmf_reversal", p1: t, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    cfgs
}

fn main() {
    println!("=== RUN11a.2 — Walk-Forward Validation ===");
    println!("3 windows × 7 coins × focused strategy grid\n");

    // Load target coins
    let coin_data: Vec<CoinData> = TARGET_COINS.iter()
        .filter_map(|coin| {
            let path = format!("{}/{}_15m_5months.csv", DATA_DIR, coin);
            load_csv(&path)
        })
        .collect();

    println!("Loaded {} coins\n", coin_data.len());

    let configs = wf_configs();
    println!("Strategy configs: {}", configs.len());
    println!("Exit modes: {}", EXIT_MODES.len());
    println!("Total per window per coin: {}\n", configs.len() * EXIT_MODES.len());

    // Walk-forward windows: split data into ~equal months
    // 14,400 candles / 5 months ≈ 2,880 per month
    // Train: 2 months, Test: 1 month
    let month = 2880usize;
    let windows: Vec<(usize, usize, usize, usize)> = vec![
        (0, 2 * month, 2 * month, 3 * month),          // W1: train M1-M2, test M3
        (month, 3 * month, 3 * month, 4 * month),      // W2: train M2-M3, test M4
        (2 * month, 4 * month, 4 * month, 5 * month),  // W3: train M3-M4, test M5
    ];

    // Process coins in parallel
    let all_results: Vec<Vec<WFResult>> = coin_data.par_iter().map(|data| {
        let n = data.close.len();
        let mut coin_results = Vec::new();

        for (wi, &(train_start, train_end, test_start, test_end)) in windows.iter().enumerate() {
            let train_end = train_end.min(n);
            let test_end = test_end.min(n);
            if train_end <= train_start || test_end <= test_start { continue; }

            // For each exit mode
            for exit in EXIT_MODES {
                // Train: find best config
                let mut best_cfg: Option<&StratConfig> = None;
                let mut best_score = f64::NEG_INFINITY;
                let mut best_is = (0i32, 0.0f64, 0.0f64, 0.0f64);

                for cfg in &configs {
                    let (trades, wr, pf, pnl) = backtest_slice(
                        &data.open[train_start..train_end],
                        &data.high[train_start..train_end],
                        &data.low[train_start..train_end],
                        &data.close[train_start..train_end],
                        &data.volume[train_start..train_end],
                        cfg, exit,
                    );

                    if trades < 10 { continue; }
                    // Score: WR * PF (balanced metric)
                    let score = wr * pf;
                    if score > best_score {
                        best_score = score;
                        best_cfg = Some(cfg);
                        best_is = (trades, wr, pf, pnl);
                    }
                }

                if let Some(cfg) = best_cfg {
                    // Test: run best config on OOS data
                    let (oos_trades, oos_wr, oos_pf, oos_pnl) = backtest_slice(
                        &data.open[test_start..test_end],
                        &data.high[test_start..test_end],
                        &data.low[test_start..test_end],
                        &data.close[test_start..test_end],
                        &data.volume[test_start..test_end],
                        cfg, exit,
                    );

                    let wr_deg = if best_is.1 > 0.0 { (best_is.1 - oos_wr) / best_is.1 * 100.0 } else { 0.0 };
                    let pf_deg = if best_is.2 > 0.0 { (best_is.2 - oos_pf) / best_is.2 * 100.0 } else { 0.0 };

                    coin_results.push(WFResult {
                        coin: data.name.clone(),
                        strategy: cfg.name.to_string(),
                        params: cfg.label(),
                        exit_mode: exit.name.to_string(),
                        is_trades: best_is.0,
                        is_win_rate: best_is.1,
                        is_pf: best_is.2,
                        is_pnl: best_is.3,
                        oos_trades,
                        oos_win_rate: oos_wr,
                        oos_pf: oos_pf,
                        oos_pnl,
                        wr_degrad_pct: wr_deg,
                        pf_degrad_pct: pf_deg,
                        window: wi + 1,
                    });
                }
            }
        }

        println!("  {} — {} walk-forward results", data.name, coin_results.len());
        coin_results
    }).collect();

    let results: Vec<WFResult> = all_results.into_iter().flatten().collect();

    // Print summary
    println!("\n{}", "=".repeat(100));
    println!("=== WALK-FORWARD RESULTS ===\n");

    // Per coin × exit mode: average IS and OOS
    println!("{:<12} {:<14} {:>3} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "Coin", "Exit", "Win", "IS_T", "IS_WR%", "IS_PF", "OOS_T", "OOS_WR%", "OOS_PF", "WR_deg%", "PF_deg%");
    println!("{}", "-".repeat(100));

    for r in &results {
        println!("{:<12} {:<14} W{} {:>7} {:>6.1}% {:>7.2} {:>7} {:>6.1}% {:>7.2} {:>+7.1}% {:>+7.1}%",
            r.coin, r.exit_mode, r.window,
            r.is_trades, r.is_win_rate, r.is_pf,
            r.oos_trades, r.oos_win_rate, r.oos_pf,
            r.wr_degrad_pct, r.pf_degrad_pct);
    }

    // Summary: average OOS per coin
    println!("\n--- Average OOS Performance per Coin ---\n");
    let mut coins: Vec<String> = results.iter().map(|r| r.coin.clone()).collect();
    coins.sort();
    coins.dedup();

    println!("{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "Coin", "Avg_OOS_T", "Avg_WR%", "Avg_PF", "Avg_P&L", "WR_deg%", "Verdict");
    println!("{}", "-".repeat(68));

    for coin in &coins {
        let cr: Vec<&WFResult> = results.iter().filter(|r| &r.coin == coin).collect();
        if cr.is_empty() { continue; }
        let n = cr.len() as f64;
        let avg_t = cr.iter().map(|r| r.oos_trades as f64).sum::<f64>() / n;
        let avg_wr = cr.iter().map(|r| r.oos_win_rate).sum::<f64>() / n;
        let avg_pf = cr.iter().map(|r| r.oos_pf).sum::<f64>() / n;
        let avg_pnl = cr.iter().map(|r| r.oos_pnl).sum::<f64>() / n;
        let avg_wr_deg = cr.iter().map(|r| r.wr_degrad_pct).sum::<f64>() / n;

        let verdict = if avg_wr >= 55.0 && avg_pf >= 1.0 && avg_wr_deg < 30.0 {
            "PASS"
        } else if avg_wr >= 50.0 && avg_pf >= 0.8 && avg_wr_deg < 40.0 {
            "MARGINAL"
        } else {
            "FAIL"
        };

        println!("{:<12} {:>8.0} {:>7.1}% {:>8.2} {:>+7.1}% {:>+7.1}%  {}",
            coin, avg_t, avg_wr, avg_pf, avg_pnl, avg_wr_deg, verdict);
    }

    // Best OOS results across all windows
    println!("\n--- Best OOS Results (OOS_WR>=55%, OOS_trades>=5) ---\n");
    let mut best: Vec<&WFResult> = results.iter()
        .filter(|r| r.oos_win_rate >= 55.0 && r.oos_trades >= 5)
        .collect();
    best.sort_by(|a, b| b.oos_pf.partial_cmp(&a.oos_pf).unwrap_or(std::cmp::Ordering::Equal));
    println!("{:<12} {:<18} {:<14} {:>3} {:>6} {:>7} {:>7} {:>6} {:>7} {:>7}",
        "Coin", "Strategy", "Exit", "Win", "IS_WR", "IS_PF", "OOS_T", "OOS_WR", "OOS_PF", "WR_deg");
    println!("{}", "-".repeat(100));
    for r in best.iter().take(20) {
        println!("{:<12} {:<18} {:<14} W{} {:>5.1}% {:>7.2} {:>7} {:>5.1}% {:>7.2} {:>+6.1}%",
            r.coin, r.params, r.exit_mode, r.window,
            r.is_win_rate, r.is_pf,
            r.oos_trades, r.oos_win_rate, r.oos_pf, r.wr_degrad_pct);
    }

    // Save
    let output = WFOutput {
        run: "RUN11a.2".to_string(),
        total_windows: 3,
        results,
    };
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("\nResults saved to {}", RESULTS_FILE);
    }
    println!("Done.");
}
