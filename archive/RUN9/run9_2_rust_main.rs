use csv::ReaderBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const DATA_CACHE_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run9_2_results.json";
const CHECKPOINT_FILE: &str = "/home/scamarena/ProjectCoin/run9_2_rust_checkpoint.json";
const R91_FILE: &str = "/home/scamarena/ProjectCoin/run9_1_results.json";

const COINS: &[&str] = &[
    "DASH", "UNI", "NEAR", "ADA", "LTC", "SHIB", "LINK", "ETH", "DOT", "XRP",
    "ATOM", "SOL", "DOGE", "XLM", "AVAX", "ALGO", "BNB", "BTC",
];

const LEVERAGE: f64 = 5.0;
const INITIAL_CAPITAL: f64 = 100.0;
const REGIME_RISK: f64 = 0.10;
const SCALP_RISK: f64 = 0.05;
const REGIME_SL: f64 = 0.003;
const MIN_HOLD: usize = 2;
const BREADTH_LONG_MAX: f64 = 0.20;
const BREADTH_SHORT_MIN: f64 = 0.50;
const ISO_SHORT_BREADTH_MAX: f64 = 0.20;

const SCALP_SLS: &[f64] = &[0.0010, 0.0015, 0.0020, 0.0025];
const SCALP_TPS: &[f64] = &[0.0020, 0.0030, 0.0040, 0.0050];
const VOL_SPIKE_MULTS: &[f64] = &[2.5, 3.0, 3.5];
const RSI_EXTREMES: &[f64] = &[15.0, 20.0, 25.0];
const STOCH_EXTREMES: &[f64] = &[5.0, 10.0, 15.0];
const BB_SQUEEZE_FACTORS: &[f64] = &[0.4, 0.5, 0.6];

// Walk-forward windows
struct Window {
    name: &'static str,
    train_start: &'static str,
    train_end: &'static str,
    test_start: &'static str,
    test_end: &'static str,
}

const WINDOWS: &[Window] = &[
    Window { name: "W1", train_start: "2025-10-15", train_end: "2025-12-14",
             test_start: "2025-12-15", test_end: "2026-01-14" },
    Window { name: "W2", train_start: "2025-11-15", train_end: "2026-01-14",
             test_start: "2026-01-15", test_end: "2026-02-14" },
    Window { name: "W3", train_start: "2025-12-15", train_end: "2026-02-14",
             test_start: "2026-02-15", test_end: "2026-03-10" },
];

#[derive(Debug, Clone)]
struct IsoShortParams {
    z_threshold: f64,
    bb_margin: f64,
    vol_mult: f64,
    adr_pct: f64,
    exit_z: f64,
    z_spread: f64,
    rsi_threshold: f64,
    vol_spike_mult: f64,
    squeeze_factor: f64,
}

