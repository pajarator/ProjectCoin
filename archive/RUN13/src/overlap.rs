//! RUN13 — Signal Overlap Analysis
//! For each strategy, count how many of its entry signals overlap with ctrl_mean_rev
//! vs fire independently. Then check win rate of the non-overlapping signals.

use run13_lib::indicators::{self, Indicators};
use run13_lib::strategies::{self, Candles, StratConfig};

use rayon::prelude::*;
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const FEE_RATE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;
const COST: f64 = FEE_RATE + SLIPPAGE;

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

/// Simulate trades from a set of entry bar indices, using signal exits (SMA20 cross / z-score revert).
/// Returns (wins, losses, win_pcts, loss_pcts)
fn sim_trades_from_entries(
    entries: &[usize], close: &[f64], ind: &Indicators,
) -> (i32, i32, Vec<f64>, Vec<f64>) {
    let n = close.len();
    let round_trip_cost = 2.0 * COST;
    let min_hold = 2usize;
    let max_hold = 30usize;
    let cooldown_period = 5usize;

    let mut win_pcts = Vec::new();
    let mut loss_pcts = Vec::new();
    let mut next_allowed = 0usize;

    for &entry_i in entries {
        if entry_i < next_allowed { continue; }
        let entry_price = close[entry_i];
        let mut exited = false;

        for held in 1..=max_hold {
            let j = entry_i + held;
            if j >= n { break; }
            let raw_pnl = (close[j] - entry_price) / entry_price;

            // SL at 0.3% (coinclaw mode)
            if raw_pnl <= -0.003 {
                let net = -0.003 - round_trip_cost;
                loss_pcts.push(net * 100.0);
                next_allowed = j + cooldown_period;
                exited = true;
                break;
            }

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
                    if net > 0.0 { win_pcts.push(net * 100.0); }
                    else { loss_pcts.push(net * 100.0); }
                    next_allowed = j + cooldown_period;
                    exited = true;
                    break;
                }
            }
        }

        if !exited {
            // Max hold exit
            let j = (entry_i + max_hold).min(n - 1);
            let raw_pnl = (close[j] - entry_price) / entry_price;
            let net = raw_pnl - round_trip_cost;
            if net > 0.0 { win_pcts.push(net * 100.0); }
            else { loss_pcts.push(net * 100.0); }
            next_allowed = j + cooldown_period;
        }
    }

    let wins = win_pcts.len() as i32;
    let losses = loss_pcts.len() as i32;
    (wins, losses, win_pcts, loss_pcts)
}

