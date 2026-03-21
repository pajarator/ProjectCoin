//! RUN11c.1 — New Indicator Strategy Discovery (12 strategies)
//! Grid search across 19 coins × 12 strategies × param variations × exit modes

use run11c_lib::indicators::{self, Indicators};
use run11c_lib::strategies::{self, Candles, StratConfig};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run11c_1_results.json";
const CHECKPOINT_FILE: &str = "/home/scamarena/ProjectCoin/run11c_1_checkpoint.json";

const FEE_RATE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;
const COST: f64 = FEE_RATE + SLIPPAGE;

#[derive(Debug, Clone, Copy)]
struct ExitMode {
    name: &'static str,
    sl: f64,
    tp: f64,
    max_hold: usize,
    min_hold: usize,
    use_signals: bool,
}

const EXIT_MODES: &[ExitMode] = &[
    ExitMode { name: "signal_only", sl: 0.0, tp: 0.0, max_hold: 30, min_hold: 2, use_signals: true },
    ExitMode { name: "coinclaw", sl: 0.003, tp: 0.0, max_hold: 30, min_hold: 2, use_signals: true },
    ExitMode { name: "signal_05", sl: 0.005, tp: 0.0, max_hold: 30, min_hold: 2, use_signals: true },
    ExitMode { name: "signal_10", sl: 0.010, tp: 0.0, max_hold: 30, min_hold: 2, use_signals: true },
    ExitMode { name: "tp_sl_05_10", sl: 0.005, tp: 0.010, max_hold: 40, min_hold: 0, use_signals: false },
    ExitMode { name: "tp_sl_10_20", sl: 0.010, tp: 0.020, max_hold: 40, min_hold: 0, use_signals: false },
    ExitMode { name: "tp_sl_15_30", sl: 0.015, tp: 0.030, max_hold: 40, min_hold: 0, use_signals: false },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestResult {
    coin: String,
    strategy: String,
    params: String,
    exit_mode: String,
    trades: i32,
    wins: i32,
    losses: i32,
    win_rate: f64,
    profit_factor: f64,
    pnl_pct: f64,
    max_drawdown_pct: f64,
    avg_win_pct: f64,
    avg_loss_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunOutput {
    run: String,
    total_coins: usize,
    total_configs: usize,
    total_backtests: usize,
    results: Vec<BacktestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Checkpoint {
    completed_coins: Vec<String>,
    results: Vec<BacktestResult>,
}

struct CoinData {
    name: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
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

fn run_backtest(data: &CoinData, ind: &Indicators, cfg: &StratConfig, exit: &ExitMode) -> BacktestResult {
    let n = data.close.len();
    let candles = Candles {
        open: &data.open, high: &data.high, low: &data.low,
        close: &data.close, volume: &data.volume,
    };

    let round_trip_cost = 2.0 * COST;
    let mut balance = 10000.0;
    let mut peak_balance = balance;
    let mut max_drawdown = 0.0;
    let mut in_position = false;
    let mut entry_price = 0.0;
    let mut candles_held: usize = 0;
    let mut cooldown: usize = 0;
    let cooldown_period: usize = 5;
    let position_size = 0.10;

    let mut win_pcts: Vec<f64> = Vec::new();
    let mut loss_pcts: Vec<f64> = Vec::new();

    for i in 1..n {
        if !in_position {
            if cooldown > 0 { cooldown -= 1; continue; }
            if strategies::check_entry(&candles, ind, i, cfg) {
                entry_price = data.close[i];
                in_position = true;
                candles_held = 0;
            }
        } else {
            candles_held += 1;
            let raw_pnl = (data.close[i] - entry_price) / entry_price;

            if exit.sl > 0.0 && raw_pnl <= -exit.sl {
                let net = -exit.sl - round_trip_cost;
                balance += balance * position_size * net;
                loss_pcts.push(net * 100.0);
                in_position = false;
                cooldown = cooldown_period;
                update_drawdown(&mut peak_balance, balance, &mut max_drawdown);
                continue;
            }

            if exit.tp > 0.0 && raw_pnl >= exit.tp {
                let net = exit.tp - round_trip_cost;
                balance += balance * position_size * net;
                win_pcts.push(net * 100.0);
                in_position = false;
                cooldown = cooldown_period;
                update_drawdown(&mut peak_balance, balance, &mut max_drawdown);
                continue;
            }

            if exit.use_signals && candles_held >= exit.min_hold {
                let mut do_exit = false;
                if !ind.sma20[i].is_nan() && data.close[i] > ind.sma20[i] {
                    if i > 0 && !ind.sma20[i - 1].is_nan() && data.close[i - 1] <= ind.sma20[i - 1] {
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
                    update_drawdown(&mut peak_balance, balance, &mut max_drawdown);
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
                update_drawdown(&mut peak_balance, balance, &mut max_drawdown);
            }
        }
    }

    if in_position {
        let raw_pnl = (data.close[n - 1] - entry_price) / entry_price;
        let net = raw_pnl - round_trip_cost;
        balance += balance * position_size * net;
        if net > 0.0 { win_pcts.push(net * 100.0); }
        else { loss_pcts.push(net * 100.0); }
    }

    let total = (win_pcts.len() + loss_pcts.len()) as i32;
    let wins = win_pcts.len() as i32;
    let losses = loss_pcts.len() as i32;
    let win_rate = if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 };
    let total_win: f64 = win_pcts.iter().sum();
    let total_loss: f64 = loss_pcts.iter().map(|x| x.abs()).sum();
    let profit_factor = if total_loss > 0.0 { total_win / total_loss } else { 0.0 };
    let avg_win = if !win_pcts.is_empty() { total_win / win_pcts.len() as f64 } else { 0.0 };
    let avg_loss = if !loss_pcts.is_empty() { loss_pcts.iter().sum::<f64>() / loss_pcts.len() as f64 } else { 0.0 };
    let pnl_pct = (balance - 10000.0) / 10000.0 * 100.0;

    BacktestResult {
        coin: data.name.clone(), strategy: cfg.name.to_string(),
        params: cfg.label(), exit_mode: exit.name.to_string(),
        trades: total, wins, losses, win_rate, profit_factor,
        pnl_pct, max_drawdown_pct: max_drawdown, avg_win_pct: avg_win, avg_loss_pct: avg_loss,
    }
}

fn update_drawdown(peak: &mut f64, balance: f64, max_dd: &mut f64) {
    if balance > *peak { *peak = balance; }
    let dd = (*peak - balance) / *peak * 100.0;
    if dd > *max_dd { *max_dd = dd; }
}

fn main() {
    println!("=== RUN11c.1 — New Indicator Strategy Discovery ===");
    println!("12 strategies × {} exit modes × 19 coins\n", EXIT_MODES.len());

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        eprintln!("\nSIGINT received, finishing current coins and saving checkpoint...");
        r.store(false, Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    let mut coin_files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(DATA_DIR) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with("_15m_5months.csv") {
                coin_files.push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    coin_files.sort();
    println!("Found {} coins", coin_files.len());

    let mut completed_coins: Vec<String> = Vec::new();
    let mut all_results: Vec<BacktestResult> = Vec::new();
    if let Ok(data) = fs::read_to_string(CHECKPOINT_FILE) {
        if let Ok(cp) = serde_json::from_str::<Checkpoint>(&data) {
            completed_coins = cp.completed_coins;
            all_results = cp.results;
            println!("Resumed from checkpoint: {} coins done", completed_coins.len());
        }
    }

    let remaining_files: Vec<String> = coin_files.iter()
        .filter(|f| {
            let name = std::path::Path::new(f).file_name()
                .and_then(|n| n.to_str()).unwrap_or("")
                .replace("_15m_5months.csv", "").replace('_', "/");
            !completed_coins.contains(&name)
        })
        .cloned().collect();

    let configs = strategies::all_configs();
    let total_configs = configs.len() * EXIT_MODES.len();
    println!("Strategy × exit configs: {} ({} strats × {} exits)", total_configs, configs.len(), EXIT_MODES.len());
    println!("Remaining: {} coins → {} total backtests\n", remaining_files.len(), remaining_files.len() * total_configs);

    println!("Loading coin data...");
    let coin_data: Vec<CoinData> = remaining_files.iter()
        .filter_map(|f| load_csv(f))
        .filter(|d| d.close.len() > 200)
        .collect();
    let avg_candles = if !coin_data.is_empty() {
        coin_data.iter().map(|d| d.close.len()).sum::<usize>() / coin_data.len()
    } else { 0 };
    println!("Loaded {} coins ({} candles avg)\n", coin_data.len(), avg_candles);

    let progress = Arc::new(AtomicUsize::new(0));
    let total_coins = coin_data.len();
    let running_ref = running.clone();
    let progress_ref = progress.clone();

    let new_results: Vec<Vec<BacktestResult>> = coin_data.par_iter()
        .map(|data| {
            if !running_ref.load(Ordering::Relaxed) { return Vec::new(); }

            let ind = indicators::compute_all(
                &data.open, &data.high, &data.low, &data.close, &data.volume,
            );

            let mut results = Vec::with_capacity(total_configs);
            for cfg in &configs {
                for exit in EXIT_MODES {
                    let r = run_backtest(data, &ind, cfg, exit);
                    if r.trades > 0 { results.push(r); }
                }
            }

            let done = progress_ref.fetch_add(1, Ordering::Relaxed) + 1;
            let pct = done as f64 / total_coins as f64 * 100.0;
            let winners = results.iter()
                .filter(|r| r.win_rate >= 60.0 && r.trades >= 30 && r.profit_factor >= 1.2)
                .count();
            println!("[{:>2}/{:>2}] {:>3.0}% | {:<12} | {} results, {} winners (WR>=60% PF>=1.2 T>=30)",
                done, total_coins, pct, data.name, results.len(), winners);

            results
        })
        .collect();

    for coin_results in &new_results {
        if let Some(first) = coin_results.first() {
            if !completed_coins.contains(&first.coin) {
                completed_coins.push(first.coin.clone());
            }
        }
        all_results.extend(coin_results.iter().cloned());
    }

    save_checkpoint(&completed_coins, &all_results);
    print_summary(&all_results);

    let output = RunOutput {
        run: "RUN11c.1".to_string(),
        total_coins: coin_files.len(),
        total_configs,
        total_backtests: all_results.len(),
        results: all_results,
    };

    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("\nResults saved to {}", RESULTS_FILE);
    }

    fs::remove_file(CHECKPOINT_FILE).ok();
    println!("Done.");
}

fn save_checkpoint(completed: &[String], results: &[BacktestResult]) {
    let cp = Checkpoint { completed_coins: completed.to_vec(), results: results.to_vec() };
    if let Ok(json) = serde_json::to_string(&cp) {
        fs::write(CHECKPOINT_FILE, json).ok();
    }
}

fn print_summary(results: &[BacktestResult]) {
    println!("\n{}", "=".repeat(90));
    println!("=== RESULTS SUMMARY ===\n");

    let total = results.len();
    println!("Total backtest results: {}", total);

    let mut winners: Vec<&BacktestResult> = results.iter()
        .filter(|r| r.win_rate >= 60.0 && r.trades >= 30 && r.profit_factor >= 1.2)
        .collect();
    winners.sort_by(|a, b| b.profit_factor.partial_cmp(&a.profit_factor).unwrap_or(std::cmp::Ordering::Equal));

    println!("Winners (WR>=60%, trades>=30, PF>=1.2): {}\n", winners.len());

    if winners.is_empty() {
        println!("No strategies met the primary criteria.\n");

        let mut relaxed: Vec<&BacktestResult> = results.iter()
            .filter(|r| r.win_rate >= 55.0 && r.trades >= 20 && r.profit_factor >= 1.0)
            .collect();
        relaxed.sort_by(|a, b| b.profit_factor.partial_cmp(&a.profit_factor).unwrap_or(std::cmp::Ordering::Equal));

        if !relaxed.is_empty() {
            println!("Relaxed criteria (WR>=55%, T>=20, PF>=1.0): {} results\n", relaxed.len());
            print_table(&relaxed[..relaxed.len().min(30)]);
        }

        let mut by_wr: Vec<&BacktestResult> = results.iter()
            .filter(|r| r.trades >= 20).collect();
        by_wr.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap_or(std::cmp::Ordering::Equal));
        println!("\nTop 30 by win rate (trades>=20):");
        print_table(&by_wr[..by_wr.len().min(30)]);
        return;
    }

    println!("--- TOP 40 Winners ---\n");
    print_table(&winners[..winners.len().min(40)]);

    println!("\n--- Best Strategy per Coin ---\n");
    let mut by_coin: HashMap<String, Vec<&BacktestResult>> = HashMap::new();
    for r in &winners { by_coin.entry(r.coin.clone()).or_default().push(r); }
    let mut coins: Vec<&String> = by_coin.keys().collect();
    coins.sort();
    for coin in coins {
        if let Some(best) = by_coin[coin].first() {
            println!("  {:<12} {:<22} {:<14} WR={:>5.1}% PF={:>5.2} T={:>4} P&L={:>+6.1}%",
                coin, best.strategy, best.exit_mode, best.win_rate, best.profit_factor, best.trades, best.pnl_pct);
        }
    }

    println!("\n--- Strategy Leaderboard ---\n");
    let mut strat_coins: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    let mut strat_counts: HashMap<String, (usize, f64, f64)> = HashMap::new();
    for r in &winners {
        strat_coins.entry(r.strategy.clone()).or_default().insert(r.coin.clone());
        let e = strat_counts.entry(r.strategy.clone()).or_insert((0, 0.0, 0.0));
        e.0 += 1; e.1 += r.win_rate; e.2 += r.profit_factor;
    }
    let mut strat_list: Vec<(String, usize, f64, f64, usize)> = strat_counts.iter()
        .map(|(name, (count, wr, pf))| {
            let n_coins = strat_coins.get(name).map(|s| s.len()).unwrap_or(0);
            (name.clone(), *count, *wr, *pf, n_coins)
        }).collect();
    strat_list.sort_by(|a, b| b.4.cmp(&a.4).then(b.1.cmp(&a.1)));

    println!("{:<22} {:>6} {:>8} {:>10} {:>10}", "Strategy", "Coins", "Entries", "Avg WR%", "Avg PF");
    println!("{}", "-".repeat(60));
    for (name, count, wr, pf, coins) in &strat_list {
        println!("{:<22} {:>6} {:>8} {:>10.1} {:>10.2}",
            name, coins, count, wr / *count as f64, pf / *count as f64);
    }

    println!("\n--- Exit Mode Leaderboard ---\n");
    let mut exit_counts: HashMap<String, (usize, f64, f64)> = HashMap::new();
    for r in &winners {
        let e = exit_counts.entry(r.exit_mode.clone()).or_insert((0, 0.0, 0.0));
        e.0 += 1; e.1 += r.win_rate; e.2 += r.profit_factor;
    }
    let mut exit_list: Vec<(&String, &(usize, f64, f64))> = exit_counts.iter().collect();
    exit_list.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    println!("{:<20} {:>8} {:>10} {:>10}", "Exit Mode", "Count", "Avg WR%", "Avg PF");
    println!("{}", "-".repeat(52));
    for (name, (count, wr, pf)) in &exit_list {
        println!("{:<20} {:>8} {:>10.1} {:>10.2}",
            name, count, wr / *count as f64, pf / *count as f64);
    }
}

fn print_table(results: &[&BacktestResult]) {
    println!("{:<12} {:<22} {:<14} {:>5} {:>6} {:>6} {:>7} {:>7} {:>7}",
        "Coin", "Strategy", "Exit", "Trd", "Win%", "PF", "P&L%", "AvgW%", "AvgL%");
    println!("{}", "-".repeat(100));
    for r in results {
        println!("{:<12} {:<22} {:<14} {:>5} {:>5.1}% {:>6.2} {:>+6.1}% {:>+6.2}% {:>+6.2}%",
            r.coin, r.strategy, r.exit_mode, r.trades, r.win_rate, r.profit_factor,
            r.pnl_pct, r.avg_win_pct, r.avg_loss_pct);
    }
}