impl Default for IsoShortParams {
    fn default() -> Self {
        Self {
            z_threshold: 1.5, bb_margin: 0.98, vol_mult: 1.2,
            adr_pct: 0.25, exit_z: -0.5, z_spread: 1.5,
            rsi_threshold: 75.0, vol_spike_mult: 2.0, squeeze_factor: 0.8,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ScalpParams {
    scalp_sl: f64,
    scalp_tp: f64,
    vol_spike_mult: f64,
    rsi_extreme: f64,
    stoch_extreme: f64,
    bb_squeeze_factor: f64,
}

#[derive(Debug, Clone)]
struct Candle {
    timestamp: String, // "YYYY-MM-DD HH:MM:SS"
    h: f64, l: f64, c: f64, v: f64,
}

#[derive(Debug, Clone, Default)]
struct Ind15m {
    sma20: f64, z: f64,
    bb_lo: f64, bb_hi: f64, bb_width: f64, bb_width_avg: f64,
    vol_ma: f64, adr_lo: f64, adr_hi: f64,
    rsi: f64,
    valid: bool,
}

#[derive(Debug, Clone, Default)]
struct Ind1m {
    rsi: f64, vol_ma: f64,
    stoch_k: f64, stoch_d: f64,
    stoch_k_prev: f64, stoch_d_prev: f64,
    bb_upper: f64, bb_lower: f64, bb_width: f64, bb_width_avg: f64,
    valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LongStrat { VwapReversion, BbBounce, AdrReversal, DualRsi, MeanReversion }
#[derive(Debug, Clone, Copy, PartialEq)]
enum ShortStrat { ShortMeanRev, ShortAdrRev, ShortBbBounce, ShortVwapRev }
#[derive(Debug, Clone, Copy, PartialEq)]
enum IsoShortStrat {
    IsoRelativeZ, IsoRsiExtreme, IsoDivergence, IsoMeanRev, IsoVwapRev,
    IsoBbBounce, IsoAdrRev, IsoVolSpike, IsoBbSqueeze,
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum Position { Long, Short }
#[derive(Debug, Clone, Copy, PartialEq)]
enum TradeType { Regime, Scalp }

#[derive(Debug, Clone)]
struct Trade { pnl_pct: f64, strat: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeStats { pf: f64, wr: f64, trades: usize, wins: usize }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestResult {
    balance: f64, pnl: f64, max_dd: f64,
    all: TradeStats, regime: TradeStats, scalp: TradeStats,
}

struct MarketCtx { avg_z: f64, avg_rsi: f64, btc_z: f64, avg_z_valid: bool, avg_rsi_valid: bool, btc_z_valid: bool }

// ---- Rolling helpers (same as run9_1) ----
fn rolling_mean(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if data.len() < window { return out; }
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..data.len() {
        if !data[i].is_nan() { sum += data[i]; count += 1; }
        if i >= window {
            if !data[i - window].is_nan() { sum -= data[i - window]; count -= 1; }
        }
        if i + 1 >= window && count == window { out[i] = sum / window as f64; }
    }
    out
}

fn rolling_std(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if data.len() < window { return out; }
    for i in (window - 1)..data.len() {
        let slice = &data[i + 1 - window..=i];
        let (mut sum, mut sum2, mut n) = (0.0, 0.0, 0usize);
        for &v in slice { if !v.is_nan() { sum += v; sum2 += v * v; n += 1; } }
        if n == window {
            let mean = sum / n as f64;
            let var = sum2 / n as f64 - mean * mean;
            out[i] = if var > 0.0 { var.sqrt() } else { 0.0 };
        }
    }
    out
}

fn rolling_min(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    for i in (window - 1)..data.len() {
        let mut m = f64::INFINITY;
        for j in (i + 1 - window)..=i { if !data[j].is_nan() && data[j] < m { m = data[j]; } }
        if m.is_finite() { out[i] = m; }
    }
    out
}

fn rolling_max(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    for i in (window - 1)..data.len() {
        let mut m = f64::NEG_INFINITY;
        for j in (i + 1 - window)..=i { if !data[j].is_nan() && data[j] > m { m = data[j]; } }
        if m.is_finite() { out[i] = m; }
    }
    out
}

fn rolling_sum(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if data.len() < window { return out; }
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..data.len() {
        if !data[i].is_nan() { sum += data[i]; count += 1; }
        if i >= window { if !data[i - window].is_nan() { sum -= data[i - window]; count -= 1; } }
        if i + 1 >= window && count == window { out[i] = sum; }
    }
    out
}

fn compute_rsi(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut rsi = vec![f64::NAN; n];
    if n < period + 1 { return rsi; }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = close[i] - close[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
    }
    let avg_gain = rolling_mean(&gains, period);
    let avg_loss = rolling_mean(&losses, period);
    for i in 0..n {
        if !avg_gain[i].is_nan() && !avg_loss[i].is_nan() {
            if avg_loss[i] == 0.0 { rsi[i] = 100.0; }
            else { rsi[i] = 100.0 - (100.0 / (1.0 + avg_gain[i] / avg_loss[i])); }
        }
    }
    rsi
}

fn compute_15m_indicators(candles: &[Candle]) -> Vec<Ind15m> {
    let n = candles.len();
    let c: Vec<f64> = candles.iter().map(|x| x.c).collect();
    let h: Vec<f64> = candles.iter().map(|x| x.h).collect();
    let l: Vec<f64> = candles.iter().map(|x| x.l).collect();
    let v: Vec<f64> = candles.iter().map(|x| x.v).collect();

    let sma20 = rolling_mean(&c, 20);
    let std20 = rolling_std(&c, 20);
    let vol_ma = rolling_mean(&v, 20);
    let adr_lo = rolling_min(&l, 24);
    let adr_hi = rolling_max(&h, 24);
    let rsi = compute_rsi(&c, 14);

    let tp: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let tp_v: Vec<f64> = (0..n).map(|i| tp[i] * v[i]).collect();
    let _tp_v_sum = rolling_sum(&tp_v, 20);
    let _v_sum = rolling_sum(&v, 20);

    let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() { bb_width_raw[i] = 4.0 * std20[i]; }
    }
    let bb_width_avg = rolling_mean(&bb_width_raw, 20);

    let mut inds = vec![Ind15m::default(); n];
    for i in 0..n {
        let valid = !sma20[i].is_nan() && !std20[i].is_nan() && !rsi[i].is_nan();
        let z = if valid && std20[i] > 0.0 { (c[i] - sma20[i]) / std20[i] } else { f64::NAN };
        inds[i] = Ind15m {
            sma20: sma20[i], z,
            bb_lo: if valid { sma20[i] - 2.0 * std20[i] } else { f64::NAN },
            bb_hi: if valid { sma20[i] + 2.0 * std20[i] } else { f64::NAN },
            bb_width: bb_width_raw[i], bb_width_avg: bb_width_avg[i],
            vol_ma: vol_ma[i], adr_lo: adr_lo[i], adr_hi: adr_hi[i],
            rsi: rsi[i], valid,
        };
    }
    inds
}

fn compute_1m_indicators(candles: &[Candle]) -> Vec<Ind1m> {
    let n = candles.len();
    let c: Vec<f64> = candles.iter().map(|x| x.c).collect();
    let h: Vec<f64> = candles.iter().map(|x| x.h).collect();
    let l: Vec<f64> = candles.iter().map(|x| x.l).collect();
    let v: Vec<f64> = candles.iter().map(|x| x.v).collect();

    let rsi = compute_rsi(&c, 14);
    let vol_ma = rolling_mean(&v, 20);
    let lowest_low = rolling_min(&l, 14);
    let highest_high = rolling_max(&h, 14);

    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !lowest_low[i].is_nan() && !highest_high[i].is_nan() {
            let range = highest_high[i] - lowest_low[i];
            if range > 0.0 { stoch_k[i] = 100.0 * (c[i] - lowest_low[i]) / range; }
        }
    }
    let stoch_d = rolling_mean(&stoch_k, 3);

    let bb_sma = rolling_mean(&c, 20);
    let bb_std = rolling_std(&c, 20);
    let mut bb_width_raw = vec![f64::NAN; n];
    let mut bb_upper = vec![f64::NAN; n];
    let mut bb_lower = vec![f64::NAN; n];
    for i in 0..n {
        if !bb_sma[i].is_nan() && !bb_std[i].is_nan() {
            bb_upper[i] = bb_sma[i] + 2.0 * bb_std[i];
            bb_lower[i] = bb_sma[i] - 2.0 * bb_std[i];
            bb_width_raw[i] = bb_upper[i] - bb_lower[i];
        }
    }
    let bb_width_avg = rolling_mean(&bb_width_raw, 20);

    let mut inds = vec![Ind1m::default(); n];
    for i in 0..n {
        inds[i] = Ind1m {
            rsi: rsi[i], vol_ma: vol_ma[i],
            stoch_k: stoch_k[i], stoch_d: stoch_d[i],
            stoch_k_prev: if i > 0 { stoch_k[i - 1] } else { f64::NAN },
            stoch_d_prev: if i > 0 { stoch_d[i - 1] } else { f64::NAN },
            bb_upper: bb_upper[i], bb_lower: bb_lower[i],
            bb_width: bb_width_raw[i], bb_width_avg: bb_width_avg[i],
            valid: !rsi[i].is_nan() && !vol_ma[i].is_nan(),
        };
    }
    inds
}

fn get_long_strat(coin: &str) -> LongStrat {
    match coin {
        "DOGE" | "BTC" => LongStrat::BbBounce,
        "XLM" => LongStrat::DualRsi,
        "AVAX" | "ALGO" => LongStrat::AdrReversal,
        _ => LongStrat::VwapReversion,
    }
}

fn get_short_strat(coin: &str) -> ShortStrat {
    match coin {
        "DASH" | "LTC" | "XLM" => ShortStrat::ShortMeanRev,
        "DOT" | "SHIB" | "BNB" => ShortStrat::ShortVwapRev,
        "ADA" | "LINK" | "XRP" | "DOGE" | "AVAX" => ShortStrat::ShortBbBounce,
        _ => ShortStrat::ShortAdrRev,
    }
}

fn long_entry(ind: &Ind15m, candle: &Candle, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if candle.c > ind.sma20 || ind.z > 0.5 { return false; }
    match strat {
        LongStrat::VwapReversion => ind.z < -1.5 && candle.c < ind.sma20 && candle.v > ind.vol_ma * 1.2,
        LongStrat::BbBounce => candle.c <= ind.bb_lo * 1.02 && candle.v > ind.vol_ma * 1.3,
        LongStrat::AdrReversal => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_lo.is_nan() && range > 0.0 && candle.c <= ind.adr_lo + range * 0.25
        }
        LongStrat::DualRsi => ind.z < -1.0,
        LongStrat::MeanReversion => ind.z < -1.5,
    }
}

fn short_entry(ind: &Ind15m, candle: &Candle, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if candle.c < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        ShortStrat::ShortVwapRev => ind.z > 1.5 && candle.c > ind.sma20 && candle.v > ind.vol_ma * 1.2,
        ShortStrat::ShortBbBounce => candle.c >= ind.bb_hi * 0.98 && candle.v > ind.vol_ma * 1.3,
        ShortStrat::ShortMeanRev => ind.z > 1.5,
        ShortStrat::ShortAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_hi.is_nan() && range > 0.0 && candle.c >= ind.adr_hi - range * 0.25
        }
    }
}

fn iso_short_entry(ind: &Ind15m, candle: &Candle, strat: IsoShortStrat,
                   params: &IsoShortParams, ctx: &MarketCtx) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if candle.c < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        IsoShortStrat::IsoRelativeZ => ctx.avg_z_valid && ind.z > ctx.avg_z + params.z_spread,
        IsoShortStrat::IsoRsiExtreme => ctx.avg_rsi_valid && !ind.rsi.is_nan() && ind.rsi > params.rsi_threshold && ctx.avg_rsi < 55.0,
        IsoShortStrat::IsoDivergence => ctx.btc_z_valid && ind.z > params.z_threshold && ctx.btc_z < 0.0,
        IsoShortStrat::IsoMeanRev => ind.z > params.z_threshold,
        IsoShortStrat::IsoVwapRev => ind.z > params.z_threshold && candle.c > ind.sma20 && candle.v > ind.vol_ma * params.vol_mult,
        IsoShortStrat::IsoBbBounce => candle.c >= ind.bb_hi * params.bb_margin && candle.v > ind.vol_ma * (params.vol_mult + 0.1),
        IsoShortStrat::IsoAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            range > 0.0 && candle.c >= ind.adr_hi - range * params.adr_pct && candle.v > ind.vol_ma * params.vol_mult
        }
        IsoShortStrat::IsoVolSpike => ind.z > 1.0 && candle.v > ind.vol_ma * params.vol_spike_mult,
        IsoShortStrat::IsoBbSqueeze => {
            !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 &&
            candle.c >= ind.bb_hi * 0.98 && ind.bb_width < ind.bb_width_avg * params.squeeze_factor
        }
    }
}

