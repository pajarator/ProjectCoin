//! RUN10.3 — Scalp TP/SL Grid Search WITH FEES
//!
//! Tests scalp strategies across a TP/SL grid with realistic fee deduction.
//! Also tests with/without F6 filter (dir_roc_3 + avg_body_3).
//!
//! Grid:
//!   TP: 0.15%, 0.20%, 0.25%, 0.30%, 0.40%, 0.50%, 0.60%, 0.80%
//!   SL: 0.10%, 0.15%, 0.20%, 0.25%, 0.30%
//!   Fee: maker (0.02%/side), taker (0.05%/side)
//!   Filter: none, F6 (dir_roc_3 + avg_body_3)
//!   Strategies: vol_spike_rev only, stoch_cross only, both
//!
//! Uses full 5-month dataset, rayon parallelism.

use chrono::NaiveDateTime;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

// ── Constants ──────────────────────────────────────────────────────────

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run10_3_results.json";

const COINS: &[&str] = &[
    "DASH", "UNI", "NEAR", "ADA", "LTC", "SHIB", "LINK", "ETH",
    "DOT", "XRP", "ATOM", "SOL", "DOGE", "XLM", "AVAX", "ALGO", "BNB", "BTC",
];

// Scalp entry params (from RUN9 best universal)
const VOL_SPIKE_MULT: f64 = 3.5;
const RSI_EXTREME: f64 = 20.0;
const STOCH_EXTREME: f64 = 5.0;
const BB_SQUEEZE_FACTOR: f64 = 0.4;

const LEVERAGE: f64 = 5.0;
const RISK_PCT: f64 = 0.05; // 5% of balance per scalp trade
const INITIAL_CAPITAL: f64 = 100.0;
const MAX_HOLD_BARS: usize = 60;

// TP/SL grid
const TP_GRID: &[f64] = &[0.0015, 0.0020, 0.0025, 0.0030, 0.0040, 0.0050, 0.0060, 0.0080];
const SL_GRID: &[f64] = &[0.0010, 0.0015, 0.0020, 0.0025, 0.0030];

// Fee scenarios (per-side)
const FEE_MAKER: f64 = 0.0002;  // 0.02%
const FEE_TAKER: f64 = 0.0005;  // 0.05%

// ── Data Structures ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Candle {
    ts: NaiveDateTime,
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
}

