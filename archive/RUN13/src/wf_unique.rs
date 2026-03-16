//! RUN13 — Walk-Forward on UNIQUE signals only
//! For laguerre_rsi, kalman_filter, kst_cross: only take entries that
//! do NOT overlap with ctrl_mean_rev within 5 bars.
//! 3-window walk-forward across all 19 coins.

use run13_lib::indicators::{self, Indicators};
use run13_lib::strategies::{self, Candles, StratConfig};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run13/run13_wf_unique_results.json";

const FEE_RATE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;
const COST: f64 = FEE_RATE + SLIPPAGE;
const OVERLAP_WINDOW: usize = 5;

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
    window: usize,
    // In-sample
    is_total_signals: usize,
    is_unique_signals: usize,
    is_trades: i32,
    is_wins: i32,
    is_win_rate: f64,
    is_pf: f64,
    is_pnl: f64,
    // Out-of-sample
    oos_total_signals: usize,
    oos_unique_signals: usize,
    oos_trades: i32,
    oos_wins: i32,
    oos_win_rate: f64,
    oos_pf: f64,
    oos_pnl: f64,
    // Degradation
    wr_degrad_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WFOutput {
    run: String,
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

/// Get entry bar indices for a strategy on a data slice
fn get_entries(
    open: &[f64], high: &[f64], low: &[f64], close: &[f64], volume: &[f64],
    cfg: &StratConfig,
) -> Vec<usize> {
    let n = close.len();
    if n < 200 { return Vec::new(); }
    let ind = indicators::compute_all(open, high, low, close, volume);
    let candles = Candles { open, high, low, close, volume };
    let mut entries = Vec::new();
    for i in 1..n {
        if strategies::check_entry(&candles, &ind, i, cfg) {
            entries.push(i);
        }
    }
    entries
}

/// Filter out entries that overlap with ctrl entries within OVERLAP_WINDOW bars
fn filter_unique(strat_entries: &[usize], ctrl_entries: &[usize]) -> Vec<usize> {
    strat_entries.iter().filter(|&&se| {
        !ctrl_entries.iter().any(|&ce| {
            (se as i64 - ce as i64).unsigned_abs() as usize <= OVERLAP_WINDOW
        })
    }).copied().collect()
}

/// Backtest from a set of entry indices using coinclaw-style exits.
/// Returns (trades, wins, win_rate, pf, pnl)
fn backtest_entries(
    entries: &[usize],
    close: &[f64], ind: &Indicators,
) -> (i32, i32, f64, f64, f64) {
    let n = close.len();
    let round_trip_cost = 2.0 * COST;
    let sl = 0.003;
    let min_hold = 2usize;
    let max_hold = 30usize;
    let cooldown_period = 5usize;
    let position_size = 0.10;

    let mut balance = 10000.0;
    let mut win_pcts: Vec<f64> = Vec::new();
    let mut loss_pcts: Vec<f64> = Vec::new();
    let mut next_allowed = 0usize;

    for &entry_i in entries {
        if entry_i < next_allowed { continue; }
        let entry_price = close[entry_i];
        let mut exited = false;

        for held in 1..=max_hold {
            let j = entry_i + held;
            if j >= n { break; }
            let raw_pnl = (close[j] - entry_price) / entry_price;

            // SL
            if raw_pnl <= -sl {
                let net = -sl - round_trip_cost;
                balance += balance * position_size * net;
                loss_pcts.push(net * 100.0);
                next_allowed = j + cooldown_period;
                exited = true;
                break;
            }

            // Signal exits after min_hold
            if held >= min_hold {
                let mut do_exit = false;
                if !ind.sma20[j].is_nan() && close[j] > ind.sma20[j] {
                    if j > 0 && !ind.sma20[j-1].is_nan() && close[j-1] <= ind.sma20[j-1] {
                        do_exit = true;
                    }
                }
                if !do_exit && !ind.z_score[j].is_nan() && ind.z_score[j] > 0.5 {
                    do_exit = true;
                }
                if do_exit {
                    let net = raw_pnl - round_trip_cost;
                    balance += balance * position_size * net;
                    if net > 0.0 { win_pcts.push(net * 100.0); }
                    else { loss_pcts.push(net * 100.0); }
                    next_allowed = j + cooldown_period;
                    exited = true;
                    break;
                }
            }
        }

        if !exited {
            let j = (entry_i + max_hold).min(n - 1);
            let raw_pnl = (close[j] - entry_price) / entry_price;
            let net = raw_pnl - round_trip_cost;
            balance += balance * position_size * net;
            if net > 0.0 { win_pcts.push(net * 100.0); }
            else { loss_pcts.push(net * 100.0); }
            next_allowed = j + cooldown_period;
        }
    }

    let wins = win_pcts.len() as i32;
    let losses = loss_pcts.len() as i32;
    let total = wins + losses;
    let wr = if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 };
    let tw: f64 = win_pcts.iter().sum();
    let tl: f64 = loss_pcts.iter().map(|x| x.abs()).sum();
    let pf = if tl > 0.0 { tw / tl } else { 0.0 };
    let pnl = (balance - 10000.0) / 10000.0 * 100.0;
    (total, wins, wr, pf, pnl)
}