fn scalp_entry(ind: &Ind1m, candle: &Candle, params: &ScalpParams) -> Option<(Position, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }
    let vol_r = candle.v / ind.vol_ma;
    let rsi_low = params.rsi_extreme;
    let rsi_high = 100.0 - params.rsi_extreme;

    if vol_r > params.vol_spike_mult {
        if ind.rsi < rsi_low { return Some((Position::Long, "scalp_vol_spike_rev")); }
        if ind.rsi > rsi_high { return Some((Position::Short, "scalp_vol_spike_rev")); }
    }
    if !ind.stoch_k.is_nan() && !ind.stoch_d.is_nan() && !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan() {
        let stoch_lo = params.stoch_extreme;
        let stoch_hi = 100.0 - params.stoch_extreme;
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d && ind.stoch_k < stoch_lo && ind.stoch_d < stoch_lo {
            return Some((Position::Long, "scalp_stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d && ind.stoch_k > stoch_hi && ind.stoch_d > stoch_hi {
            return Some((Position::Short, "scalp_stoch_cross"));
        }
    }
    if !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 && !ind.bb_upper.is_nan() {
        let squeeze = ind.bb_width < ind.bb_width_avg * params.bb_squeeze_factor;
        if squeeze && vol_r > 2.0 {
            if candle.c > ind.bb_upper { return Some((Position::Long, "scalp_bb_squeeze_break")); }
            if candle.c < ind.bb_lower { return Some((Position::Short, "scalp_bb_squeeze_break")); }
        }
    }
    None
}

fn calc_stats(trades: &[Trade]) -> TradeStats {
    if trades.is_empty() { return TradeStats { pf: 0.0, wr: 0.0, trades: 0, wins: 0 }; }
    let wins: Vec<&Trade> = trades.iter().filter(|t| t.pnl_pct > 0.0).collect();
    let tw: f64 = wins.iter().map(|t| t.pnl_pct).sum();
    let tl: f64 = trades.iter().filter(|t| t.pnl_pct <= 0.0).map(|t| t.pnl_pct).sum();
    let pf = if tl != 0.0 { (tw / tl).abs() } else if tw > 0.0 { 999.0 } else { 0.0 };
    TradeStats { pf, wr: wins.len() as f64 / trades.len() as f64 * 100.0, trades: trades.len(), wins: wins.len() }
}

/// Holds pre-computed candles + indicators for a coin, with timestamp index for slicing
struct CoinFullData {
    candles_15m: Vec<Candle>,
    candles_1m: Vec<Candle>,
    ind_15m: Vec<Ind15m>,
    ind_1m: Vec<Ind1m>,
    long_strat: LongStrat,
    short_strat: ShortStrat,
}

/// A sliced view into CoinFullData for a date range
struct CoinSlice {
    start_15m: usize,
    end_15m: usize,
    start_1m: usize,
    end_1m: usize,
}

fn find_range(candles: &[Candle], start: &str, end: &str) -> (usize, usize) {
    // Timestamps are "YYYY-MM-DD HH:MM:SS", date strings are "YYYY-MM-DD"
    // start is inclusive (>=), end comparisons: use <= for test_end, < for train_end
    // We'll return range where timestamp >= start and timestamp starts with date <= end
    let s = candles.iter().position(|c| &c.timestamp[..10] >= start).unwrap_or(candles.len());
    // Find first candle past end date
    let e = candles.iter().rposition(|c| &c.timestamp[..10] <= end).map(|i| i + 1).unwrap_or(s);
    (s, e)
}

fn run_backtest_slice(
    coin: &CoinFullData,
    s15: &CoinSlice,
    breadth: &[f64],  // indexed same as full 15m
    avg_z: &[f64],
    avg_rsi: &[f64],
    btc_z: &[f64],
    scalp_params: Option<&ScalpParams>,
    iso_params: &IsoShortParams,
) -> Option<BacktestResult> {
    let candles_15m = &coin.candles_15m[s15.start_15m..s15.end_15m];
    let ind_15m = &coin.ind_15m[s15.start_15m..s15.end_15m];

    let first_valid = ind_15m.iter().position(|x| x.valid)?;
    if candles_15m.len() - first_valid < 50 { return None; }

    let candles_1m = &coin.candles_1m[s15.start_1m..s15.end_1m];
    let ind_1m = &coin.ind_1m[s15.start_1m..s15.end_1m];
    let n_1m = candles_1m.len();

    // Build m1_windows for this slice
    // Each 15m candle maps to 15 consecutive 1m candles
    let n_15m = candles_15m.len();
    let mut m1_windows: Vec<(usize, usize)> = Vec::with_capacity(n_15m);
    for i in 0..n_15m {
        let start = i * 15;
        let end = ((i + 1) * 15).min(n_1m);
        if start >= n_1m { m1_windows.push((n_1m, n_1m)); }
        else { m1_windows.push((start, end)); }
    }

    let mut balance = INITIAL_CAPITAL;
    let mut peak_balance = INITIAL_CAPITAL;
    let mut max_dd: f64 = 0.0;
    let mut position: Option<Position> = None;
    let mut trade_type: Option<TradeType> = None;
    let mut entry_price = 0.0;
    let mut peak_price = 0.0;
    let mut trough_price = 0.0;
    let mut cooldown: usize = 0;
    let mut candles_held: usize = 0;
    let mut regime_trades: Vec<Trade> = Vec::new();
    let mut scalp_trades: Vec<Trade> = Vec::new();
    let mut all_trades: Vec<Trade> = Vec::new();

    for local_i in first_valid..n_15m {
        let global_i = s15.start_15m + local_i;
        let candle = &candles_15m[local_i];
        let ind = &ind_15m[local_i];
        if !ind.valid { continue; }
        let price = candle.c;

        let b = if global_i < breadth.len() { breadth[global_i] } else { 0.0 };
        let market_mode = if b <= BREADTH_LONG_MAX { 0 } else if b >= BREADTH_SHORT_MIN { 2 } else { 1 };

        // === SCALP EXIT ===
        if let (Some(pos), Some(TradeType::Scalp)) = (position, trade_type) {
            if let Some(sp) = scalp_params {
                let (ws, we) = m1_windows[local_i];
                for j in ws..we {
                    if j >= n_1m { break; }
                    let p = candles_1m[j].c;
                    let pnl = match pos {
                        Position::Long => (p - entry_price) / entry_price,
                        Position::Short => (entry_price - p) / entry_price,
                    };
                    if pnl >= sp.scalp_tp {
                        balance += balance * SCALP_RISK * sp.scalp_tp * LEVERAGE;
                        let t = Trade { pnl_pct: sp.scalp_tp * LEVERAGE * 100.0, strat: None };
                        all_trades.push(t.clone()); scalp_trades.push(t);
                        position = None; trade_type = None; cooldown = 0; break;
                    } else if pnl <= -sp.scalp_sl {
                        balance -= balance * SCALP_RISK * sp.scalp_sl * LEVERAGE;
                        let t = Trade { pnl_pct: -sp.scalp_sl * LEVERAGE * 100.0, strat: None };
                        all_trades.push(t.clone()); scalp_trades.push(t);
                        position = None; trade_type = None; cooldown = 0; break;
                    }
                }
            }
        }

        // === REGIME EXIT ===
        if let (Some(pos), Some(TradeType::Regime)) = (position, trade_type) {
            candles_held += 1;
            let mut exited = false;
            match pos {
                Position::Long => {
                    if candle.h > peak_price { peak_price = candle.h; }
                    let price_pnl = (price - entry_price) / entry_price;
                    if price_pnl <= -REGIME_SL {
                        balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE;
                        let t = Trade { pnl_pct: -REGIME_SL * LEVERAGE * 100.0, strat: None };
                        all_trades.push(t.clone()); regime_trades.push(t); exited = true;
                    }
                    if !exited && price_pnl > 0.0 && candles_held >= MIN_HOLD {
                        if candle.c > ind.sma20 || ind.z > 0.5 {
                            balance += balance * REGIME_RISK * price_pnl * LEVERAGE;
                            let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0, strat: None };
                            all_trades.push(t.clone()); regime_trades.push(t); exited = true;
                        }
                    }
                }
                Position::Short => {
                    if candle.l < trough_price { trough_price = candle.l; }
                    let price_pnl = (entry_price - price) / entry_price;
                    if price_pnl <= -REGIME_SL {
                        balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE;
                        let t = Trade { pnl_pct: -REGIME_SL * LEVERAGE * 100.0, strat: None };
                        all_trades.push(t.clone()); regime_trades.push(t); exited = true;
                    }
                    if !exited && price_pnl > 0.0 && candles_held >= MIN_HOLD {
                        if price < ind.sma20 || ind.z < iso_params.exit_z {
                            balance += balance * REGIME_RISK * price_pnl * LEVERAGE;
                            let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0, strat: None };
                            all_trades.push(t.clone()); regime_trades.push(t); exited = true;
                        }
                    }
                }
            }
            if exited { position = None; trade_type = None; cooldown = 2; candles_held = 0; }
        }

        if cooldown > 0 { cooldown -= 1; }

        // === REGIME ENTRY ===
        if position.is_none() && cooldown == 0 {
            let ctx = MarketCtx {
                avg_z: if global_i < avg_z.len() { avg_z[global_i] } else { f64::NAN },
                avg_rsi: if global_i < avg_rsi.len() { avg_rsi[global_i] } else { f64::NAN },
                btc_z: if global_i < btc_z.len() { btc_z[global_i] } else { f64::NAN },
                avg_z_valid: global_i < avg_z.len() && !avg_z[global_i].is_nan(),
                avg_rsi_valid: global_i < avg_rsi.len() && !avg_rsi[global_i].is_nan(),
                btc_z_valid: global_i < btc_z.len() && !btc_z[global_i].is_nan(),
            };
            match market_mode {
                0 => {
                    if long_entry(ind, candle, coin.long_strat) {
                        position = Some(Position::Long); trade_type = Some(TradeType::Regime);
                        entry_price = price; peak_price = candle.h; candles_held = 0;
                    }
                    // iso_short_strat is None currently, skip
                }
                1 => {} // iso_short — skip (no iso strats)
                2 => {
                    if short_entry(ind, candle, coin.short_strat) {
                        position = Some(Position::Short); trade_type = Some(TradeType::Regime);
                        entry_price = price; trough_price = candle.l; candles_held = 0;
                    }
                }
                _ => {}
            }
            let _ = ctx; // suppress warning
        }

        // === SCALP ENTRY ===
        if let Some(sp) = scalp_params {
            if position.is_none() && cooldown == 0 {
                let (ws, we) = m1_windows[local_i];
                let mut j = ws;
                while j < we && j < n_1m {
                    if position.is_some() { break; }
                    if let Some((dir, strat_name)) = scalp_entry(&ind_1m[j], &candles_1m[j], sp) {
                        position = Some(dir); trade_type = Some(TradeType::Scalp);
                        entry_price = candles_1m[j].c;
                        for k in (j + 1)..we {
                            if k >= n_1m || position.is_none() { break; }
                            let p = candles_1m[k].c;
                            let pnl = match dir {
                                Position::Long => (p - entry_price) / entry_price,
                                Position::Short => (entry_price - p) / entry_price,
                            };
                            if pnl >= sp.scalp_tp {
                                balance += balance * SCALP_RISK * sp.scalp_tp * LEVERAGE;
                                let t = Trade { pnl_pct: sp.scalp_tp * LEVERAGE * 100.0, strat: Some(strat_name.into()) };
                                all_trades.push(t.clone()); scalp_trades.push(t);
                                position = None; trade_type = None; break;
                            } else if pnl <= -sp.scalp_sl {
                                balance -= balance * SCALP_RISK * sp.scalp_sl * LEVERAGE;
                                let t = Trade { pnl_pct: -sp.scalp_sl * LEVERAGE * 100.0, strat: Some(strat_name.into()) };
                                all_trades.push(t.clone()); scalp_trades.push(t);
                                position = None; trade_type = None; break;
                            }
                        }
                    }
                    j += 1;
                }
            }
        }

        if balance > peak_balance { peak_balance = balance; }
        let dd = (peak_balance - balance) / peak_balance * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    // Close open position
    if let Some(pos) = position {
        let last_price = candles_15m.last()?.c;
        let price_pnl = match pos {
            Position::Long => (last_price - entry_price) / entry_price,
            Position::Short => (entry_price - last_price) / entry_price,
        };
        let risk = if trade_type == Some(TradeType::Regime) { REGIME_RISK } else { SCALP_RISK };
        balance += balance * risk * price_pnl * LEVERAGE;
        let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0, strat: None };
        all_trades.push(t.clone());
        if trade_type == Some(TradeType::Regime) { regime_trades.push(t); }
        else { scalp_trades.push(t); }
    }

    if all_trades.is_empty() { return None; }

    Some(BacktestResult {
        balance, pnl: (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100.0, max_dd,
        all: calc_stats(&all_trades), regime: calc_stats(&regime_trades), scalp: calc_stats(&scalp_trades),
    })
}

