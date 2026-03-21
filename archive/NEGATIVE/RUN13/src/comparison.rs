//! RUN13.3 — Side-by-Side Comparison
//! For each coin: "current assignment only" vs "current + complementary signal"
//! Tests whether adding laguerre_rsi / kalman_filter / kst_cross as secondary
//! entry alongside ctrl_mean_rev captures more profitable trades.

use run13_lib::indicators::{self, Indicators};
use run13_lib::strategies::{self, Candles, StratConfig};

use serde::{Deserialize, Serialize};
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run13/run13_3_results.json";

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

#[derive(Debug, Clone, Copy)]
enum ExitReason { StopLoss, Sma20Cross, ZScoreRevert, MaxHold, EndOfData }

impl std::fmt::Display for ExitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::StopLoss => write!(f, "SL"),
            Self::Sma20Cross => write!(f, "SMA20"),
            Self::ZScoreRevert => write!(f, "Z_REV"),
            Self::MaxHold => write!(f, "MAX_HOLD"),
            Self::EndOfData => write!(f, "EOD"),
        }
    }
}

struct Trade {
    entry_idx: usize,
    net_pnl_pct: f64,
    exit_reason: ExitReason,
    candles_held: usize,
    source: &'static str, // "ctrl" or "complement"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinResult {
    coin: String,
    mode: String, // "baseline" or "baseline+complement"
    complement_name: String,
    total_trades: i32,
    ctrl_trades: i32,
    complement_trades: i32,
    wins: i32,
    win_rate: f64,
    profit_factor: f64,
    pnl_pct: f64,
    max_drawdown_pct: f64,
    avg_win_pct: f64,
    avg_loss_pct: f64,
    avg_hold: f64,
    sl_exits: i32,
    sma20_exits: i32,
    z_rev_exits: i32,
    max_hold_exits: i32,
    // Complement-only stats
    comp_wins: i32,
    comp_losses: i32,
    comp_win_rate: f64,
    comp_pf: f64,
    // Per-month
    month_trades: Vec<i32>,
    month_wr: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComparisonOutput {
    run: String,
    results: Vec<CoinResult>,
}

fn load_csv(path: &str) -> Option<CoinData> {
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path).ok()?;
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

/// Run detailed backtest with entry sources tracked.
/// entries: Vec<(bar_index, source_label)> — must be sorted by bar_index.
fn run_backtest(
    close: &[f64], ind: &Indicators,
    entries: &[(usize, &'static str)],
    month_size: usize,
) -> Vec<Trade> {
    let n = close.len();
    let round_trip_cost = 2.0 * COST;
    let sl = 0.003;
    let min_hold = 2usize;
    let max_hold = 30usize;
    let cooldown_period = 5usize;

    let mut trades: Vec<Trade> = Vec::new();
    let mut next_allowed = 0usize;

    for &(entry_i, source) in entries {
        if entry_i < next_allowed || entry_i >= n { continue; }
        let entry_price = close[entry_i];
        let mut exited = false;

        for held in 1..=max_hold {
            let j = entry_i + held;
            if j >= n { break; }
            let raw_pnl = (close[j] - entry_price) / entry_price;

            if raw_pnl <= -sl {
                let net = -sl - round_trip_cost;
                trades.push(Trade { entry_idx: entry_i, net_pnl_pct: net * 100.0, exit_reason: ExitReason::StopLoss, candles_held: held, source });
                next_allowed = j + cooldown_period;
                exited = true;
                break;
            }

            if held >= min_hold {
                let mut reason: Option<ExitReason> = None;
                if !ind.sma20[j].is_nan() && close[j] > ind.sma20[j]
                    && j > 0 && !ind.sma20[j-1].is_nan() && close[j-1] <= ind.sma20[j-1] {
                    reason = Some(ExitReason::Sma20Cross);
                }
                if reason.is_none() && !ind.z_score[j].is_nan() && ind.z_score[j] > 0.5 {
                    reason = Some(ExitReason::ZScoreRevert);
                }
                if let Some(r) = reason {
                    let net = raw_pnl - round_trip_cost;
                    trades.push(Trade { entry_idx: entry_i, net_pnl_pct: net * 100.0, exit_reason: r, candles_held: held, source });
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
            trades.push(Trade { entry_idx: entry_i, net_pnl_pct: net * 100.0, exit_reason: ExitReason::MaxHold, candles_held: max_hold.min(j - entry_i), source });
            next_allowed = j + cooldown_period;
        }
    }
    trades
}

fn summarize_trades(trades: &[Trade], coin: &str, mode: &str, comp_name: &str, month_size: usize) -> CoinResult {
    let position_size = 0.10;
    let mut balance = 10000.0;
    let mut peak = 10000.0;
    let mut max_dd = 0.0;

    for t in trades {
        balance += balance * position_size * t.net_pnl_pct / 100.0;
        if balance > peak { peak = balance; }
        let dd = (peak - balance) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    let wins: Vec<&Trade> = trades.iter().filter(|t| t.net_pnl_pct > 0.0).collect();
    let losses: Vec<&Trade> = trades.iter().filter(|t| t.net_pnl_pct <= 0.0).collect();
    let total = trades.len() as i32;
    let win_count = wins.len() as i32;
    let wr = if total > 0 { win_count as f64 / total as f64 * 100.0 } else { 0.0 };
    let tw: f64 = wins.iter().map(|t| t.net_pnl_pct).sum();
    let tl: f64 = losses.iter().map(|t| t.net_pnl_pct.abs()).sum();
    let pf = if tl > 0.0 { tw / tl } else { 0.0 };
    let avg_win = if !wins.is_empty() { tw / wins.len() as f64 } else { 0.0 };
    let avg_loss = if !losses.is_empty() { losses.iter().map(|t| t.net_pnl_pct).sum::<f64>() / losses.len() as f64 } else { 0.0 };
    let avg_hold = if !trades.is_empty() { trades.iter().map(|t| t.candles_held as f64).sum::<f64>() / trades.len() as f64 } else { 0.0 };
    let pnl = (balance - 10000.0) / 10000.0 * 100.0;

    let ctrl_trades = trades.iter().filter(|t| t.source == "ctrl").count() as i32;
    let complement_trades = trades.iter().filter(|t| t.source == "complement").count() as i32;

    let sl_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::StopLoss)).count() as i32;
    let sma20_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::Sma20Cross)).count() as i32;
    let z_rev_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::ZScoreRevert)).count() as i32;
    let max_hold_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::MaxHold)).count() as i32;

    // Complement-only stats
    let comp_trades_vec: Vec<&Trade> = trades.iter().filter(|t| t.source == "complement").collect();
    let comp_wins = comp_trades_vec.iter().filter(|t| t.net_pnl_pct > 0.0).count() as i32;
    let comp_losses_count = comp_trades_vec.iter().filter(|t| t.net_pnl_pct <= 0.0).count() as i32;
    let comp_total = comp_wins + comp_losses_count;
    let comp_wr = if comp_total > 0 { comp_wins as f64 / comp_total as f64 * 100.0 } else { 0.0 };
    let comp_tw: f64 = comp_trades_vec.iter().filter(|t| t.net_pnl_pct > 0.0).map(|t| t.net_pnl_pct).sum();
    let comp_tl: f64 = comp_trades_vec.iter().filter(|t| t.net_pnl_pct <= 0.0).map(|t| t.net_pnl_pct.abs()).sum();
    let comp_pf = if comp_tl > 0.0 { comp_tw / comp_tl } else { 0.0 };

    // Per-month
    let num_months = 5;
    let mut month_trades_count = vec![0i32; num_months];
    let mut month_wins = vec![0.0f64; num_months];
    for t in trades {
        let m = (t.entry_idx / month_size).min(num_months - 1);
        month_trades_count[m] += 1;
        if t.net_pnl_pct > 0.0 { month_wins[m] += 1.0; }
    }
    let mut month_wr = vec![0.0; num_months];
    for m in 0..num_months {
        if month_trades_count[m] > 0 {
            month_wr[m] = month_wins[m] / month_trades_count[m] as f64 * 100.0;
        }
    }

    CoinResult {
        coin: coin.to_string(),
        mode: mode.to_string(),
        complement_name: comp_name.to_string(),
        total_trades: total,
        ctrl_trades,
        complement_trades,
        wins: win_count,
        win_rate: wr,
        profit_factor: pf,
        pnl_pct: pnl,
        max_drawdown_pct: max_dd,
        avg_win_pct: avg_win,
        avg_loss_pct: avg_loss,
        avg_hold,
        sl_exits, sma20_exits, z_rev_exits, max_hold_exits,
        comp_wins,
        comp_losses: comp_losses_count,
        comp_win_rate: comp_wr,
        comp_pf,
        month_trades: month_trades_count,
        month_wr,
    }
}