fn main() {
    println!("=== RUN13 — Walk-Forward on UNIQUE Signals ===");
    println!("Only entries that do NOT fire within {} bars of ctrl_mean_rev\n", OVERLAP_WINDOW);

    let ctrl = StratConfig { name: "ctrl_mean_rev", p1: 1.5, p2: 1.2, p3: 0.0, z_filter: 0.0 };

    // Strategies to WF — the 3 winners from overlap analysis
    let test_configs: Vec<(&str, Vec<StratConfig>)> = vec![
        ("laguerre_rsi", vec![
            StratConfig { name: "laguerre_rsi", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // gamma=0.5
            StratConfig { name: "laguerre_rsi", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // gamma=0.6
            StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // gamma=0.7
            StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // gamma=0.8
            StratConfig { name: "laguerre_rsi", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "laguerre_rsi", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "laguerre_rsi", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
            StratConfig { name: "laguerre_rsi", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
            StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
            StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
        ]),
        ("kalman_filter", vec![
            StratConfig { name: "kalman_filter", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // Q=0.01
            StratConfig { name: "kalman_filter", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // Q=0.001
            StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.0 }, // Q=0.0001
            StratConfig { name: "kalman_filter", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "kalman_filter", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "kalman_filter", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
            StratConfig { name: "kalman_filter", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
            StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
        ]),
        ("kst_cross", vec![
            StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
            StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 },
            StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.5 },
        ]),
    ];

    // Load coin data
    let coin_data: Vec<CoinData> = TARGET_COINS.iter()
        .filter_map(|coin| {
            let path = format!("{}/{}_15m_5months.csv", DATA_DIR, coin);
            load_csv(&path)
        })
        .collect();
    println!("Loaded {} coins\n", coin_data.len());

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
            if train_end <= train_start + 200 || test_end <= test_start + 200 { continue; }

            let train_o = &data.open[train_start..train_end];
            let train_h = &data.high[train_start..train_end];
            let train_l = &data.low[train_start..train_end];
            let train_c = &data.close[train_start..train_end];
            let train_v = &data.volume[train_start..train_end];

            let test_o = &data.open[test_start..test_end];
            let test_h = &data.high[test_start..test_end];
            let test_l = &data.low[test_start..test_end];
            let test_c = &data.close[test_start..test_end];
            let test_v = &data.volume[test_start..test_end];

            // ctrl entries for train and test
            let ctrl_train = get_entries(train_o, train_h, train_l, train_c, train_v, &ctrl);
            let ctrl_test = get_entries(test_o, test_h, test_l, test_c, test_v, &ctrl);

            let train_ind = indicators::compute_all(train_o, train_h, train_l, train_c, train_v);
            let test_ind = indicators::compute_all(test_o, test_h, test_l, test_c, test_v);

            for (_strat_name, cfgs) in &test_configs {
                // Find best cfg in-sample (by WR*PF on unique signals)
                let mut best_cfg: Option<&StratConfig> = None;
                let mut best_score = f64::NEG_INFINITY;
                let mut best_is_stats = (0usize, 0usize, 0i32, 0i32, 0.0f64, 0.0f64, 0.0f64);

                for cfg in cfgs {
                    let strat_train = get_entries(train_o, train_h, train_l, train_c, train_v, cfg);
                    let unique_train = filter_unique(&strat_train, &ctrl_train);

                    let (trades, wins, wr, pf, pnl) = backtest_entries(&unique_train, train_c, &train_ind);
                    if trades < 3 { continue; }

                    let score = wr * pf;
                    if score > best_score {
                        best_score = score;
                        best_cfg = Some(cfg);
                        best_is_stats = (strat_train.len(), unique_train.len(), trades, wins, wr, pf, pnl);
                    }
                }

                if let Some(cfg) = best_cfg {
                    // Test OOS
                    let strat_test = get_entries(test_o, test_h, test_l, test_c, test_v, cfg);
                    let unique_test = filter_unique(&strat_test, &ctrl_test);

                    let (oos_trades, oos_wins, oos_wr, oos_pf, oos_pnl) =
                        backtest_entries(&unique_test, test_c, &test_ind);

                    let wr_deg = if best_is_stats.4 > 0.0 {
                        (best_is_stats.4 - oos_wr) / best_is_stats.4 * 100.0
                    } else { 0.0 };

                    coin_results.push(WFResult {
                        coin: data.name.clone(),
                        strategy: cfg.name.to_string(),
                        params: cfg.label(),
                        window: wi + 1,
                        is_total_signals: best_is_stats.0,
                        is_unique_signals: best_is_stats.1,
                        is_trades: best_is_stats.2,
                        is_wins: best_is_stats.3,
                        is_win_rate: best_is_stats.4,
                        is_pf: best_is_stats.5,
                        is_pnl: best_is_stats.6,
                        oos_total_signals: strat_test.len(),
                        oos_unique_signals: unique_test.len(),
                        oos_trades,
                        oos_wins,
                        oos_win_rate: oos_wr,
                        oos_pf,
                        oos_pnl,
                        wr_degrad_pct: wr_deg,
                    });
                }
            }
        }

        let coin_name = &data.name;
        let n_res = coin_results.len();
        let oos_winners = coin_results.iter()
            .filter(|r| r.oos_trades >= 3 && r.oos_win_rate >= 55.0 && r.oos_pf >= 1.0)
            .count();
        println!("  {:<12} — {} results, {} OOS winners (WR>=55% PF>=1.0 T>=3)",
            coin_name, n_res, oos_winners);

        coin_results
    }).collect();

    let results: Vec<WFResult> = all_results.into_iter().flatten().collect();

    // ========================================
    // SUMMARY
    // ========================================
    println!("\n{}", "=".repeat(120));
    println!("=== WALK-FORWARD RESULTS (UNIQUE SIGNALS ONLY) ===\n");

    // Per-strategy summary
    for strat_name in &["laguerre_rsi", "kalman_filter", "kst_cross"] {
        let sr: Vec<&WFResult> = results.iter().filter(|r| r.strategy == *strat_name).collect();
        if sr.is_empty() { continue; }

        println!("--- {} ---\n", strat_name);
        println!("{:<12} {:>3} {:>30} {:>5}/{:>5} {:>6} {:>6} {:>6}  {:>5}/{:>5} {:>6} {:>6} {:>6} {:>7}",
            "Coin", "Win", "Params", "IS_T", "IS_U", "IS_WR", "IS_PF", "IS_P&L",
            "OOS_T", "OOS_U", "OOS_WR", "OOS_PF", "OOS_P&L", "WR_deg");
        println!("{}", "-".repeat(140));

        for r in &sr {
            println!("{:<12} W{} {:>30} {:>5}/{:>5} {:>5.1}% {:>6.2} {:>+5.1}%  {:>5}/{:>5} {:>5.1}% {:>6.2} {:>+5.1}% {:>+6.1}%",
                r.coin, r.window, r.params,
                r.is_trades, r.is_unique_signals, r.is_win_rate, r.is_pf, r.is_pnl,
                r.oos_trades, r.oos_unique_signals, r.oos_win_rate, r.oos_pf, r.oos_pnl,
                r.wr_degrad_pct);
        }

        // Averages
        let n = sr.len() as f64;
        let avg_is_wr = sr.iter().map(|r| r.is_win_rate).sum::<f64>() / n;
        let avg_oos_wr = sr.iter().map(|r| r.oos_win_rate).sum::<f64>() / n;
        let avg_oos_pf = sr.iter().map(|r| r.oos_pf).sum::<f64>() / n;
        let avg_oos_trades = sr.iter().map(|r| r.oos_trades as f64).sum::<f64>() / n;
        let avg_deg = sr.iter().map(|r| r.wr_degrad_pct).sum::<f64>() / n;

        // Count how many windows have OOS WR >= 50%
        let oos_positive = sr.iter().filter(|r| r.oos_trades >= 3 && r.oos_win_rate >= 50.0).count();

        println!("\nAVG: IS_WR={:.1}%  OOS_WR={:.1}%  OOS_PF={:.2}  OOS_T={:.0}  WR_deg={:+.1}%",
            avg_is_wr, avg_oos_wr, avg_oos_pf, avg_oos_trades, avg_deg);
        println!("Windows with OOS WR>=50% (T>=3): {}/{} ({:.0}%)\n",
            oos_positive, sr.len(), oos_positive as f64 / sr.len() as f64 * 100.0);
    }

    // Per-coin best unique strategy
    println!("\n{}", "=".repeat(100));
    println!("=== PER-COIN: BEST UNIQUE STRATEGY (avg across windows) ===\n");
    println!("{:<12} {:>14} {:>30} {:>6} {:>7} {:>7} {:>7} {:>8}",
        "Coin", "Strategy", "Params", "Avg_T", "Avg_WR", "Avg_PF", "Avg_P&L", "Verdict");
    println!("{}", "-".repeat(100));

    let mut coins: Vec<String> = results.iter().map(|r| r.coin.clone()).collect();
    coins.sort();
    coins.dedup();

    for coin in &coins {
        // For each strategy, compute average OOS across windows
        let mut best_strat = "";
        let mut best_params = String::new();
        let mut best_score = f64::NEG_INFINITY;
        let mut best_avg = (0.0f64, 0.0f64, 0.0f64, 0.0f64);

        for strat_name in &["laguerre_rsi", "kalman_filter", "kst_cross"] {
            let cr: Vec<&WFResult> = results.iter()
                .filter(|r| &r.coin == coin && r.strategy == *strat_name && r.oos_trades >= 2)
                .collect();
            if cr.is_empty() { continue; }
            let n = cr.len() as f64;
            let avg_t = cr.iter().map(|r| r.oos_trades as f64).sum::<f64>() / n;
            let avg_wr = cr.iter().map(|r| r.oos_win_rate).sum::<f64>() / n;
            let avg_pf = cr.iter().map(|r| r.oos_pf).sum::<f64>() / n;
            let avg_pnl = cr.iter().map(|r| r.oos_pnl).sum::<f64>() / n;
            let score = avg_wr * avg_pf;
            if score > best_score {
                best_score = score;
                best_strat = strat_name;
                best_params = cr[0].params.clone();
                best_avg = (avg_t, avg_wr, avg_pf, avg_pnl);
            }
        }

        if best_strat.is_empty() { continue; }

        let verdict = if best_avg.1 >= 55.0 && best_avg.2 >= 1.0 {
            "PASS"
        } else if best_avg.1 >= 50.0 && best_avg.2 >= 0.8 {
            "MARGINAL"
        } else {
            "FAIL"
        };

        println!("{:<12} {:>14} {:>30} {:>5.0} {:>6.1}% {:>7.2} {:>+6.1}% {:>8}",
            coin, best_strat, best_params, best_avg.0, best_avg.1, best_avg.2, best_avg.3, verdict);
    }

    // Final recommendation
    println!("\n{}", "=".repeat(100));
    println!("=== RECOMMENDATION ===\n");

    let passing_coins: Vec<(&String, &str)> = coins.iter().filter_map(|coin| {
        for strat_name in &["laguerre_rsi", "kalman_filter", "kst_cross"] {
            let cr: Vec<&WFResult> = results.iter()
                .filter(|r| &r.coin == coin && r.strategy == *strat_name && r.oos_trades >= 3)
                .collect();
            if cr.is_empty() { continue; }
            let n = cr.len() as f64;
            let avg_wr = cr.iter().map(|r| r.oos_win_rate).sum::<f64>() / n;
            let avg_pf = cr.iter().map(|r| r.oos_pf).sum::<f64>() / n;
            if avg_wr >= 55.0 && avg_pf >= 1.0 {
                return Some((coin, *strat_name));
            }
        }
        None
    }).collect();

    if passing_coins.is_empty() {
        println!("No coin×strategy combination passes walk-forward with unique signals.");
        println!("The complementary edge seen in full-sample does not hold out-of-sample.");
    } else {
        println!("Coins with viable complementary strategies (OOS WR>=55%, PF>=1.0):");
        for (coin, strat) in &passing_coins {
            println!("  {} → {}", coin, strat);
        }
        println!("\nThese could be added as secondary entry signals alongside ctrl_mean_rev.");
    }

    // Save
    let output = WFOutput {
        run: "RUN13_WF_UNIQUE".to_string(),
        results,
    };
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("\nResults saved to {}", RESULTS_FILE);
    }
    println!("\nDone.");
}