#[derive(Debug, Clone, Default)]
struct Ind1m {
    rsi: f64,
    stoch_k: f64,
    stoch_d: f64,
    stoch_k_prev: f64,
    stoch_d_prev: f64,
    bb_upper: f64,
    bb_lower: f64,
    bb_width: f64,
    bb_width_avg: f64,
    vol_ma: f64,
    vol_ratio: f64,
    // Filter indicators
    roc_3: f64,
    avg_body_3: f64,
    spread_pct: f64,
    ema5: f64,
    ema12: f64,
    ema_spread: f64,
    valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir { Long, Short }

#[derive(Debug, Clone)]
struct Trade {
    direction: Dir,
    strategy: u8, // 0=vol_spike, 1=stoch_cross, 2=bb_squeeze
    pnl_gross: f64,   // price move % (without fees)
    exit: ExitType,
    // Filter snapshot
    dir_roc_3: f64,
    avg_body_3: f64,
    spread_pct: f64,
}

#[derive(Debug, Clone, Copy)]
enum ExitType { TP, SL, Timeout }

#[derive(Debug, Clone, Serialize)]
struct ComboResult {
    tp_pct: f64,
    sl_pct: f64,
    fee_label: String,
    fee_per_side: f64,
    filter: String,
    strategies: String,
    total_trades: usize,
    wins: usize,
    losses: usize,
    win_rate: f64,
    gross_pnl: f64,       // % of initial capital, no fees
    net_pnl: f64,         // % of initial capital, with fees
    fee_drag: f64,         // total fees paid as % of initial capital
    avg_net_per_trade: f64, // net pnl per trade as % of balance
    profit_factor: f64,
    max_drawdown: f64,
    monthly_est: f64,      // estimated monthly return %
}

#[derive(Debug, Serialize)]
struct FinalResults {
    combos: Vec<ComboResult>,
    top_20_by_net_pnl: Vec<ComboResult>,
    top_10_maker: Vec<ComboResult>,
    top_10_taker: Vec<ComboResult>,
    breakeven_analysis: Vec<BreakevenRow>,
}

#[derive(Debug, Serialize)]
struct BreakevenRow {
    tp_pct: f64,
    sl_pct: f64,
    be_wr_maker: f64,
    be_wr_taker: f64,
    actual_wr_no_filter: f64,
    actual_wr_f6: f64,
    profitable_maker_nofilt: bool,
    profitable_maker_f6: bool,
    profitable_taker_nofilt: bool,
    profitable_taker_f6: bool,
}

// ── CSV Loading ────────────────────────────────────────────────────────

fn load_candles(coin: &str, tf: &str) -> Vec<Candle> {
    let path = format!("{}/{}_USDT_{}_5months.csv", DATA_DIR, coin, tf);
    let mut candles = Vec::new();
    let Ok(mut rdr) = csv::ReaderBuilder::new().has_headers(true).from_path(&path) else {
        return candles;
    };
    for result in rdr.records().flatten() {
        let ts_str = result.get(0).unwrap_or("");
        let ts = match NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
            Ok(t) => t,
            Err(_) => continue,
        };
        let p = |i: usize| -> f64 { result.get(i).unwrap_or("0").parse().unwrap_or(0.0) };
        let c = Candle { ts, o: p(1), h: p(2), l: p(3), c: p(4), v: p(5) };
        if c.c > 0.0 { candles.push(c); }
    }
    candles
}

// ── Indicators ─────────────────────────────────────────────────────────

fn rolling_mean(vals: &[f64], period: usize, idx: usize) -> f64 {
    if idx + 1 < period { return f64::NAN; }
    vals[idx + 1 - period..=idx].iter().sum::<f64>() / period as f64
}

fn ema_vec(vals: &[f64], period: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; vals.len()];
    let mult = 2.0 / (period as f64 + 1.0);
    for i in 0..vals.len() {
        if i < period - 1 {
            continue;
        } else if i == period - 1 {
            out[i] = vals[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
        } else if out[i - 1].is_finite() {
            out[i] = (vals[i] - out[i - 1]) * mult + out[i - 1];
        }
    }
    out
}

fn compute_rsi(closes: &[f64], period: usize) -> Vec<f64> {
    let n = closes.len();
    let mut rsi = vec![f64::NAN; n];
    if n < period + 1 { return rsi; }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 { avg_gain += d; } else { avg_loss -= d; }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    rsi[period] = if avg_loss > 0.0 { 100.0 - 100.0 / (1.0 + avg_gain / avg_loss) } else { 100.0 };
    for i in (period + 1)..n {
        let d = closes[i] - closes[i - 1];
        let (g, l) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
        rsi[i] = if avg_loss > 0.0 { 100.0 - 100.0 / (1.0 + avg_gain / avg_loss) } else { 100.0 };
    }
    rsi
}