fn load_csv(path: &str) -> Option<Vec<Candle>> {
    let content = fs::read_to_string(path).ok()?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(content.as_bytes());
    let headers = rdr.headers().ok()?.clone();
    let find_col = |names: &[&str]| -> Option<usize> {
        for name in names { if let Some(pos) = headers.iter().position(|h| h == *name) { return Some(pos); } }
        None
    };
    let t_idx = find_col(&["t", "timestamp"])?;
    let h_idx = find_col(&["high", "h"])?;
    let l_idx = find_col(&["low", "l"])?;
    let c_idx = find_col(&["close", "c"])?;
    let v_idx = find_col(&["volume", "v"])?;
    let mut candles = Vec::new();
    for result in rdr.records() {
        let record = result.ok()?;
        let t: String = record.get(t_idx)?.to_string();
        let h: f64 = record.get(h_idx)?.parse().ok()?;
        let l: f64 = record.get(l_idx)?.parse().ok()?;
        let c: f64 = record.get(c_idx)?.parse().ok()?;
        let v: f64 = record.get(v_idx)?.parse().ok()?;
        candles.push(Candle { timestamp: t, h, l, c, v });
    }
    Some(candles)
}

fn build_combos() -> Vec<ScalpParams> {
    let mut combos = Vec::new();
    for &sl in SCALP_SLS { for &tp in SCALP_TPS { for &vm in VOL_SPIKE_MULTS {
        for &rsi in RSI_EXTREMES { for &st in STOCH_EXTREMES { for &bb in BB_SQUEEZE_FACTORS {
            combos.push(ScalpParams { scalp_sl: sl, scalp_tp: tp, vol_spike_mult: vm,
                rsi_extreme: rsi, stoch_extreme: st, bb_squeeze_factor: bb });
        }}}
    }}}
    combos
}

