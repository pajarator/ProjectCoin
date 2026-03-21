//! RUN11c.2 — Walk-Forward Validation
//! 3-window walk-forward for RUN11c.1 winners:
//!   - ou_mean_rev (DASH, NEAR, ALGO, LINK)
//!   - fib_entry (UNI)
//! Test across ALL 19 coins to check if OU edge generalizes

use run11c_lib::indicators::{self, Indicators};
use run11c_lib::strategies::{self, Candles, StratConfig};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run11c_2_results.json";

const FEE_RATE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;
const COST: f64 = FEE_RATE + SLIPPAGE;

#[derive(Debug, Clone, Copy)]
struct ExitMode {
    name: &'static str,
    sl: f64,
    max_hold: usize,
    min_hold: usize,
}

const EXIT_MODES: &[ExitMode] = &[
    ExitMode { name: "signal_only", sl: 0.0, max_hold: 30, min_hold: 2 },
    ExitMode { name: "signal_05", sl: 0.005, max_hold: 30, min_hold: 2 },
    ExitMode { name: "signal_10", sl: 0.010, max_hold: 30, min_hold: 2 },
];

// Test ALL coins — OU may generalize
const TARGET_COINS: &[&str] = &[
    "ADA_USDT", "ALGO_USDT", "ATOM_USDT", "AVAX_USDT", "BNB_USDT",
    "BTC_USDT", "DASH_USDT", "DOGE_USDT", "DOT_USDT", "ETH_USDT",
    "LINK_USDT", "LTC_USDT", "NEAR_USDT", "SHIB_USDT", "SOL_USDT",
    "TRX_USDT", "UNI_USDT", "XLM_USDT", "XRP_USDT",
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
    is_trades: i32,
    is_win_rate: f64,
    is_pf: f64,
    is_pnl: f64,
    oos_trades: i32,
    oos_win_rate: f64,
    oos_pf: f64,
    oos_pnl: f64,
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
        .from_path(path).ok()?;
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

            if exit.sl > 0.0 && raw_pnl <= -exit.sl {
                let net = -exit.sl - round_trip_cost;
                balance += balance * position_size * net;
                loss_pcts.push(net * 100.0);
                in_position = false;
                cooldown = cooldown_period;
                continue;
            }

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

fn wf_configs() -> Vec<StratConfig> {
    let mut cfgs = Vec::new();

    // ou_mean_rev: sweep halflife and deviation thresholds
    for &hl in &[5.0, 10.0, 20.0] {
        for &dev in &[1.5, 2.0, 2.5] {
            // z_filter is redundant for OU (deviation check subsumes it), but test 0.0 only
            cfgs.push(StratConfig { name: "ou_mean_rev", p1: hl, p2: dev, p3: 0.0, z_filter: 0.0 });
        }
    }

    // fib_entry: both levels
    for &level in &[0.0, 1.0] {
        for &z in &[0.0, -0.5, -1.0] {
            cfgs.push(StratConfig { name: "fib_entry", p1: level, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // hurst_filter: add as a comparison
    for &h in &[0.4, 0.45, 0.5] {
        cfgs.push(StratConfig { name: "hurst_filter", p1: h, p2: 0.0, p3: 0.0, z_filter: 0.0 });
    }

    // pct_rank
    for &t in &[5.0, 10.0, 15.0] {
        for &z in &[0.0, -0.5, -1.0] {
            cfgs.push(StratConfig { name: "pct_rank", p1: t, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // accel_reversal
    for &lb in &[3.0, 5.0] {
        for &z in &[-0.5, -1.0, -1.5] {
            cfgs.push(StratConfig { name: "accel_reversal", p1: lb, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // vwap_atr_revert
    for &m in &[1.0, 1.5, 2.0] {
        for &z in &[0.0, -1.0, -1.5] {
            cfgs.push(StratConfig { name: "vwap_atr_revert", p1: m, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // pivot_revert
    for &d in &[0.0, 0.002, 0.005] {
        for &z in &[0.0, -0.5, -1.0] {
            cfgs.push(StratConfig { name: "pivot_revert", p1: d, p2: 0.0, p3: 0.0, z_filter: z });
        }
    }

    // momentum_shift
    for &z in &[0.0, -0.5, -1.0, -1.5] {
        cfgs.push(StratConfig { name: "momentum_shift", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: z });
    }

    // ctrl_mean_rev (baseline)
    cfgs.push(StratConfig { name: "ctrl_mean_rev", p1: 1.5, p2: 1.2, p3: 0.0, z_filter: 0.0 });

    cfgs
}

fn main() {
    println!("=== RUN11c.2 — Walk-Forward Validation ===");
    println!("3 windows × 19 coins × full strategy grid\n");

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

    let month = 2880usize;
    let windows: Vec<(usize, usize, usize, usize)> = vec![
        (0, 2 * month, 2 * month, 3 * month),
        (month, 3 * month, 3 * month, 4 * month),
        (2 * month, 4 * month, 4 * month, 5 * month),
    ];

    let all_results: Vec<Vec<WFResult>> = coin_data.par_iter().map(|data| {
        let n = data.close.len();
        let mut coin_results = Vec::new();

        for (wi, &(train_start, train_end, test_start, test_end)) in windows.iter().enumerate() {
            let train_end = train_end.min(n);
            let test_end = test_end.min(n);
            if train_end <= train_start || test_end <= test_start { continue; }

            for exit in EXIT_MODES {
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
                    if trades < 5 { continue; }
                    let score = wr * pf;
                    if score > best_score {
                        best_score = score;
                        best_cfg = Some(cfg);
                        best_is = (trades, wr, pf, pnl);
                    }
                }

                if let Some(cfg) = best_cfg {
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
                        is_trades: best_is.0, is_win_rate: best_is.1,
                        is_pf: best_is.2, is_pnl: best_is.3,
                        oos_trades, oos_win_rate: oos_wr,
                        oos_pf, oos_pnl,
                        wr_degrad_pct: wr_deg, pf_degrad_pct: pf_deg,
                        window: wi + 1,
                    });
                }
            }
        }
        println!("  {} — {} walk-forward results", data.name, coin_results.len());
        coin_results
    }).collect();

    let results: Vec<WFResult> = all_results.into_iter().flatten().collect();

    // Summary
    println!("\n{}", "=".repeat(110));
    println!("=== WALK-FORWARD RESULTS ===\n");

    // Per-coin average OOS
    println!("--- Average OOS Performance per Coin ---\n");
    let mut coins: Vec<String> = results.iter().map(|r| r.coin.clone()).collect();
    coins.sort();
    coins.dedup();

    println!("{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>12} {:>10}",
        "Coin", "Avg_T", "Avg_WR%", "Avg_PF", "Avg_P&L", "WR_deg%", "Top_Strat", "Verdict");
    println!("{}", "-".repeat(90));

    for coin in &coins {
        let cr: Vec<&WFResult> = results.iter().filter(|r| &r.coin == coin).collect();
        if cr.is_empty() { continue; }
        let n = cr.len() as f64;
        let avg_t = cr.iter().map(|r| r.oos_trades as f64).sum::<f64>() / n;
        let avg_wr = cr.iter().map(|r| r.oos_win_rate).sum::<f64>() / n;
        let avg_pf = cr.iter().map(|r| r.oos_pf).sum::<f64>() / n;
        let avg_pnl = cr.iter().map(|r| r.oos_pnl).sum::<f64>() / n;
        let avg_wr_deg = cr.iter().map(|r| r.wr_degrad_pct).sum::<f64>() / n;

        // Most common strategy in best results
        let best = cr.iter()
            .max_by(|a, b| (a.oos_win_rate * a.oos_pf)
                .partial_cmp(&(b.oos_win_rate * b.oos_pf))
                .unwrap_or(std::cmp::Ordering::Equal));
        let top_strat = best.map(|b| b.strategy.as_str()).unwrap_or("?");

        let verdict = if avg_wr >= 55.0 && avg_pf >= 1.0 && avg_wr_deg < 30.0 {
            "PASS"
        } else if avg_wr >= 50.0 && avg_pf >= 0.8 && avg_wr_deg < 40.0 {
            "MARGINAL"
        } else {
            "FAIL"
        };

        println!("{:<12} {:>8.0} {:>7.1}% {:>8.2} {:>+7.1}% {:>+7.1}% {:>12} {}",
            coin, avg_t, avg_wr, avg_pf, avg_pnl, avg_wr_deg, top_strat, verdict);
    }

    // Best OOS results
    println!("\n--- Best OOS Results (WR>=55%, T>=5) ---\n");
    let mut best: Vec<&WFResult> = results.iter()
        .filter(|r| r.oos_win_rate >= 55.0 && r.oos_trades >= 5)
        .collect();
    best.sort_by(|a, b| (b.oos_win_rate * b.oos_pf)
        .partial_cmp(&(a.oos_win_rate * a.oos_pf))
        .unwrap_or(std::cmp::Ordering::Equal));
    println!("{:<12} {:<30} {:<12} {:>3} {:>6} {:>7} {:>6} {:>6} {:>7} {:>7}",
        "Coin", "Params", "Exit", "Win", "IS_WR", "IS_PF", "OOS_T", "OOS_WR", "OOS_PF", "WR_deg");
    println!("{}", "-".repeat(115));
    for r in best.iter().take(30) {
        println!("{:<12} {:<30} {:<12} W{} {:>5.1}% {:>7.2} {:>6} {:>5.1}% {:>7.2} {:>+6.1}%",
            r.coin, r.params, r.exit_mode, r.window,
            r.is_win_rate, r.is_pf,
            r.oos_trades, r.oos_win_rate, r.oos_pf, r.wr_degrad_pct);
    }

    // Strategy breakdown: which strategy is selected most often as best IS?
    println!("\n--- Strategy Selection Frequency (best IS per window) ---\n");
    let mut strat_freq: std::collections::HashMap<String, (usize, f64, f64)> = std::collections::HashMap::new();
    for r in &results {
        let e = strat_freq.entry(r.strategy.clone()).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        e.1 += r.oos_win_rate;
        e.2 += r.oos_pf;
    }
    let mut freq_list: Vec<(String, usize, f64, f64)> = strat_freq.iter()
        .map(|(k, (c, wr, pf))| (k.clone(), *c, *wr, *pf))
        .collect();
    freq_list.sort_by(|a, b| b.1.cmp(&a.1));
    println!("{:<22} {:>8} {:>10} {:>10}", "Strategy", "Selected", "Avg_OOS_WR", "Avg_OOS_PF");
    println!("{}", "-".repeat(54));
    for (name, count, wr, pf) in &freq_list {
        println!("{:<22} {:>8} {:>9.1}% {:>10.2}", name, count, wr / *count as f64, pf / *count as f64);
    }

    // Save
    let output = WFOutput {
        run: "RUN11c.2".to_string(),
        total_windows: 3,
        results,
    };
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("\nResults saved to {}", RESULTS_FILE);
    }
    println!("Done.");
}