fn compute_1m_indicators(candles: &[Candle]) -> Vec<Ind1m> {
    let n = candles.len();
    let mut inds = vec![Ind1m::default(); n];
    if n < 30 { return inds; }

    let closes: Vec<f64> = candles.iter().map(|c| c.c).collect();
    let highs: Vec<f64> = candles.iter().map(|c| c.h).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.l).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.v).collect();

    let rsi = compute_rsi(&closes, 14);
    let ema5 = ema_vec(&closes, 5);
    let ema12 = ema_vec(&closes, 12);

    // Rolling arrays
    let mut vol_ma20 = vec![f64::NAN; n];
    let mut sma20 = vec![f64::NAN; n];
    let mut bb_std = vec![f64::NAN; n];
    for i in 0..n {
        vol_ma20[i] = rolling_mean(&volumes, 20, i);
        sma20[i] = rolling_mean(&closes, 20, i);
        if i + 1 >= 20 {
            let mean = sma20[i];
            let var = closes[i + 1 - 20..=i].iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 20.0;
            bb_std[i] = var.sqrt();
        }
    }

    // Stochastic
    let mut stoch_k = vec![f64::NAN; n];
    for i in 13..n {
        let lo = lows[i - 13..=i].iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = highs[i - 13..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = hi - lo;
        if range > 0.0 { stoch_k[i] = (closes[i] - lo) / range * 100.0; }
    }
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if stoch_k[i].is_finite() && stoch_k[i - 1].is_finite() && stoch_k[i - 2].is_finite() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i - 1] + stoch_k[i - 2]) / 3.0;
        }
    }

    // BB width avg
    let mut bb_width_arr = vec![f64::NAN; n];
    for i in 0..n {
        if sma20[i].is_finite() && bb_std[i].is_finite() {
            bb_width_arr[i] = 4.0 * bb_std[i]; // upper - lower = 2*2*std
        }
    }
    let mut bb_width_avg = vec![f64::NAN; n];
    for i in 19..n {
        let mut sum = 0.0;
        let mut cnt = 0;
        for j in (i - 19)..=i {
            if bb_width_arr[j].is_finite() { sum += bb_width_arr[j]; cnt += 1; }
        }
        if cnt >= 10 { bb_width_avg[i] = sum / cnt as f64; }
    }

    // Body % for avg_body_3
    let body_pct: Vec<f64> = candles.iter().map(|c| (c.c - c.o).abs() / c.c * 100.0).collect();

    for i in 0..n {
        let c = &candles[i];
        let ind = &mut inds[i];
        ind.rsi = rsi[i];
        ind.stoch_k = stoch_k[i];
        ind.stoch_d = stoch_d[i];
        ind.stoch_k_prev = if i >= 1 { stoch_k[i - 1] } else { f64::NAN };
        ind.stoch_d_prev = if i >= 1 { stoch_d[i - 1] } else { f64::NAN };

        let std_v = bb_std[i];
        ind.bb_upper = if sma20[i].is_finite() && std_v.is_finite() { sma20[i] + 2.0 * std_v } else { f64::NAN };
        ind.bb_lower = if sma20[i].is_finite() && std_v.is_finite() { sma20[i] - 2.0 * std_v } else { f64::NAN };
        ind.bb_width = bb_width_arr[i];
        ind.bb_width_avg = bb_width_avg[i];

        ind.vol_ma = vol_ma20[i];
        ind.vol_ratio = if vol_ma20[i].is_finite() && vol_ma20[i] > 0.0 { c.v / vol_ma20[i] } else { f64::NAN };

        ind.roc_3 = if i >= 3 && closes[i - 3] > 0.0 {
            (c.c - closes[i - 3]) / closes[i - 3] * 100.0
        } else { f64::NAN };

        ind.avg_body_3 = if i >= 2 {
            (body_pct[i] + body_pct[i - 1] + body_pct[i - 2]) / 3.0
        } else { f64::NAN };

        let full_range = c.h - c.l;
        ind.spread_pct = if c.c > 0.0 { full_range / c.c * 100.0 } else { 0.0 };

        ind.ema5 = ema5[i];
        ind.ema12 = ema12[i];
        ind.ema_spread = if ema5[i].is_finite() && ema12[i].is_finite() && c.c > 0.0 {
            (ema5[i] - ema12[i]) / c.c * 100.0
        } else { f64::NAN };

        ind.valid = ind.rsi.is_finite() && ind.vol_ma.is_finite() && ind.vol_ma > 0.0;
    }
    inds
}

// ── Scalp Entry ────────────────────────────────────────────────────────