fn main() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let s = shutdown.clone();
    ctrlc::set_handler(move || {
        eprintln!("\nSIGINT received, will save after current task...");
        s.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("{}", "=".repeat(90));
    println!("RUN9.2 - WALK-FORWARD VALIDATION (Rust + Rayon)");
    println!("{}", "=".repeat(90));
    println!("Train: 2 months | Test: 1 month | 3 windows");

    let combos = build_combos();
    println!("Scalp combos: {}", combos.len());

    // Load universal params from run9_1
    let universal_params: Option<ScalpParams> = fs::read_to_string(R91_FILE).ok().and_then(|content| {
        let v: serde_json::Value = serde_json::from_str(&content).ok()?;
        let bp = v.get("best_params")?;
        serde_json::from_value(bp.clone()).ok()
    });
    if let Some(ref up) = universal_params {
        println!("Loaded universal params from RUN9.1: sl={} tp={} vm={} rsi={} st={} bb={}",
            up.scalp_sl, up.scalp_tp, up.vol_spike_mult, up.rsi_extreme, up.stoch_extreme, up.bb_squeeze_factor);
    } else {
        println!("WARNING: No universal params from run9_1_results.json, will search all combos in train");
    }

    // Load data
    println!("\nLoading data...");
    let mut all_coins: HashMap<String, CoinFullData> = HashMap::new();

    for &coin in COINS {
        let path_15m = format!("{}/{}_USDT_15m_5months.csv", DATA_CACHE_DIR, coin);
        let path_1m = format!("{}/{}_USDT_1m_5months.csv", DATA_CACHE_DIR, coin);
        let candles_15m = match load_csv(&path_15m) {
            Some(c) => c, None => { eprintln!("  Missing 15m data for {}", coin); continue; }
        };
        let candles_1m = match load_csv(&path_1m) {
            Some(c) => c, None => { eprintln!("  Missing 1m data for {}", coin); continue; }
        };
        println!("  {} - 15m: {} candles, 1m: {} candles", coin, candles_15m.len(), candles_1m.len());
        let ind_15m = compute_15m_indicators(&candles_15m);
        let ind_1m = compute_1m_indicators(&candles_1m);
        all_coins.insert(coin.to_string(), CoinFullData {
            candles_15m, candles_1m, ind_15m, ind_1m,
            long_strat: get_long_strat(coin), short_strat: get_short_strat(coin),
        });
    }
    println!("Loaded {} coins", all_coins.len());

    // Build market breadth from full 15m data
    println!("Computing market breadth...");
    let n_15m = all_coins.values().map(|c| c.candles_15m.len()).max().unwrap_or(0);
    let mut z_all: HashMap<String, Vec<f64>> = HashMap::new();
    let mut rsi_all: HashMap<String, Vec<f64>> = HashMap::new();
    for (coin, cd) in &all_coins {
        z_all.insert(coin.clone(), cd.ind_15m.iter().map(|x| x.z).collect());
        rsi_all.insert(coin.clone(), cd.ind_15m.iter().map(|x| x.rsi).collect());
    }
    let mut breadth = vec![0.0f64; n_15m];
    let mut avg_z_vec = vec![f64::NAN; n_15m];
    let mut avg_rsi_vec = vec![f64::NAN; n_15m];
    for i in 0..n_15m {
        let (mut below, mut total, mut zsum) = (0usize, 0usize, 0.0);
        let (mut rsum, mut rcount) = (0.0, 0usize);
        for zs in z_all.values() {
            if i < zs.len() && !zs[i].is_nan() { if zs[i] < -1.0 { below += 1; } zsum += zs[i]; total += 1; }
        }
        for rs in rsi_all.values() {
            if i < rs.len() && !rs[i].is_nan() { rsum += rs[i]; rcount += 1; }
        }
        if total > 0 { breadth[i] = below as f64 / total as f64; avg_z_vec[i] = zsum / total as f64; }
        if rcount > 0 { avg_rsi_vec[i] = rsum / rcount as f64; }
    }
    let btc_z_vec: Vec<f64> = z_all.get("BTC").cloned().unwrap_or_default();

    let iso_params = IsoShortParams::default();
    let all_coins = Arc::new(all_coins);
    let breadth = Arc::new(breadth);
    let avg_z_vec = Arc::new(avg_z_vec);
    let avg_rsi_vec = Arc::new(avg_rsi_vec);
    let btc_z_vec = Arc::new(btc_z_vec);
    let combos = Arc::new(combos);

    // Build task list: (coin, window_idx)
    let mut tasks: Vec<(String, usize)> = Vec::new();
    for &coin in COINS {
        if all_coins.contains_key(coin) {
            for wi in 0..WINDOWS.len() { tasks.push((coin.to_string(), wi)); }
        }
    }

    // Load checkpoint
    let mut done_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut saved_results: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    if let Ok(content) = fs::read_to_string(CHECKPOINT_FILE) {
        if let Ok(cp) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(dk) = cp.get("done_keys").and_then(|v| v.as_array()) {
                for k in dk { if let Some(s) = k.as_str() { done_keys.insert(s.to_string()); } }
            }
            if let Some(ar) = cp.get("all_results").and_then(|v| v.as_object()) {
                for (k, v) in ar {
                    if let Some(arr) = v.as_array() {
                        saved_results.insert(k.clone(), arr.clone());
                    }
                }
            }
            println!("Resumed from checkpoint: {}/{} tasks done", done_keys.len(), tasks.len());
        }
    }

    let total_tasks = tasks.len();
    let counter = AtomicUsize::new(done_keys.len());
    let start = Instant::now();

    println!("\nStarting walk-forward: {} tasks ({} coins x {} windows)",
        total_tasks, COINS.len(), WINDOWS.len());

    // Run tasks in parallel
    let results: Vec<(String, usize, serde_json::Value)> = tasks.par_iter()
        .filter_map(|(coin, wi)| {
            let task_key = format!("{}_{}", coin, WINDOWS[*wi].name);
            if done_keys.contains(&task_key) { return None; }
            if shutdown.load(Ordering::SeqCst) { return None; }

            let w = &WINDOWS[*wi];
            let cd = all_coins.get(coin.as_str())?;

            // Slice data for train and test
            let (train_s15, train_e15) = find_range(&cd.candles_15m, w.train_start, w.train_end);
            let (test_s15, test_e15) = find_range(&cd.candles_15m, w.test_start, w.test_end);
            let (train_s1m, train_e1m) = find_range(&cd.candles_1m, w.train_start, w.train_end);
            let (test_s1m, test_e1m) = find_range(&cd.candles_1m, w.test_start, w.test_end);

            if train_e15 - train_s15 < 100 || test_e15 - test_s15 < 50 {
                let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
                eprintln!("  [{}/{}] {} {} - insufficient data, skipping", done, total_tasks, coin, w.name);
                return Some((coin.clone(), *wi, serde_json::json!({
                    "window": w.name, "skipped": true
                })));
            }

            let train_slice = CoinSlice { start_15m: train_s15, end_15m: train_e15, start_1m: train_s1m, end_1m: train_e1m };
            let test_slice = CoinSlice { start_15m: test_s15, end_15m: test_e15, start_1m: test_s1m, end_1m: test_e1m };

            // Baseline: no scalps on test
            let test_base = run_backtest_slice(cd, &test_slice, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec, None, &iso_params);

            // Train: find best combo
            let mut best_train_score: f64 = -999.0;
            let mut best_train_combo: Option<ScalpParams> = None;
            let mut best_train_pf: f64 = 0.0;

            for params in combos.iter() {
                let r = run_backtest_slice(cd, &train_slice, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec, Some(params), &iso_params);
                if let Some(r) = r {
                    if r.all.trades >= 3 {
                        let score = r.all.pf * (r.all.wr / 100.0).sqrt();
                        if score > best_train_score {
                            best_train_score = score;
                            best_train_combo = Some(*params);
                            best_train_pf = r.all.pf;
                        }
                    }
                }
            }

            // Test: per-coin best
            let test_percoin = best_train_combo.as_ref().and_then(|p| {
                run_backtest_slice(cd, &test_slice, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec, Some(p), &iso_params)
            });

            // Test: universal params
            let test_universal = universal_params.as_ref().and_then(|p| {
                run_backtest_slice(cd, &test_slice, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec, Some(p), &iso_params)
            });

            let result = serde_json::json!({
                "window": w.name,
                "train_best_combo": best_train_combo,
                "train_best_pf": best_train_pf,
                "test_base_pf": test_base.as_ref().map(|r| r.all.pf).unwrap_or(0.0),
                "test_base_wr": test_base.as_ref().map(|r| r.all.wr).unwrap_or(0.0),
                "test_base_pnl": test_base.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                "test_base_trades": test_base.as_ref().map(|r| r.all.trades).unwrap_or(0),
                "test_percoin_pf": test_percoin.as_ref().map(|r| r.all.pf).unwrap_or(0.0),
                "test_percoin_wr": test_percoin.as_ref().map(|r| r.all.wr).unwrap_or(0.0),
                "test_percoin_pnl": test_percoin.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                "test_percoin_trades": test_percoin.as_ref().map(|r| r.all.trades).unwrap_or(0),
                "test_percoin_scalps": test_percoin.as_ref().map(|r| r.scalp.trades).unwrap_or(0),
                "test_universal_pf": test_universal.as_ref().map(|r| r.all.pf).unwrap_or(0.0),
                "test_universal_wr": test_universal.as_ref().map(|r| r.all.wr).unwrap_or(0.0),
                "test_universal_pnl": test_universal.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                "test_universal_trades": test_universal.as_ref().map(|r| r.all.trades).unwrap_or(0),
                "test_universal_scalps": test_universal.as_ref().map(|r| r.scalp.trades).unwrap_or(0),
            });

            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let elapsed = start.elapsed().as_secs_f64();
            let rate = (done - done_keys.len()) as f64 / elapsed;
            let eta = if rate > 0.0 { (total_tasks - done) as f64 / rate / 60.0 } else { 0.0 };
            eprintln!("  [{}/{}] {} {} | base_pnl={:+.1}% percoin_pnl={:+.1}% univ_pnl={:+.1}% | {:.2}/s ETA:{:.1}m",
                done, total_tasks, coin, w.name,
                test_base.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                test_percoin.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                test_universal.as_ref().map(|r| r.pnl).unwrap_or(0.0),
                rate, eta);

            Some((coin.clone(), *wi, result))
        })
        .collect();

    // Merge results
    let mut all_results: HashMap<String, Vec<serde_json::Value>> = saved_results;
    for (coin, _wi, result) in &results {
        all_results.entry(coin.clone()).or_default().push(result.clone());
    }

    // Save checkpoint
    let all_done_keys: Vec<String> = {
        let mut dk = done_keys.into_iter().collect::<Vec<_>>();
        for (coin, wi, _) in &results {
            dk.push(format!("{}_{}", coin, WINDOWS[*wi].name));
        }
        dk
    };
    let checkpoint = serde_json::json!({ "done_keys": all_done_keys, "all_results": all_results });
    fs::write(CHECKPOINT_FILE, serde_json::to_string(&checkpoint).unwrap()).ok();

    // === PRINT RESULTS ===
    println!("\n{}", "=".repeat(90));
    println!("WALK-FORWARD RESULTS BY COIN");
    println!("{}", "=".repeat(90));

    for &coin in COINS {
        if let Some(wins) = all_results.get(coin) {
            let valid_wins: Vec<&serde_json::Value> = wins.iter()
                .filter(|w| !w.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false)).collect();
            if valid_wins.is_empty() { continue; }
            println!("\n{}", coin);
            println!("  {:<4} {:<10} {:<10} {:<12} {:<12} {:<10} {:<10} Scalps#",
                "Win", "Base PF", "Base WR", "PerCoin PF", "PerCoin WR", "Univ PF", "Univ WR");
            println!("  {}", "-".repeat(95));
            for w in &valid_wins {
                let wname = w["window"].as_str().unwrap_or("?");
                let low_conf = if w["test_base_trades"].as_u64().unwrap_or(0) < 3 { " *" } else { "" };
                println!("  {:<4} {:<10.2} {:<10.1} {:<12.2} {:<12.1} {:<10.2} {:<10.1} {}{}",
                    wname,
                    w["test_base_pf"].as_f64().unwrap_or(0.0),
                    w["test_base_wr"].as_f64().unwrap_or(0.0),
                    w["test_percoin_pf"].as_f64().unwrap_or(0.0),
                    w["test_percoin_wr"].as_f64().unwrap_or(0.0),
                    w["test_universal_pf"].as_f64().unwrap_or(0.0),
                    w["test_universal_wr"].as_f64().unwrap_or(0.0),
                    w["test_universal_scalps"].as_u64().unwrap_or(0),
                    low_conf);
            }
        }
    }

    // === DEGRADATION ANALYSIS ===
    println!("\n{}", "=".repeat(90));
    println!("DEGRADATION ANALYSIS");
    println!("{}", "=".repeat(90));

    let mut train_pfs_pc: Vec<f64> = Vec::new();
    let mut test_pfs_pc: Vec<f64> = Vec::new();
    let mut test_pfs_univ: Vec<f64> = Vec::new();
    let mut test_pfs_base: Vec<f64> = Vec::new();

    for wins in all_results.values() {
        for w in wins {
            if w.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false) { continue; }
            let train_pf = w["train_best_pf"].as_f64().unwrap_or(0.0);
            if train_pf > 0.0 {
                train_pfs_pc.push(train_pf);
                test_pfs_pc.push(w["test_percoin_pf"].as_f64().unwrap_or(0.0));
            }
            test_pfs_univ.push(w["test_universal_pf"].as_f64().unwrap_or(0.0));
            test_pfs_base.push(w["test_base_pf"].as_f64().unwrap_or(0.0));
        }
    }

    let avg = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    let avg_train_pc = avg(&train_pfs_pc);
    let avg_test_pc = avg(&test_pfs_pc);
    let avg_test_univ = avg(&test_pfs_univ);
    let avg_test_base = avg(&test_pfs_base);
    let deg_pc = if avg_train_pc > 0.0 { (1.0 - avg_test_pc / avg_train_pc) * 100.0 } else { 0.0 };

    println!("\n  Per-coin: Train PF {:.2} -> Test PF {:.2}  (degradation: {:.1}%)", avg_train_pc, avg_test_pc, deg_pc);
    println!("  Universal: Test PF {:.2}", avg_test_univ);
    println!("  Baseline (no scalps): Test PF {:.2}", avg_test_base);

    let (recommendation, rec_str) = if deg_pc > 40.0 || avg_test_univ > avg_test_pc {
        println!("\n  RECOMMENDATION: Universal params preferred (per-coin degrades {:.0}%)", deg_pc);
        ("universal", "universal")
    } else {
        println!("\n  RECOMMENDATION: Per-coin params viable (degradation {:.0}%)", deg_pc);
        ("per_coin", "per_coin")
    };

    let best_test = avg_test_univ.max(avg_test_pc);
    if best_test > avg_test_base {
        println!("\n  VERDICT: Scalps ({:.2}) BEAT baseline ({:.2})", best_test, avg_test_base);
    } else {
        println!("\n  VERDICT: Baseline ({:.2}) is already optimal -- scalps don't help OOS", avg_test_base);
    }

    // Save final results
    let save_data = serde_json::json!({
        "recommendation": rec_str,
        "avg_train_pf_percoin": avg_train_pc,
        "avg_test_pf_percoin": avg_test_pc,
        "avg_test_pf_universal": avg_test_univ,
        "avg_test_pf_baseline": avg_test_base,
        "degradation_percoin_pct": deg_pc,
        "universal_params": universal_params,
        "coin_results": all_results,
    });
    fs::write(RESULTS_FILE, serde_json::to_string_pretty(&save_data).unwrap()).expect("Failed to write results");
    println!("\nResults saved to {}", RESULTS_FILE);

    // Clean up checkpoint on success
    if counter.load(Ordering::Relaxed) >= total_tasks {
        fs::remove_file(CHECKPOINT_FILE).ok();
        println!("Checkpoint removed (clean finish)");
    }

    let elapsed = start.elapsed();
    println!("Total time: {:.1}s", elapsed.as_secs_f64());
    let _ = recommendation;
}