fn main() {
    println!("=== RUN13 — Signal Overlap & Complementarity Analysis ===");
    println!("For each new strategy: how many signals overlap with ctrl_mean_rev?");
    println!("What's the win rate of the NON-overlapping (unique) signals?\n");

    // Load all coins
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

    let coin_data: Vec<CoinData> = coin_files.iter()
        .filter_map(|f| load_csv(f))
        .filter(|d| d.close.len() > 200)
        .collect();
    println!("Loaded {} coins\n", coin_data.len());

    // Strategies to test (best param per strategy, no z-filter so we see raw signals)
    let test_strats: Vec<StratConfig> = vec![
        StratConfig { name: "pin_bar", p1: 0.6, p2: 0.3, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "hammer", p1: 0.7, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "engulfing", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "qqe", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: 0.0 }, // gamma=0.7
        StratConfig { name: "mass_index", p1: 26.5, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "dema_tema", p1: 1.0, p2: 1.0, p3: 0.0, z_filter: 0.0 }, // TEMA20
        StratConfig { name: "parabolic_sar", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "ichimoku", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: 0.0 },
        StratConfig { name: "kalman_filter", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: 0.0 }, // Q=0.001
    ];

    let ctrl = StratConfig { name: "ctrl_mean_rev", p1: 1.5, p2: 1.2, p3: 0.0, z_filter: 0.0 };

    // Also test with z-filter (the way they'd actually be used)
    let test_strats_filtered: Vec<StratConfig> = vec![
        StratConfig { name: "pin_bar", p1: 0.6, p2: 0.3, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "hammer", p1: 0.7, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "engulfing", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "qqe", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "mass_index", p1: 26.5, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "dema_tema", p1: 1.0, p2: 1.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "parabolic_sar", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "ichimoku", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
        StratConfig { name: "kalman_filter", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.0 },
    ];

    // Overlap window: if two signals fire within N bars of each other, consider them overlapping
    let overlap_window = 5usize;

    // Aggregate across all coins
    struct StratStats {
        name: String,
        total_signals: usize,
        overlap_signals: usize,
        unique_signals: usize,
        // Trades from unique-only entries (coinclaw exit mode)
        unique_wins: i32,
        unique_losses: i32,
        unique_win_pcts: Vec<f64>,
        unique_loss_pcts: Vec<f64>,
        // Trades from ALL entries
        all_wins: i32,
        all_losses: i32,
    }

    println!("=== PART 1: RAW SIGNALS (no z-filter) ===\n");
    println!("Overlap window: {} bars (signals within {} bars of ctrl_mean_rev count as overlapping)\n", overlap_window, overlap_window);

    let results: Vec<Vec<(String, usize, usize, usize, i32, i32, Vec<f64>, Vec<f64>, i32, i32)>> = coin_data.par_iter().map(|data| {
        let n = data.close.len();
        let ind = indicators::compute_all(&data.open, &data.high, &data.low, &data.close, &data.volume);
        let candles = Candles {
            open: &data.open, high: &data.high, low: &data.low,
            close: &data.close, volume: &data.volume,
        };

        // Get ctrl_mean_rev entry bars
        let mut ctrl_entries: Vec<usize> = Vec::new();
        for i in 1..n {
            if strategies::check_entry(&candles, &ind, i, &ctrl) {
                ctrl_entries.push(i);
            }
        }

        let mut coin_results = Vec::new();

        for strat in &test_strats {
            // Get this strategy's entry bars
            let mut strat_entries: Vec<usize> = Vec::new();
            for i in 1..n {
                if strategies::check_entry(&candles, &ind, i, strat) {
                    strat_entries.push(i);
                }
            }

            // Classify each strat entry as overlapping or unique
            let mut overlap_entries: Vec<usize> = Vec::new();
            let mut unique_entries: Vec<usize> = Vec::new();

            for &se in &strat_entries {
                let overlaps = ctrl_entries.iter().any(|&ce| {
                    (se as i64 - ce as i64).unsigned_abs() as usize <= overlap_window
                });
                if overlaps {
                    overlap_entries.push(se);
                } else {
                    unique_entries.push(se);
                }
            }

            // Simulate trades from unique entries only
            let (u_wins, u_losses, u_win_pcts, u_loss_pcts) =
                sim_trades_from_entries(&unique_entries, &data.close, &ind);

            // Simulate trades from all entries
            let (a_wins, a_losses, _, _) =
                sim_trades_from_entries(&strat_entries, &data.close, &ind);

            coin_results.push((
                strat.name.to_string(),
                strat_entries.len(),
                overlap_entries.len(),
                unique_entries.len(),
                u_wins, u_losses, u_win_pcts, u_loss_pcts,
                a_wins, a_losses,
            ));
        }
        coin_results
    }).collect();

    // Aggregate
    let mut agg: Vec<StratStats> = test_strats.iter().map(|s| StratStats {
        name: s.name.to_string(),
        total_signals: 0, overlap_signals: 0, unique_signals: 0,
        unique_wins: 0, unique_losses: 0,
        unique_win_pcts: Vec::new(), unique_loss_pcts: Vec::new(),
        all_wins: 0, all_losses: 0,
    }).collect();

    for coin_results in &results {
        for (idx, (_, total, overlap, unique, u_wins, u_losses, u_wp, u_lp, a_wins, a_losses)) in coin_results.iter().enumerate() {
            agg[idx].total_signals += total;
            agg[idx].overlap_signals += overlap;
            agg[idx].unique_signals += unique;
            agg[idx].unique_wins += u_wins;
            agg[idx].unique_losses += u_losses;
            agg[idx].unique_win_pcts.extend(u_wp);
            agg[idx].unique_loss_pcts.extend(u_lp);
            agg[idx].all_wins += a_wins;
            agg[idx].all_losses += a_losses;
        }
    }

    // Also get ctrl_mean_rev stats
    let ctrl_total: usize = coin_data.iter().map(|data| {
        let ind = indicators::compute_all(&data.open, &data.high, &data.low, &data.close, &data.volume);
        let candles = Candles {
            open: &data.open, high: &data.high, low: &data.low,
            close: &data.close, volume: &data.volume,
        };
        let n = data.close.len();
        let mut count = 0;
        for i in 1..n {
            if strategies::check_entry(&candles, &ind, i, &ctrl) { count += 1; }
        }
        count
    }).sum();

    println!("ctrl_mean_rev total signals across all coins: {}\n", ctrl_total);

    println!("{:<18} {:>7} {:>7} {:>7} {:>7}  {:>7} {:>7} {:>6} {:>8} {:>8}",
        "Strategy", "Total", "Overlap", "Unique", "Uniq%", "U_Trd", "U_WR%", "U_PF", "All_Trd", "All_WR%");
    println!("{}", "-".repeat(105));

    for s in &agg {
        let uniq_pct = if s.total_signals > 0 { s.unique_signals as f64 / s.total_signals as f64 * 100.0 } else { 0.0 };
        let u_total = s.unique_wins + s.unique_losses;
        let u_wr = if u_total > 0 { s.unique_wins as f64 / u_total as f64 * 100.0 } else { 0.0 };
        let u_tw: f64 = s.unique_win_pcts.iter().sum();
        let u_tl: f64 = s.unique_loss_pcts.iter().map(|x| x.abs()).sum();
        let u_pf = if u_tl > 0.0 { u_tw / u_tl } else { 0.0 };
        let a_total = s.all_wins + s.all_losses;
        let a_wr = if a_total > 0 { s.all_wins as f64 / a_total as f64 * 100.0 } else { 0.0 };

        println!("{:<18} {:>7} {:>7} {:>7} {:>6.1}%  {:>7} {:>6.1}% {:>6.2} {:>8} {:>7.1}%",
            s.name, s.total_signals, s.overlap_signals, s.unique_signals, uniq_pct,
            u_total, u_wr, u_pf, a_total, a_wr);
    }

    // Part 2: With z-filter
    println!("\n\n=== PART 2: WITH z < -1.0 FILTER ===\n");

    let results2: Vec<Vec<(String, usize, usize, usize, i32, i32, Vec<f64>, Vec<f64>, i32, i32)>> = coin_data.par_iter().map(|data| {
        let n = data.close.len();
        let ind = indicators::compute_all(&data.open, &data.high, &data.low, &data.close, &data.volume);
        let candles = Candles {
            open: &data.open, high: &data.high, low: &data.low,
            close: &data.close, volume: &data.volume,
        };

        let mut ctrl_entries: Vec<usize> = Vec::new();
        for i in 1..n {
            if strategies::check_entry(&candles, &ind, i, &ctrl) {
                ctrl_entries.push(i);
            }
        }

        let mut coin_results = Vec::new();
        for strat in &test_strats_filtered {
            let mut strat_entries: Vec<usize> = Vec::new();
            for i in 1..n {
                if strategies::check_entry(&candles, &ind, i, strat) {
                    strat_entries.push(i);
                }
            }

            let mut unique_entries: Vec<usize> = Vec::new();
            let mut overlap_count = 0usize;
            for &se in &strat_entries {
                let overlaps = ctrl_entries.iter().any(|&ce| {
                    (se as i64 - ce as i64).unsigned_abs() as usize <= overlap_window
                });
                if overlaps { overlap_count += 1; }
                else { unique_entries.push(se); }
            }

            let (u_wins, u_losses, u_win_pcts, u_loss_pcts) =
                sim_trades_from_entries(&unique_entries, &data.close, &ind);
            let (a_wins, a_losses, _, _) =
                sim_trades_from_entries(&strat_entries, &data.close, &ind);

            coin_results.push((
                strat.name.to_string(),
                strat_entries.len(),
                overlap_count,
                unique_entries.len(),
                u_wins, u_losses, u_win_pcts, u_loss_pcts,
                a_wins, a_losses,
            ));
        }
        coin_results
    }).collect();

    let mut agg2: Vec<StratStats> = test_strats_filtered.iter().map(|s| StratStats {
        name: s.name.to_string(),
        total_signals: 0, overlap_signals: 0, unique_signals: 0,
        unique_wins: 0, unique_losses: 0,
        unique_win_pcts: Vec::new(), unique_loss_pcts: Vec::new(),
        all_wins: 0, all_losses: 0,
    }).collect();

    for coin_results in &results2 {
        for (idx, (_, total, overlap, unique, u_wins, u_losses, u_wp, u_lp, a_wins, a_losses)) in coin_results.iter().enumerate() {
            agg2[idx].total_signals += total;
            agg2[idx].overlap_signals += overlap;
            agg2[idx].unique_signals += unique;
            agg2[idx].unique_wins += u_wins;
            agg2[idx].unique_losses += u_losses;
            agg2[idx].unique_win_pcts.extend(u_wp);
            agg2[idx].unique_loss_pcts.extend(u_lp);
            agg2[idx].all_wins += a_wins;
            agg2[idx].all_losses += a_losses;
        }
    }

    println!("{:<18} {:>7} {:>7} {:>7} {:>7}  {:>7} {:>7} {:>6} {:>8} {:>8}",
        "Strategy", "Total", "Overlap", "Unique", "Uniq%", "U_Trd", "U_WR%", "U_PF", "All_Trd", "All_WR%");
    println!("{}", "-".repeat(105));

    for s in &agg2 {
        let uniq_pct = if s.total_signals > 0 { s.unique_signals as f64 / s.total_signals as f64 * 100.0 } else { 0.0 };
        let u_total = s.unique_wins + s.unique_losses;
        let u_wr = if u_total > 0 { s.unique_wins as f64 / u_total as f64 * 100.0 } else { 0.0 };
        let u_tw: f64 = s.unique_win_pcts.iter().sum();
        let u_tl: f64 = s.unique_loss_pcts.iter().map(|x| x.abs()).sum();
        let u_pf = if u_tl > 0.0 { u_tw / u_tl } else { 0.0 };
        let a_total = s.all_wins + s.all_losses;
        let a_wr = if a_total > 0 { s.all_wins as f64 / a_total as f64 * 100.0 } else { 0.0 };

        println!("{:<18} {:>7} {:>7} {:>7} {:>6.1}%  {:>7} {:>6.1}% {:>6.2} {:>8} {:>7.1}%",
            s.name, s.total_signals, s.overlap_signals, s.unique_signals, uniq_pct,
            u_total, u_wr, u_pf, a_total, a_wr);
    }

    // Part 3: Per-coin breakdown for strategies with best unique WR
    println!("\n\n=== PART 3: PER-COIN UNIQUE SIGNAL PERFORMANCE (z<-1.0, coinclaw exits) ===\n");
    println!("Showing strategies with overall unique WR >= 50%\n");

    // Identify which strategies to drill into
    let good_strats: Vec<usize> = agg2.iter().enumerate()
        .filter(|(_, s)| {
            let u_total = s.unique_wins + s.unique_losses;
            u_total >= 10 && (s.unique_wins as f64 / u_total as f64) >= 0.50
        })
        .map(|(idx, _)| idx)
        .collect();

    if good_strats.is_empty() {
        println!("No strategy has >= 50% unique WR with >= 10 trades.");
        println!("\nShowing all strategies anyway:\n");
        for (idx, s) in agg2.iter().enumerate() {
            let u_total = s.unique_wins + s.unique_losses;
            if u_total < 5 { continue; }
            println!("  {} — {} unique trades, {:.1}% WR",
                s.name, u_total, if u_total > 0 { s.unique_wins as f64 / u_total as f64 * 100.0 } else { 0.0 });
        }
    }

    for &idx in &good_strats {
        let strat = &test_strats_filtered[idx];
        println!("--- {} (z<-1.0) ---\n", strat.name);
        println!("{:<12} {:>7} {:>7} {:>7} {:>7} {:>8} {:>6}",
            "Coin", "Signals", "Unique", "U_Trd", "U_Wins", "U_WR%", "U_PF");
        println!("{}", "-".repeat(60));

        for (ci, coin_results) in results2.iter().enumerate() {
            let (_, total, _, unique, u_wins, u_losses, ref u_wp, ref u_lp, _, _) = coin_results[idx];
            let u_total = u_wins + u_losses;
            if total == 0 { continue; }
            let u_wr = if u_total > 0 { u_wins as f64 / u_total as f64 * 100.0 } else { 0.0 };
            let u_tw: f64 = u_wp.iter().sum();
            let u_tl: f64 = u_lp.iter().map(|x| x.abs()).sum();
            let u_pf = if u_tl > 0.0 { u_tw / u_tl } else { 0.0 };

            println!("{:<12} {:>7} {:>7} {:>7} {:>7} {:>7.1}% {:>6.2}",
                coin_data[ci].name, total, unique, u_total, u_wins, u_wr, u_pf);
        }
        println!();
    }

    println!("Done.");
}
