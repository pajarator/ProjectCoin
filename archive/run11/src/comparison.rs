//! RUN11c.3 — Side-by-Side Comparison (DASH)
//! Compares DASH's current VwapReversion against ou_mean_rev (RUN11c.2 winner)
//!
//! Strategies tested:
//!   1. vwap_rev (baseline) — Z < -1.5, price < VWAP, vol > 1.2× avg
//!   2. ctrl_mean_rev — Z < -1.5, vol > 1.2× avg
//!   3. ou_mean_rev (hl=10, dev=1.5) — broader, more trades
//!   4. ou_mean_rev (hl=10, dev=2.0) — tighter, higher WR
//!   5. ou_mean_rev (hl=10, dev=2.5) — strictest, highest WR

use run11c_lib::indicators::{self, Indicators};
use run11c_lib::strategies::{self, Candles, StratConfig};

use serde::{Deserialize, Serialize};
use std::fs;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run11c_3_results.json";

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StratResult {
    strategy: String,
    exit_mode: String,
    trades: i32,
    wins: i32,
    win_rate: f64,
    profit_factor: f64,
    pnl_pct: f64,
    avg_win_pct: f64,
    avg_loss_pct: f64,
    avg_hold: f64,
    sl_exits: i32,
    sma20_exits: i32,
    z_rev_exits: i32,
    max_hold_exits: i32,
    month_wr: Vec<f64>,
    month_trades: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComparisonOutput {
    run: String,
    coin: String,
    results: Vec<StratResult>,
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

fn rolling_sum_local(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < window { return out; }
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        if !data[i].is_nan() { sum += data[i]; count += 1; }
        if i >= window {
            if !data[i - window].is_nan() { sum -= data[i - window]; count -= 1; }
        }
        if i + 1 >= window && count == window { out[i] = sum; }
    }
    out
}

fn compute_rolling_vwap(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], window: usize) -> Vec<f64> {
    let n = close.len();
    let mut tp_vol = vec![0.0; n];
    for i in 0..n {
        tp_vol[i] = (high[i] + low[i] + close[i]) / 3.0 * volume[i];
    }
    let tp_vol_sum = rolling_sum_local(&tp_vol, window);
    let vol_sum = rolling_sum_local(volume, window);
    let mut vwap = vec![f64::NAN; n];
    for i in 0..n {
        if !tp_vol_sum[i].is_nan() && !vol_sum[i].is_nan() && vol_sum[i] > 0.0 {
            vwap[i] = tp_vol_sum[i] / vol_sum[i];
        }
    }
    vwap
}

/// COINCLAW VwapReversion entry
fn vwap_rev_entry(ind: &Indicators, vwap: &[f64], close: &[f64], i: usize) -> bool {
    if i < 96 { return false; }
    if ind.z_score[i].is_nan() || ind.vol_ratio[i].is_nan() || vwap[i].is_nan() { return false; }
    if !ind.sma20[i].is_nan() && close[i] > ind.sma20[i] { return false; }
    if ind.z_score[i] > 0.5 { return false; }
    ind.z_score[i] < -1.5 && close[i] < vwap[i] && ind.vol_ratio[i] > 1.2
}