/// Returns (direction, strategy_id) or None.
/// strategy: 0=vol_spike_rev, 1=stoch_cross, 2=bb_squeeze
fn scalp_entry(ind: &Ind1m, price: f64) -> Option<(Dir, u8)> {
    if !ind.valid { return None; }
    let vr = ind.vol_ratio;

    // vol_spike_rev
    if vr > VOL_SPIKE_MULT {
        if ind.rsi < RSI_EXTREME { return Some((Dir::Long, 0)); }
        if ind.rsi > 100.0 - RSI_EXTREME { return Some((Dir::Short, 0)); }
    }

    // stoch_cross
    if ind.stoch_k.is_finite() && ind.stoch_d.is_finite()
        && ind.stoch_k_prev.is_finite() && ind.stoch_d_prev.is_finite()
    {
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < STOCH_EXTREME && ind.stoch_d < STOCH_EXTREME
        {
            return Some((Dir::Long, 1));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > 100.0 - STOCH_EXTREME && ind.stoch_d > 100.0 - STOCH_EXTREME
        {
            return Some((Dir::Short, 1));
        }
    }

    // bb_squeeze
    if ind.bb_width_avg.is_finite() && ind.bb_width_avg > 0.0 && ind.bb_upper.is_finite() {
        let squeeze = ind.bb_width < ind.bb_width_avg * BB_SQUEEZE_FACTOR;
        if squeeze && vr > 2.0 {
            if price > ind.bb_upper { return Some((Dir::Long, 2)); }
            if price < ind.bb_lower { return Some((Dir::Short, 2)); }
        }
    }

    None
}

// ── Collect raw trades (TP/SL agnostic — record gross price move) ──────

/// For each scalp entry, simulate forward up to MAX_HOLD_BARS and record
/// the maximum favorable excursion (MFE) and maximum adverse excursion (MAE)
/// per bar. We store the full price path so we can evaluate any TP/SL combo
/// without re-scanning candles.
#[derive(Debug, Clone)]
struct RawTrade {
    direction: Dir,
    strategy: u8,
    entry_price: f64,
    // Price moves as fraction of entry price, signed for direction
    // pnl_path[j] = directional PnL at bar j+1 after entry
    pnl_path: Vec<f64>,
    // Filter snapshot
    dir_roc_3: f64,
    avg_body_3: f64,
    spread_pct: f64,
}

fn collect_raw_trades(candles: &[Candle], inds: &[Ind1m]) -> Vec<RawTrade> {
    let n = candles.len();
    let mut trades = Vec::new();
    let mut i = 0;

    while i < n {
        let ind = &inds[i];
        let price = candles[i].c;
        let Some((dir, strat)) = scalp_entry(ind, price) else {
            i += 1;
            continue;
        };

        let entry_price = price;
        let sign = if dir == Dir::Long { 1.0 } else { -1.0 };

        // Direction-adjusted roc_3 for filter
        let dir_roc_3 = if ind.roc_3.is_finite() { ind.roc_3 * sign } else { f64::NAN };

        // Record price path
        let end = (i + MAX_HOLD_BARS + 1).min(n);
        let mut pnl_path = Vec::with_capacity(end - i - 1);
        for j in (i + 1)..end {
            let p = candles[j].c;
            let pnl = if dir == Dir::Long {
                (p - entry_price) / entry_price
            } else {
                (entry_price - p) / entry_price
            };
            pnl_path.push(pnl);
        }

        // Find how many bars the most aggressive exit would use (SL=0.10%)
        // to determine cooldown — use smallest SL in grid
        let min_sl = 0.0010;
        let max_tp = 0.0080;
        let mut bars_used = pnl_path.len();
        for (j, &pnl) in pnl_path.iter().enumerate() {
            if pnl >= max_tp || pnl <= -min_sl {
                bars_used = j + 1;
                break;
            }
        }

        trades.push(RawTrade {
            direction: dir,
            strategy: strat,
            entry_price,
            pnl_path,
            dir_roc_3,
            avg_body_3: ind.avg_body_3,
            spread_pct: ind.spread_pct,
        });

        i += bars_used.max(1) + 1;
    }
    trades
}

// ── Evaluate a TP/SL combo on raw trades ───────────────────────────────

fn evaluate_combo(
    trades: &[&RawTrade],
    tp: f64,
    sl: f64,
    fee_per_side: f64,
) -> (usize, usize, f64, f64, f64, f64) {
    // Returns: (wins, losses, gross_pnl_balance%, net_pnl_balance%, total_fee_drag%, max_dd%)
    let rt_fee = fee_per_side * 2.0; // round-trip fee as fraction of position
    let mut balance = INITIAL_CAPITAL;
    let mut peak = INITIAL_CAPITAL;
    let mut max_dd = 0.0_f64;
    let mut wins = 0_usize;
    let mut losses = 0_usize;
    let mut gross_total = 0.0_f64;
    let mut fee_total = 0.0_f64;

    for trade in trades {
        let position_value = balance * RISK_PCT * LEVERAGE;
        let fee_cost = position_value * rt_fee;

        // Walk the price path
        let mut trade_pnl_frac = 0.0_f64; // fraction of entry price
        let mut hit = false;
        for &pnl in &trade.pnl_path {
            if pnl >= tp {
                trade_pnl_frac = tp;
                hit = true;
                break;
            } else if pnl <= -sl {
                trade_pnl_frac = -sl;
                hit = true;
                break;
            }
        }
        if !hit {
            // Timeout: use last price
            trade_pnl_frac = trade.pnl_path.last().copied().unwrap_or(0.0);
        }

        let gross_dollar = position_value * trade_pnl_frac;
        let net_dollar = gross_dollar - fee_cost;

        gross_total += gross_dollar;
        fee_total += fee_cost;
        balance += net_dollar;

        if net_dollar > 0.0 { wins += 1; } else { losses += 1; }

        if balance > peak { peak = balance; }
        let dd = (peak - balance) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    let gross_pnl = gross_total / INITIAL_CAPITAL * 100.0;
    let net_pnl = (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100.0;
    let fee_drag = fee_total / INITIAL_CAPITAL * 100.0;

    (wins, losses, gross_pnl, net_pnl, fee_drag, max_dd)
}

// ── F6 Filter ──────────────────────────────────────────────────────────

/// F6: dir_roc_3 < threshold AND avg_body_3 > threshold
/// Use the RUN10.1 discovered values as universal thresholds.
/// dir_roc_3 < -0.195, avg_body_3 > 0.072
fn passes_f6(trade: &RawTrade) -> bool {
    trade.dir_roc_3.is_finite() && trade.dir_roc_3 < -0.195
        && trade.avg_body_3.is_finite() && trade.avg_body_3 > 0.072
}

// ── Main ───────────────────────────────────────────────────────────────

fn main() {
    println!("{}", "=".repeat(90));
    println!("RUN10.3 — Scalp TP/SL Grid Search WITH FEES");
    println!("{}", "=".repeat(90));
    println!("TP grid: {:?}", TP_GRID.iter().map(|x| format!("{:.2}%", x * 100.0)).collect::<Vec<_>>());
    println!("SL grid: {:?}", SL_GRID.iter().map(|x| format!("{:.2}%", x * 100.0)).collect::<Vec<_>>());
    println!("Fees: maker=0.02%/side, taker=0.05%/side");
    println!("Strategies: vol_spike_rev(0), stoch_cross(1), bb_squeeze(2)");
    println!("Filter: none vs F6(dir_roc_3+avg_body_3)");
    println!("Coins: {}", COINS.len());
    println!("{}", "=".repeat(90));

    // Load 1m data and compute indicators
    println!("\nLoading and computing indicators...");
    let coin_trades: Vec<(String, Vec<RawTrade>)> = COINS
        .par_iter()
        .filter_map(|&coin| {
            let candles = load_candles(coin, "1m");
            if candles.len() < 100 { return None; }
            let inds = compute_1m_indicators(&candles);
            let trades = collect_raw_trades(&candles, &inds);
            println!("  {}: {} candles, {} raw trades", coin, candles.len(), trades.len());
            Some((coin.to_string(), trades))
        })
        .collect();

    // Merge all trades
    let all_trades: Vec<&RawTrade> = coin_trades.iter().flat_map(|(_, t)| t.iter()).collect();
    println!("\nTotal raw trades: {}", all_trades.len());

    // Strategy breakdown
    let strat_names = ["vol_spike_rev", "stoch_cross", "bb_squeeze"];
    for (id, name) in strat_names.iter().enumerate() {
        let cnt = all_trades.iter().filter(|t| t.strategy == id as u8).count();
        println!("  {}: {} trades", name, cnt);
    }

    // Pre-filter sets
    let trades_no_filter: Vec<&RawTrade> = all_trades.clone();
    let trades_f6: Vec<&RawTrade> = all_trades.iter().filter(|t| passes_f6(t)).copied().collect();
    let trades_no_bb: Vec<&RawTrade> = all_trades.iter().filter(|t| t.strategy != 2).copied().collect();
    let trades_no_bb_f6: Vec<&RawTrade> = trades_no_bb.iter().filter(|t| passes_f6(t)).copied().collect();
    let trades_vol_only: Vec<&RawTrade> = all_trades.iter().filter(|t| t.strategy == 0).copied().collect();
    let trades_vol_only_f6: Vec<&RawTrade> = trades_vol_only.iter().filter(|t| passes_f6(t)).copied().collect();

    println!("\nTrade sets:");
    println!("  All strategies, no filter: {}", trades_no_filter.len());
    println!("  All strategies, F6 filter: {}", trades_f6.len());
    println!("  No bb_squeeze, no filter:  {}", trades_no_bb.len());
    println!("  No bb_squeeze, F6 filter:  {}", trades_no_bb_f6.len());
    println!("  vol_spike only, no filter: {}", trades_vol_only.len());
    println!("  vol_spike only, F6 filter: {}", trades_vol_only_f6.len());

    // Build combo grid
    struct TradeSet<'a> {
        name: &'static str,
        trades: Vec<&'a RawTrade>,
    }

    let trade_sets: Vec<TradeSet> = vec![
        TradeSet { name: "all_nofilt", trades: trades_no_filter },
        TradeSet { name: "all_F6", trades: trades_f6 },
        TradeSet { name: "nobb_nofilt", trades: trades_no_bb },
        TradeSet { name: "nobb_F6", trades: trades_no_bb_f6 },
        TradeSet { name: "volspike_nofilt", trades: trades_vol_only },
        TradeSet { name: "volspike_F6", trades: trades_vol_only_f6 },
    ];

    let fees = [("maker", FEE_MAKER), ("taker", FEE_TAKER)];

    // Run grid
    println!("\nRunning {} TP × {} SL × {} fees × {} sets = {} combos...",
             TP_GRID.len(), SL_GRID.len(), fees.len(), trade_sets.len(),
             TP_GRID.len() * SL_GRID.len() * fees.len() * trade_sets.len());

    let mut all_results: Vec<ComboResult> = Vec::new();

    for &tp in TP_GRID {
        for &sl in SL_GRID {
            for &(fee_label, fee_ps) in &fees {
                for ts in &trade_sets {
                    if ts.trades.is_empty() { continue; }

                    // Re-evaluate which trades are wins/losses under THIS tp/sl
                    // (raw trade paths allow us to do this)
                    let (wins, losses, gross_pnl, net_pnl, fee_drag, max_dd) =
                        evaluate_combo(&ts.trades, tp, sl, fee_ps);

                    let total = wins + losses;
                    let wr = if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 };

                    // Profit factor: sum of positive trade pnls / sum of negative
                    // We approximate from net results
                    let avg_net = if total > 0 { net_pnl / total as f64 } else { 0.0 };
                    let months = 5.0;
                    let monthly = net_pnl / months;

                    // Approximate PF from win/loss and average sizes
                    let pf = if losses > 0 && wins > 0 {
                        let avg_win_size = (tp - fee_ps * 2.0) * LEVERAGE;
                        let avg_loss_size = (sl + fee_ps * 2.0) * LEVERAGE;
                        (wins as f64 * avg_win_size) / (losses as f64 * avg_loss_size)
                    } else if wins > 0 {
                        999.0
                    } else {
                        0.0
                    };

                    all_results.push(ComboResult {
                        tp_pct: tp * 100.0,
                        sl_pct: sl * 100.0,
                        fee_label: fee_label.to_string(),
                        fee_per_side: fee_ps * 100.0,
                        filter: ts.name.to_string(),
                        strategies: ts.name.split('_').next().unwrap_or("all").to_string(),
                        total_trades: total,
                        wins,
                        losses,
                        win_rate: (wr * 10.0).round() / 10.0,
                        gross_pnl: (gross_pnl * 100.0).round() / 100.0,
                        net_pnl: (net_pnl * 100.0).round() / 100.0,
                        fee_drag: (fee_drag * 100.0).round() / 100.0,
                        avg_net_per_trade: (avg_net * 10000.0).round() / 10000.0,
                        profit_factor: (pf * 100.0).round() / 100.0,
                        max_drawdown: (max_dd * 10.0).round() / 10.0,
                        monthly_est: (monthly * 100.0).round() / 100.0,
                    });
                }
            }
        }
    }

    println!("Done. {} results.\n", all_results.len());

    // === PRINT TOP RESULTS ===

    // Sort by net_pnl descending
    let mut sorted = all_results.clone();
    sorted.sort_by(|a, b| b.net_pnl.partial_cmp(&a.net_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("{}", "=".repeat(120));
    println!("TOP 25 COMBOS BY NET P&L (across all fees/filters/strategies)");
    println!("{}", "=".repeat(120));
    print_header();
    for r in sorted.iter().take(25) {
        print_row(r);
    }

    // Top 10 taker only
    let mut taker_sorted: Vec<&ComboResult> = all_results.iter().filter(|r| r.fee_label == "taker").collect();
    taker_sorted.sort_by(|a, b| b.net_pnl.partial_cmp(&a.net_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n{}", "=".repeat(120));
    println!("TOP 15 COMBOS — TAKER FEES ONLY (realistic for market orders)");
    println!("{}", "=".repeat(120));
    print_header();
    for r in taker_sorted.iter().take(15) {
        print_row(r);
    }

    // Top 10 maker only
    let mut maker_sorted: Vec<&ComboResult> = all_results.iter().filter(|r| r.fee_label == "maker").collect();
    maker_sorted.sort_by(|a, b| b.net_pnl.partial_cmp(&a.net_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n{}", "=".repeat(120));
    println!("TOP 15 COMBOS — MAKER FEES ONLY (limit orders)");
    println!("{}", "=".repeat(120));
    print_header();
    for r in maker_sorted.iter().take(15) {
        print_row(r);
    }

    // === BREAKEVEN ANALYSIS ===
    println!("\n{}", "=".repeat(100));
    println!("BREAKEVEN ANALYSIS — Actual WR at each TP/SL vs required WR");
    println!("{}", "=".repeat(100));

    let mut be_rows = Vec::new();
    println!("{:<8} {:<8} {:>8} {:>8} {:>10} {:>10}  {:>6} {:>6} {:>6} {:>6}",
             "TP%", "SL%", "BE_mkr", "BE_tkr", "WR_nofilt", "WR_F6",
             "mk_nf", "mk_f6", "tk_nf", "tk_f6");
    println!("{}", "-".repeat(100));

    for &tp in TP_GRID {
        for &sl in SL_GRID {
            // Compute actual WR at this TP/SL for no-filter and F6 (no bb_squeeze)
            let wr_nf = compute_wr_at_tpsl(&trade_sets[2].trades, tp, sl); // nobb_nofilt
            let wr_f6 = compute_wr_at_tpsl(&trade_sets[3].trades, tp, sl); // nobb_F6

            let be_maker = breakeven_wr(tp, sl, FEE_MAKER);
            let be_taker = breakeven_wr(tp, sl, FEE_TAKER);

            let mk_nf = wr_nf > be_maker;
            let mk_f6 = wr_f6 > be_maker;
            let tk_nf = wr_nf > be_taker;
            let tk_f6 = wr_f6 > be_taker;

            let mark = |b: bool| if b { "  ✓" } else { "  ✗" };

            println!("{:<8.2} {:<8.2} {:>7.1}% {:>7.1}% {:>9.1}% {:>9.1}% {}{}{}{}",
                     tp * 100.0, sl * 100.0, be_maker, be_taker, wr_nf, wr_f6,
                     mark(mk_nf), mark(mk_f6), mark(tk_nf), mark(tk_f6));

            be_rows.push(BreakevenRow {
                tp_pct: tp * 100.0,
                sl_pct: sl * 100.0,
                be_wr_maker: (be_maker * 10.0).round() / 10.0,
                be_wr_taker: (be_taker * 10.0).round() / 10.0,
                actual_wr_no_filter: (wr_nf * 10.0).round() / 10.0,
                actual_wr_f6: (wr_f6 * 10.0).round() / 10.0,
                profitable_maker_nofilt: mk_nf,
                profitable_maker_f6: mk_f6,
                profitable_taker_nofilt: tk_nf,
                profitable_taker_f6: tk_f6,
            });
        }
    }

    // === SAVE ===
    let top20: Vec<ComboResult> = sorted.iter().take(20).cloned().collect();
    let top10_maker: Vec<ComboResult> = maker_sorted.iter().take(10).map(|r| (*r).clone()).collect();
    let top10_taker: Vec<ComboResult> = taker_sorted.iter().take(10).map(|r| (*r).clone()).collect();

    let results = FinalResults {
        combos: all_results,
        top_20_by_net_pnl: top20,
        top_10_maker: top10_maker,
        top_10_taker: top10_taker,
        breakeven_analysis: be_rows,
    };

    let json = serde_json::to_string_pretty(&results).unwrap();
    fs::write(RESULTS_FILE, &json).unwrap();
    println!("\nResults saved to {}", RESULTS_FILE);
    println!("Done.");
}

fn print_header() {
    println!("{:<7} {:<7} {:<7} {:<16} {:>7} {:>6} {:>9} {:>9} {:>9} {:>5} {:>8}",
             "TP%", "SL%", "Fee", "Filter", "Trades", "WR%", "Gross%", "Net%", "FeeDrag", "PF", "Mo%");
    println!("{}", "-".repeat(120));
}

fn print_row(r: &ComboResult) {
    println!("{:<7.2} {:<7.2} {:<7} {:<16} {:>7} {:>5.1}% {:>+8.1}% {:>+8.1}% {:>8.1}% {:>5.2} {:>+7.1}%",
             r.tp_pct, r.sl_pct, r.fee_label, r.filter,
             r.total_trades, r.win_rate, r.gross_pnl, r.net_pnl, r.fee_drag,
             r.profit_factor, r.monthly_est);
}

fn compute_wr_at_tpsl(trades: &[&RawTrade], tp: f64, sl: f64) -> f64 {
    let mut wins = 0;
    let mut total = 0;
    for t in trades {
        total += 1;
        let mut hit = false;
        for &pnl in &t.pnl_path {
            if pnl >= tp { wins += 1; hit = true; break; }
            if pnl <= -sl { hit = true; break; }
        }
        if !hit {
            if let Some(&last) = t.pnl_path.last() {
                if last > 0.0 { wins += 1; }
            }
        }
    }
    if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 }
}

fn breakeven_wr(tp: f64, sl: f64, fee_per_side: f64) -> f64 {
    let rt = fee_per_side * 2.0;
    let win_net = tp - rt;
    let loss_net = sl + rt;
    if win_net + loss_net > 0.0 {
        loss_net / (win_net + loss_net) * 100.0
    } else {
        100.0
    }
}