fn main() {
    println!("=== RUN13.3 — Side-by-Side: Baseline vs Baseline + Complement ===\n");

    let ctrl = StratConfig { name: "ctrl_mean_rev", p1: 1.5, p2: 1.2, p3: 0.0, z_filter: 0.0 };

    // Per-coin complement assignments from WF results
    struct CoinAssignment {
        coin: &'static str,
        comp_name: &'static str,
        cfg: StratConfig,
    }

    let assignments = vec![
        CoinAssignment { coin: "ADA_USDT", comp_name: "laguerre_rsi_g05_z-1.0",
            cfg: StratConfig { name: "laguerre_rsi", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 } },
        CoinAssignment { coin: "ALGO_USDT", comp_name: "laguerre_rsi_g08_z-1.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "ATOM_USDT", comp_name: "laguerre_rsi_g06_z-1.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "AVAX_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "BNB_USDT", comp_name: "kst_cross_z-1.0",
            cfg: StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -1.0 } },
        CoinAssignment { coin: "BTC_USDT", comp_name: "kst_cross_z-0.5",
            cfg: StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 } },
        CoinAssignment { coin: "DASH_USDT", comp_name: "kst_cross_z-0.5",
            cfg: StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 } },
        CoinAssignment { coin: "DOGE_USDT", comp_name: "kst_cross_z-0.5",
            cfg: StratConfig { name: "kst_cross", p1: 0.0, p2: 0.0, p3: 0.0, z_filter: -0.5 } },
        CoinAssignment { coin: "DOT_USDT", comp_name: "laguerre_rsi_g07_z-1.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "ETH_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "LINK_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "LTC_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "NEAR_USDT", comp_name: "laguerre_rsi_g08_z-0.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -0.5 } },
        CoinAssignment { coin: "SHIB_USDT", comp_name: "laguerre_rsi_g08_z-1.0",
            cfg: StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -1.0 } },
        CoinAssignment { coin: "SOL_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "TRX_USDT", comp_name: "laguerre_rsi_g08_z-1.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "UNI_USDT", comp_name: "kalman_Q0001_z-1.5",
            cfg: StratConfig { name: "kalman_filter", p1: 2.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "XLM_USDT", comp_name: "laguerre_rsi_g06_z-1.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 1.0, p2: 0.0, p3: 0.0, z_filter: -1.5 } },
        CoinAssignment { coin: "XRP_USDT", comp_name: "laguerre_rsi_g08_z-0.5",
            cfg: StratConfig { name: "laguerre_rsi", p1: 3.0, p2: 0.0, p3: 0.0, z_filter: -0.5 } },
    ];

    let mut all_results: Vec<CoinResult> = Vec::new();

    for assign in &assignments {
        let path = format!("{}/{}_15m_5months.csv", DATA_DIR, assign.coin);
        let data = match load_csv(&path) {
            Some(d) => d,
            None => { println!("SKIP: {}", assign.coin); continue; }
        };
        let n = data.close.len();
        let ind = indicators::compute_all(&data.open, &data.high, &data.low, &data.close, &data.volume);
        let candles = Candles {
            open: &data.open, high: &data.high, low: &data.low,
            close: &data.close, volume: &data.volume,
        };
        let month_size = n / 5;

        // Get ctrl entries
        let mut ctrl_entries: Vec<usize> = Vec::new();
        for i in 1..n {
            if strategies::check_entry(&candles, &ind, i, &ctrl) {
                ctrl_entries.push(i);
            }
        }

        // Get complement entries (unique only)
        let mut comp_entries_raw: Vec<usize> = Vec::new();
        for i in 1..n {
            if strategies::check_entry(&candles, &ind, i, &assign.cfg) {
                comp_entries_raw.push(i);
            }
        }
        let comp_entries: Vec<usize> = comp_entries_raw.iter().filter(|&&se| {
            !ctrl_entries.iter().any(|&ce| {
                (se as i64 - ce as i64).unsigned_abs() as usize <= OVERLAP_WINDOW
            })
        }).copied().collect();

        // Mode 1: Baseline (ctrl only)
        let baseline_entries: Vec<(usize, &'static str)> = ctrl_entries.iter()
            .map(|&i| (i, "ctrl"))
            .collect();
        let baseline_trades = run_backtest(&data.close, &ind, &baseline_entries, month_size);
        let baseline_result = summarize_trades(&baseline_trades, &data.name, "baseline", "none", month_size);

        // Mode 2: Baseline + complement
        let mut combined_entries: Vec<(usize, &'static str)> = ctrl_entries.iter()
            .map(|&i| (i, "ctrl"))
            .collect();
        for &i in &comp_entries {
            combined_entries.push((i, "complement"));
        }
        combined_entries.sort_by_key(|&(i, _)| i);

        let combined_trades = run_backtest(&data.close, &ind, &combined_entries, month_size);
        let combined_result = summarize_trades(&combined_trades, &data.name, "baseline+comp", assign.comp_name, month_size);

        println!("{:<12} ctrl={:>3} signals, comp={:>3} unique signals ({}) → baseline T={}, combined T={}",
            data.name, ctrl_entries.len(), comp_entries.len(), assign.comp_name,
            baseline_result.total_trades, combined_result.total_trades);

        all_results.push(baseline_result);
        all_results.push(combined_result);
    }

    // ========================================
    // SUMMARY
    // ========================================
    println!("\n{}", "=".repeat(130));
    println!("=== COMPARISON TABLE ===\n");
    println!("{:<12} {:<18} {:>20} {:>5} {:>3}/{:>3} {:>6} {:>6} {:>7} {:>6} {:>7} {:>7}",
        "Coin", "Mode", "Complement", "Trd", "Ctr", "Cmp", "WR%", "PF", "P&L%", "MaxDD", "AvgW%", "AvgL%");
    println!("{}", "-".repeat(130));

    for r in &all_results {
        println!("{:<12} {:<18} {:>20} {:>5} {:>3}/{:>3} {:>5.1}% {:>6.2} {:>+6.1}% {:>5.1}% {:>+6.2}% {:>+6.2}%",
            r.coin, r.mode, r.complement_name, r.total_trades,
            r.ctrl_trades, r.complement_trades,
            r.win_rate, r.profit_factor, r.pnl_pct, r.max_drawdown_pct,
            r.avg_win_pct, r.avg_loss_pct);
    }

    // Head-to-head
    println!("\n{}", "=".repeat(130));
    println!("=== HEAD-TO-HEAD PER COIN ===\n");
    println!("{:<12} {:>20} {:>5} {:>6} {:>6} {:>7}  {:>5} {:>6} {:>6} {:>7}  {:>6} {:>6} {:>7}  {:>8}",
        "Coin", "Complement", "B_T", "B_WR%", "B_PF", "B_P&L",
        "C_T", "C_WR%", "C_PF", "C_P&L",
        "dWR%", "dPF", "dP&L", "Verdict");
    println!("{}", "-".repeat(145));

    let mut total_base_pnl = 0.0;
    let mut total_comp_pnl = 0.0;
    let mut better_count = 0;
    let mut worse_count = 0;
    let mut coin_count = 0;

    let coins: Vec<String> = {
        let mut c: Vec<String> = all_results.iter().map(|r| r.coin.clone()).collect();
        c.sort(); c.dedup(); c
    };

    for coin in &coins {
        let base = all_results.iter().find(|r| &r.coin == coin && r.mode == "baseline");
        let comp = all_results.iter().find(|r| &r.coin == coin && r.mode == "baseline+comp");
        if let (Some(b), Some(c)) = (base, comp) {
            let dwr = c.win_rate - b.win_rate;
            let dpf = c.profit_factor - b.profit_factor;
            let dpnl = c.pnl_pct - b.pnl_pct;

            let verdict = if c.pnl_pct > b.pnl_pct && c.win_rate >= b.win_rate - 2.0 {
                better_count += 1;
                "BETTER"
            } else if (c.pnl_pct - b.pnl_pct).abs() < 0.3 {
                "NEUTRAL"
            } else {
                worse_count += 1;
                "WORSE"
            };

            println!("{:<12} {:>20} {:>5} {:>5.1}% {:>6.2} {:>+6.1}%  {:>5} {:>5.1}% {:>6.2} {:>+6.1}%  {:>+5.1}% {:>+5.2} {:>+6.1}%  {:>8}",
                coin, c.complement_name,
                b.total_trades, b.win_rate, b.profit_factor, b.pnl_pct,
                c.total_trades, c.win_rate, c.profit_factor, c.pnl_pct,
                dwr, dpf, dpnl, verdict);

            total_base_pnl += b.pnl_pct;
            total_comp_pnl += c.pnl_pct;
            coin_count += 1;
        }
    }

    println!("\n{}", "=".repeat(130));
    println!("=== PORTFOLIO SUMMARY ===\n");
    println!("Coins tested: {}", coin_count);
    println!("Baseline total P&L:        {:>+7.1}% ({:>+.2}% avg/coin)", total_base_pnl, total_base_pnl / coin_count as f64);
    println!("Baseline+Comp total P&L:   {:>+7.1}% ({:>+.2}% avg/coin)", total_comp_pnl, total_comp_pnl / coin_count as f64);
    println!("Difference:                {:>+7.1}%", total_comp_pnl - total_base_pnl);
    println!("Better: {}, Worse: {}, Neutral: {}", better_count, worse_count, coin_count - better_count - worse_count);

    // Complement-only breakdown
    println!("\n{}", "=".repeat(100));
    println!("=== COMPLEMENT-ONLY TRADE STATS ===\n");
    println!("{:<12} {:>20} {:>5} {:>5} {:>6} {:>6}",
        "Coin", "Complement", "Trd", "Wins", "WR%", "PF");
    println!("{}", "-".repeat(60));
    let mut total_comp_trades = 0;
    let mut total_comp_wins = 0;
    for r in all_results.iter().filter(|r| r.mode == "baseline+comp") {
        if r.complement_trades > 0 {
            println!("{:<12} {:>20} {:>5} {:>5} {:>5.1}% {:>6.2}",
                r.coin, r.complement_name, r.comp_wins + r.comp_losses,
                r.comp_wins, r.comp_win_rate, r.comp_pf);
            total_comp_trades += r.comp_wins + r.comp_losses;
            total_comp_wins += r.comp_wins;
        }
    }
    let overall_comp_wr = if total_comp_trades > 0 { total_comp_wins as f64 / total_comp_trades as f64 * 100.0 } else { 0.0 };
    println!("\nOverall complement trades: {}, wins: {}, WR: {:.1}%", total_comp_trades, total_comp_wins, overall_comp_wr);

    // Per-month
    println!("\n{}", "=".repeat(100));
    println!("=== PER-MONTH WIN RATE (baseline+comp) ===\n");
    println!("{:<12} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>6}",
        "Coin", "M1", "M2", "M3", "M4", "M5", "Avg");
    println!("{}", "-".repeat(70));
    for r in all_results.iter().filter(|r| r.mode == "baseline+comp") {
        print!("{:<12}", r.coin);
        for m in 0..5 {
            if r.month_trades[m] > 0 {
                print!(" {:>5.1}%/{}", r.month_wr[m], r.month_trades[m]);
            } else {
                print!("    -/0 ");
            }
        }
        println!("  {:>5.1}%", r.win_rate);
    }

    // Final recommendation
    println!("\n{}", "=".repeat(100));
    println!("=== RECOMMENDATION ===\n");
    let net_gain = total_comp_pnl - total_base_pnl;
    if net_gain > 0.0 && better_count > worse_count {
        println!("ADD complementary signals to COINCLAW.");
        println!("Net portfolio gain: {:+.1}% across {} coins ({} better, {} worse).",
            net_gain, coin_count, better_count, worse_count);
        println!("\nAssignments:");
        for r in all_results.iter().filter(|r| r.mode == "baseline+comp") {
            let base = all_results.iter().find(|b| b.coin == r.coin && b.mode == "baseline");
            if let Some(b) = base {
                if r.pnl_pct > b.pnl_pct {
                    println!("  {} → add {} (P&L {:+.1}% → {:+.1}%)",
                        r.coin, r.complement_name, b.pnl_pct, r.pnl_pct);
                }
            }
        }
    } else {
        println!("NO net benefit from adding complementary signals.");
        println!("Net change: {:+.1}% ({} better, {} worse).", net_gain, better_count, worse_count);
    }

    // Save
    let output = ComparisonOutput { run: "RUN13.3".to_string(), results: all_results };
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("\nResults saved to {}", RESULTS_FILE);
    }
    println!("\nDone.");
}