fn run_backtest_detailed(
    data: &CoinData, ind: &Indicators, vwap: &[f64],
    strategy_name: &str, cfg: Option<&StratConfig>,
    exit: &ExitMode, month_size: usize,
) -> StratResult {
    let n = data.close.len();
    let candles = Candles {
        open: &data.open, high: &data.high, low: &data.low,
        close: &data.close, volume: &data.volume,
    };
    let round_trip_cost = 2.0 * COST;
    let position_size = 0.10;
    let cooldown_period: usize = 5;

    let mut balance = 10000.0;
    let mut in_position = false;
    let mut entry_price = 0.0;
    let mut entry_idx = 0usize;
    let mut candles_held: usize = 0;
    let mut cooldown: usize = 0;
    let mut trades: Vec<Trade> = Vec::new();

    for i in 1..n {
        if !in_position {
            if cooldown > 0 { cooldown -= 1; continue; }
            let entry = match strategy_name {
                "vwap_rev" => vwap_rev_entry(ind, vwap, &data.close, i),
                _ => cfg.map_or(false, |c| strategies::check_entry(&candles, ind, i, c)),
            };
            if entry {
                entry_price = data.close[i];
                entry_idx = i;
                in_position = true;
                candles_held = 0;
            }
        } else {
            candles_held += 1;
            let raw_pnl = (data.close[i] - entry_price) / entry_price;

            if exit.sl > 0.0 && raw_pnl <= -exit.sl {
                let net = -exit.sl - round_trip_cost;
                balance += balance * position_size * net;
                trades.push(Trade { entry_idx, net_pnl_pct: net * 100.0, exit_reason: ExitReason::StopLoss, candles_held });
                in_position = false; cooldown = cooldown_period; continue;
            }

            if candles_held >= exit.min_hold {
                let mut reason: Option<ExitReason> = None;
                if !ind.sma20[i].is_nan() && data.close[i] > ind.sma20[i]
                    && i > 0 && !ind.sma20[i-1].is_nan() && data.close[i-1] <= ind.sma20[i-1] {
                    reason = Some(ExitReason::Sma20Cross);
                }
                if reason.is_none() && !ind.z_score[i].is_nan() && ind.z_score[i] > 0.5 {
                    reason = Some(ExitReason::ZScoreRevert);
                }
                if let Some(r) = reason {
                    let net = raw_pnl - round_trip_cost;
                    balance += balance * position_size * net;
                    trades.push(Trade { entry_idx, net_pnl_pct: net * 100.0, exit_reason: r, candles_held });
                    in_position = false; cooldown = cooldown_period; continue;
                }
            }

            if exit.max_hold > 0 && candles_held >= exit.max_hold {
                let net = raw_pnl - round_trip_cost;
                balance += balance * position_size * net;
                trades.push(Trade { entry_idx, net_pnl_pct: net * 100.0, exit_reason: ExitReason::MaxHold, candles_held });
                in_position = false; cooldown = cooldown_period;
            }
        }
    }

    if in_position {
        let raw_pnl = (data.close[n-1] - entry_price) / entry_price;
        let net = raw_pnl - round_trip_cost;
        balance += balance * position_size * net;
        trades.push(Trade { entry_idx, net_pnl_pct: net * 100.0, exit_reason: ExitReason::EndOfData, candles_held });
    }

    let wins: Vec<&Trade> = trades.iter().filter(|t| t.net_pnl_pct > 0.0).collect();
    let losses: Vec<&Trade> = trades.iter().filter(|t| t.net_pnl_pct <= 0.0).collect();
    let total = trades.len() as i32;
    let win_count = wins.len() as i32;
    let win_rate = if total > 0 { win_count as f64 / total as f64 * 100.0 } else { 0.0 };
    let total_win: f64 = wins.iter().map(|t| t.net_pnl_pct).sum();
    let total_loss: f64 = losses.iter().map(|t| t.net_pnl_pct.abs()).sum();
    let pf = if total_loss > 0.0 { total_win / total_loss } else { 0.0 };
    let avg_win = if !wins.is_empty() { total_win / wins.len() as f64 } else { 0.0 };
    let avg_loss = if !losses.is_empty() { losses.iter().map(|t| t.net_pnl_pct).sum::<f64>() / losses.len() as f64 } else { 0.0 };
    let avg_hold = if !trades.is_empty() { trades.iter().map(|t| t.candles_held as f64).sum::<f64>() / trades.len() as f64 } else { 0.0 };
    let pnl_pct = (balance - 10000.0) / 10000.0 * 100.0;

    let sl_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::StopLoss)).count() as i32;
    let sma20_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::Sma20Cross)).count() as i32;
    let z_rev_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::ZScoreRevert)).count() as i32;
    let max_hold_exits = trades.iter().filter(|t| matches!(t.exit_reason, ExitReason::MaxHold)).count() as i32;

    let num_months = 5;
    let mut month_wr = vec![0.0; num_months];
    let mut month_trades_count = vec![0i32; num_months];
    for t in &trades {
        let m = (t.entry_idx / month_size).min(num_months - 1);
        month_trades_count[m] += 1;
        if t.net_pnl_pct > 0.0 { month_wr[m] += 1.0; }
    }
    for m in 0..num_months {
        if month_trades_count[m] > 0 {
            month_wr[m] = month_wr[m] / month_trades_count[m] as f64 * 100.0;
        }
    }

    StratResult {
        strategy: strategy_name.to_string(), exit_mode: exit.name.to_string(),
        trades: total, wins: win_count, win_rate, profit_factor: pf, pnl_pct,
        avg_win_pct: avg_win, avg_loss_pct: avg_loss, avg_hold,
        sl_exits, sma20_exits, z_rev_exits, max_hold_exits,
        month_wr, month_trades: month_trades_count,
    }
}

fn main() {
    println!("=== RUN11c.3 — Side-by-Side Comparison (DASH) ===");
    println!("Baseline: VwapReversion (COINCLAW v10)");
    println!("Challengers: ou_mean_rev (3 deviation thresholds)\n");

    let path = format!("{}/DASH_USDT_15m_5months.csv", DATA_DIR);
    let data = load_csv(&path).expect("Failed to load DASH data");
    println!("Loaded {} — {} candles", data.name, data.close.len());

    let ind = indicators::compute_all(&data.open, &data.high, &data.low, &data.close, &data.volume);
    let vwap = compute_rolling_vwap(&data.high, &data.low, &data.close, &data.volume, 96);
    let month_size = data.close.len() / 5;

    let ou_15 = StratConfig { name: "ou_mean_rev", p1: 10.0, p2: 1.5, p3: 0.0, z_filter: 0.0 };
    let ou_20 = StratConfig { name: "ou_mean_rev", p1: 10.0, p2: 2.0, p3: 0.0, z_filter: 0.0 };
    let ou_25 = StratConfig { name: "ou_mean_rev", p1: 10.0, p2: 2.5, p3: 0.0, z_filter: 0.0 };
    let ctrl = StratConfig { name: "ctrl_mean_rev", p1: 1.5, p2: 1.2, p3: 0.0, z_filter: 0.0 };

    struct TestCombo<'a> { name: &'a str, cfg: Option<&'a StratConfig> }
    let combos = vec![
        TestCombo { name: "vwap_rev", cfg: None },
        TestCombo { name: "ctrl_mean_rev", cfg: Some(&ctrl) },
        TestCombo { name: "ou_dev1.5", cfg: Some(&ou_15) },
        TestCombo { name: "ou_dev2.0", cfg: Some(&ou_20) },
        TestCombo { name: "ou_dev2.5", cfg: Some(&ou_25) },
    ];

    let mut all_results: Vec<StratResult> = Vec::new();
    for combo in &combos {
        for exit in EXIT_MODES {
            let mut result = run_backtest_detailed(&data, &ind, &vwap, combo.name, combo.cfg, exit, month_size);
            // Override strategy name for display
            if combo.name != "vwap_rev" && combo.name != "ctrl_mean_rev" {
                result.strategy = combo.name.to_string();
            }
            all_results.push(result);
        }
    }

    // Print comparison
    println!("\n{}", "=".repeat(115));
    println!("=== COMPARISON TABLE ===\n");
    println!("{:<18} {:<12} {:>5} {:>5} {:>6} {:>6} {:>7} {:>7} {:>7} {:>5}",
        "Strategy", "Exit", "Trd", "Win", "WR%", "PF", "P&L%", "AvgW%", "AvgL%", "Hold");
    println!("{}", "-".repeat(100));
    for r in &all_results {
        println!("{:<18} {:<12} {:>5} {:>5} {:>5.1}% {:>6.2} {:>+6.1}% {:>+6.2}% {:>+6.2}% {:>5.1}",
            r.strategy, r.exit_mode, r.trades, r.wins, r.win_rate, r.profit_factor,
            r.pnl_pct, r.avg_win_pct, r.avg_loss_pct, r.avg_hold);
    }

    // Exit reasons
    println!("\n{}", "=".repeat(80));
    println!("=== EXIT REASON DISTRIBUTION ===\n");
    println!("{:<18} {:<12} {:>5} {:>6} {:>6} {:>8} {:>5}",
        "Strategy", "Exit", "SL", "SMA20", "Z_REV", "MAXHOLD", "EOD");
    println!("{}", "-".repeat(65));
    for r in &all_results {
        let eod = r.trades - r.sl_exits - r.sma20_exits - r.z_rev_exits - r.max_hold_exits;
        println!("{:<18} {:<12} {:>5} {:>6} {:>6} {:>8} {:>5}",
            r.strategy, r.exit_mode, r.sl_exits, r.sma20_exits, r.z_rev_exits, r.max_hold_exits, eod);
    }

    // Per-month (signal_only)
    println!("\n{}", "=".repeat(90));
    println!("=== PER-MONTH WIN RATE (signal_only) ===\n");
    println!("{:<18} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>6}",
        "Strategy", "M1", "M2", "M3", "M4", "M5", "Avg");
    println!("{}", "-".repeat(70));
    for r in all_results.iter().filter(|r| r.exit_mode == "signal_only") {
        print!("{:<18}", r.strategy);
        for m in 0..5 {
            if r.month_trades[m] > 0 {
                print!(" {:>5.1}%/{}", r.month_wr[m], r.month_trades[m]);
            } else {
                print!("    -/0 ");
            }
        }
        println!("  {:>5.1}%", r.win_rate);
    }

    // Head-to-head
    println!("\n{}", "=".repeat(90));
    println!("=== HEAD-TO-HEAD SUMMARY (signal_only) ===\n");
    let baseline = all_results.iter().find(|r| r.strategy == "vwap_rev" && r.exit_mode == "signal_only");
    if let Some(base) = baseline {
        println!("BASELINE: vwap_rev — WR={:.1}% PF={:.2} P&L={:+.1}% Trades={}\n",
            base.win_rate, base.profit_factor, base.pnl_pct, base.trades);
        for c in all_results.iter().filter(|r| r.strategy != "vwap_rev" && r.exit_mode == "signal_only") {
            let wr_diff = c.win_rate - base.win_rate;
            let pf_diff = c.profit_factor - base.profit_factor;
            let pnl_diff = c.pnl_pct - base.pnl_pct;
            let verdict = if c.win_rate >= base.win_rate && c.profit_factor >= base.profit_factor {
                "BETTER"
            } else if c.win_rate >= base.win_rate - 2.0 && c.profit_factor >= base.profit_factor - 0.1 {
                "COMPARABLE"
            } else { "WORSE" };
            println!("  vs {:<16} WR={:.1}% ({:+.1}) PF={:.2} ({:+.2}) P&L={:+.1}% ({:+.1}) T={} → {}",
                c.strategy, c.win_rate, wr_diff, c.profit_factor, pf_diff, c.pnl_pct, pnl_diff, c.trades, verdict);
        }
    }

    // Best overall
    println!("\n{}", "=".repeat(90));
    println!("=== BEST CONFIG OVERALL (WR×PF score) ===\n");
    let mut by_score: Vec<&StratResult> = all_results.iter().filter(|r| r.trades >= 10).collect();
    by_score.sort_by(|a, b| {
        let sa = a.win_rate * a.profit_factor;
        let sb = b.win_rate * b.profit_factor;
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, r) in by_score.iter().take(8).enumerate() {
        let score = r.win_rate * r.profit_factor;
        println!("  #{} {:<18} {:<12} WR={:.1}% PF={:.2} P&L={:+.1}% T={} Score={:.0}",
            i + 1, r.strategy, r.exit_mode, r.win_rate, r.profit_factor, r.pnl_pct, r.trades, score);
    }

    // Recommendation
    println!("\n{}", "=".repeat(90));
    if let Some(best) = by_score.first() {
        if let Some(base) = baseline {
            if best.strategy != "vwap_rev" && best.win_rate > base.win_rate && best.profit_factor > base.profit_factor {
                println!("RECOMMENDATION: Replace DASH long_strat with '{}' ({}) — beats vwap_rev by +{:.1}% WR, +{:.2} PF",
                    best.strategy, best.exit_mode, best.win_rate - base.win_rate, best.profit_factor - base.profit_factor);
            } else {
                println!("RECOMMENDATION: Keep current vwap_rev assignment — no challenger clearly beats it");
            }
        }
    }
    println!();

    let output = ComparisonOutput { run: "RUN11c.3".to_string(), coin: data.name, results: all_results };
    if let Ok(json) = serde_json::to_string_pretty(&output) {
        fs::write(RESULTS_FILE, json).ok();
        println!("Results saved to {}", RESULTS_FILE);
    }
    println!("Done.");
}
