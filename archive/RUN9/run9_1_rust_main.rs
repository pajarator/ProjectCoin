use csv::ReaderBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const DATA_CACHE_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run9_1_results.json";
const CHECKPOINT_FILE: &str = "/home/scamarena/ProjectCoin/run9_1_rust_checkpoint.json";

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
    o: f64, h: f64, l: f64, c: f64, v: f64,
}

#[derive(Debug, Clone, Default)]
struct Ind15m {
    sma20: f64, sma9: f64, std20: f64, z: f64,
    bb_lo: f64, bb_hi: f64, bb_width: f64, bb_width_avg: f64,
    vol_ma: f64, adr_lo: f64, adr_hi: f64,
    rsi: f64, vwap: f64, adx: f64,
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
struct Trade {
    pnl_pct: f64,
    trade_type: TradeType,
    dir: Position,
    reason: String,
    strat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeStats {
    pf: f64, wr: f64, trades: usize, wins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestResult {
    balance: f64,
    pnl: f64,
    max_dd: f64,
    all: TradeStats,
    regime: TradeStats,
    scalp: TradeStats,
    scalp_by_strat: HashMap<String, TradeStats>,
}

struct MarketCtx {
    avg_z: f64,
    avg_rsi: f64,
    btc_z: f64,
    avg_z_valid: bool,
    avg_rsi_valid: bool,
    btc_z_valid: bool,
}

// ---- Rolling helpers ----
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
        if i + 1 >= window && count == window {
            out[i] = sum / window as f64;
        }
    }
    out
}

fn rolling_std(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if data.len() < window { return out; }
    for i in (window - 1)..data.len() {
        let slice = &data[i + 1 - window..=i];
        let mut sum = 0.0;
        let mut sum2 = 0.0;
        let mut n = 0usize;
        for &v in slice {
            if !v.is_nan() { sum += v; sum2 += v * v; n += 1; }
        }
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
        for j in (i + 1 - window)..=i {
            if !data[j].is_nan() && data[j] < m { m = data[j]; }
        }
        if m.is_finite() { out[i] = m; }
    }
    out
}

fn rolling_max(data: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    for i in (window - 1)..data.len() {
        let mut m = f64::NEG_INFINITY;
        for j in (i + 1 - window)..=i {
            if !data[j].is_nan() && data[j] > m { m = data[j]; }
        }
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
        if i >= window {
            if !data[i - window].is_nan() { sum -= data[i - window]; count -= 1; }
        }
        if i + 1 >= window && count == window {
            out[i] = sum;
        }
    }
    out
}

fn compute_rsi(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut rsi = vec![f64::NAN; n];
    if n < period + 1 { return rsi; }
    let mut deltas = vec![f64::NAN; n];
    for i in 1..n { deltas[i] = close[i] - close[i - 1]; }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        if !deltas[i].is_nan() {
            if deltas[i] > 0.0 { gains[i] = deltas[i]; }
            else { losses[i] = -deltas[i]; }
        }
    }
    let avg_gain = rolling_mean(&gains, period);
    let avg_loss = rolling_mean(&losses, period);
    for i in 0..n {
        if !avg_gain[i].is_nan() && !avg_loss[i].is_nan() {
            if avg_loss[i] == 0.0 {
                rsi[i] = 100.0;
            } else {
                let rs = avg_gain[i] / avg_loss[i];
                rsi[i] = 100.0 - (100.0 / (1.0 + rs));
            }
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
    let sma9 = rolling_mean(&c, 9);
    let std20 = rolling_std(&c, 20);
    let vol_ma = rolling_mean(&v, 20);
    let adr_lo = rolling_min(&l, 24);
    let adr_hi = rolling_max(&h, 24);
    let rsi = compute_rsi(&c, 14);

    let tp: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let tp_v: Vec<f64> = (0..n).map(|i| tp[i] * v[i]).collect();
    let tp_v_sum = rolling_sum(&tp_v, 20);
    let v_sum = rolling_sum(&v, 20);

    // BB width avg
    let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() {
            bb_width_raw[i] = 4.0 * std20[i]; // (sma+2*std) - (sma-2*std)
        }
    }
    let bb_width_avg = rolling_mean(&bb_width_raw, 20);

    // ADX computation
    let mut high_low = vec![0.0; n];
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    let mut true_range = vec![0.0; n];
    for i in 1..n {
        high_low[i] = h[i] - l[i];
        let hh = h[i] - h[i - 1];
        let ll = l[i - 1] - l[i];
        if hh > ll && hh > 0.0 { plus_dm[i] = hh; }
        if ll > hh && ll > 0.0 { minus_dm[i] = ll; }
        let a = high_low[i];
        let b = (h[i] - c[i - 1]).abs();
        let cc = (l[i] - c[i - 1]).abs();
        true_range[i] = a.max(b).max(cc);
    }
    let atr = rolling_mean(&true_range, 14);
    let pdm_avg = rolling_mean(&plus_dm, 14);
    let mdm_avg = rolling_mean(&minus_dm, 14);
    let mut dx = vec![f64::NAN; n];
    for i in 0..n {
        if !atr[i].is_nan() && atr[i] > 0.0 && !pdm_avg[i].is_nan() && !mdm_avg[i].is_nan() {
            let pdi = 100.0 * pdm_avg[i] / atr[i];
            let mdi = 100.0 * mdm_avg[i] / atr[i];
            let dsum = pdi + mdi;
            if dsum > 0.0 { dx[i] = 100.0 * (pdi - mdi).abs() / dsum; }
        }
    }
    let adx = rolling_mean(&dx, 14);

    let mut inds = vec![Ind15m::default(); n];
    for i in 0..n {
        let valid = !sma20[i].is_nan() && !std20[i].is_nan() && !rsi[i].is_nan();
        let z = if !sma20[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            (c[i] - sma20[i]) / std20[i]
        } else { f64::NAN };

        inds[i] = Ind15m {
            sma20: sma20[i],
            sma9: sma9[i],
            std20: std20[i],
            z,
            bb_lo: if valid { sma20[i] - 2.0 * std20[i] } else { f64::NAN },
            bb_hi: if valid { sma20[i] + 2.0 * std20[i] } else { f64::NAN },
            bb_width: bb_width_raw[i],
            bb_width_avg: bb_width_avg[i],
            vol_ma: vol_ma[i],
            adr_lo: adr_lo[i],
            adr_hi: adr_hi[i],
            rsi: rsi[i],
            vwap: if !tp_v_sum[i].is_nan() && !v_sum[i].is_nan() && v_sum[i] > 0.0 {
                tp_v_sum[i] / v_sum[i]
            } else { f64::NAN },
            adx: adx[i],
            valid,
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
            if range > 0.0 {
                stoch_k[i] = 100.0 * (c[i] - lowest_low[i]) / range;
            }
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
            rsi: rsi[i],
            vol_ma: vol_ma[i],
            stoch_k: stoch_k[i],
            stoch_d: stoch_d[i],
            stoch_k_prev: if i > 0 { stoch_k[i - 1] } else { f64::NAN },
            stoch_d_prev: if i > 0 { stoch_d[i - 1] } else { f64::NAN },
            bb_upper: bb_upper[i],
            bb_lower: bb_lower[i],
            bb_width: bb_width_raw[i],
            bb_width_avg: bb_width_avg[i],
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

fn get_iso_short_strat(_coin: &str) -> Option<IsoShortStrat> {
    // Load from run6_1_results.json at runtime if needed
    // For now return None (OPTIMAL_ISO_SHORT_STRAT is empty in Python)
    None
}

fn long_entry(ind: &Ind15m, candle: &Candle, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if candle.c > ind.sma20 || ind.z > 0.5 { return false; }
    match strat {
        LongStrat::VwapReversion => {
            ind.z < -1.5 && candle.c < ind.sma20 && candle.v > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            candle.c <= ind.bb_lo * 1.02 && candle.v > ind.vol_ma * 1.3
        }
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
        ShortStrat::ShortVwapRev => {
            ind.z > 1.5 && candle.c > ind.sma20 && candle.v > ind.vol_ma * 1.2
        }
        ShortStrat::ShortBbBounce => {
            candle.c >= ind.bb_hi * 0.98 && candle.v > ind.vol_ma * 1.3
        }
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
        IsoShortStrat::IsoRelativeZ => {
            ctx.avg_z_valid && ind.z > ctx.avg_z + params.z_spread
        }
        IsoShortStrat::IsoRsiExtreme => {
            ctx.avg_rsi_valid && !ind.rsi.is_nan() &&
            ind.rsi > params.rsi_threshold && ctx.avg_rsi < 55.0
        }
        IsoShortStrat::IsoDivergence => {
            ctx.btc_z_valid && ind.z > params.z_threshold && ctx.btc_z < 0.0
        }
        IsoShortStrat::IsoMeanRev => ind.z > params.z_threshold,
        IsoShortStrat::IsoVwapRev => {
            ind.z > params.z_threshold && candle.c > ind.sma20 &&
            candle.v > ind.vol_ma * params.vol_mult
        }
        IsoShortStrat::IsoBbBounce => {
            candle.c >= ind.bb_hi * params.bb_margin &&
            candle.v > ind.vol_ma * (params.vol_mult + 0.1)
        }
        IsoShortStrat::IsoAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            range > 0.0 && candle.c >= ind.adr_hi - range * params.adr_pct &&
            candle.v > ind.vol_ma * params.vol_mult
        }
        IsoShortStrat::IsoVolSpike => {
            ind.z > 1.0 && candle.v > ind.vol_ma * params.vol_spike_mult
        }
        IsoShortStrat::IsoBbSqueeze => {
            !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 &&
            candle.c >= ind.bb_hi * 0.98 &&
            ind.bb_width < ind.bb_width_avg * params.squeeze_factor
        }
    }
}

fn scalp_entry(ind: &Ind1m, candle: &Candle, params: &ScalpParams) -> Option<(Position, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }

    let vol_r = candle.v / ind.vol_ma;
    let rsi_low = params.rsi_extreme;
    let rsi_high = 100.0 - params.rsi_extreme;

    // 1. scalp_vol_spike_rev
    if vol_r > params.vol_spike_mult {
        if ind.rsi < rsi_low { return Some((Position::Long, "scalp_vol_spike_rev")); }
        if ind.rsi > rsi_high { return Some((Position::Short, "scalp_vol_spike_rev")); }
    }

    // 2. scalp_stoch_cross
    if !ind.stoch_k.is_nan() && !ind.stoch_d.is_nan() &&
       !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan() {
        let stoch_lo = params.stoch_extreme;
        let stoch_hi = 100.0 - params.stoch_extreme;
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d &&
           ind.stoch_k < stoch_lo && ind.stoch_d < stoch_lo {
            return Some((Position::Long, "scalp_stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d &&
           ind.stoch_k > stoch_hi && ind.stoch_d > stoch_hi {
            return Some((Position::Short, "scalp_stoch_cross"));
        }
    }

    // 3. scalp_bb_squeeze_break
    if !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 && !ind.bb_upper.is_nan() {
        let squeeze = ind.bb_width < ind.bb_width_avg * params.bb_squeeze_factor;
        if squeeze && vol_r > 2.0 {
            if candle.c > ind.bb_upper { return Some((Position::Long, "scalp_bb_squeeze_break")); }
            if candle.c < ind.bb_lower { return Some((Position::Short, "scalp_bb_squeeze_break")); }
        }
    }

    None
}

struct CoinData {
    candles_15m: Vec<Candle>,
    candles_1m: Vec<Candle>,
    ind_15m: Vec<Ind15m>,
    ind_1m: Vec<Ind1m>,
    long_strat: LongStrat,
    short_strat: ShortStrat,
    iso_short_strat: Option<IsoShortStrat>,
    // For each 15m candle, the range [start, end) into candles_1m
    m1_windows: Vec<(usize, usize)>,
}

fn build_m1_windows(n_15m: usize, n_1m: usize) -> Vec<(usize, usize)> {
    // Each 15m candle maps to 15 consecutive 1m candles
    let mut windows = Vec::with_capacity(n_15m);
    for i in 0..n_15m {
        let start = i * 15;
        let end = ((i + 1) * 15).min(n_1m);
        if start >= n_1m {
            windows.push((n_1m, n_1m));
        } else {
            windows.push((start, end));
        }
    }
    windows
}

fn calc_stats(trades: &[Trade]) -> TradeStats {
    if trades.is_empty() {
        return TradeStats { pf: 0.0, wr: 0.0, trades: 0, wins: 0 };
    }
    let wins: Vec<&Trade> = trades.iter().filter(|t| t.pnl_pct > 0.0).collect();
    let tw: f64 = wins.iter().map(|t| t.pnl_pct).sum();
    let tl: f64 = trades.iter().filter(|t| t.pnl_pct <= 0.0).map(|t| t.pnl_pct).sum();
    let pf = if tl != 0.0 { (tw / tl).abs() } else if tw > 0.0 { 999.0 } else { 0.0 };
    TradeStats {
        pf,
        wr: wins.len() as f64 / trades.len() as f64 * 100.0,
        trades: trades.len(),
        wins: wins.len(),
    }
}

fn run_backtest(
    coin: &CoinData,
    breadth: &[f64],
    avg_z: &[f64],
    avg_rsi: &[f64],
    btc_z: &[f64],
    scalp_params: Option<&ScalpParams>,
    iso_params: &IsoShortParams,
) -> Option<BacktestResult> {
    let n = coin.candles_15m.len();
    // Find first valid index
    let first_valid = coin.ind_15m.iter().position(|x| x.valid)?;
    if n - first_valid < 50 { return None; }

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

    for i in first_valid..n {
        let candle = &coin.candles_15m[i];
        let ind = &coin.ind_15m[i];
        if !ind.valid { continue; }

        let price = candle.c;
        let b = if i < breadth.len() { breadth[i] } else { 0.0 };
        let market_mode = if b <= BREADTH_LONG_MAX { 0 }      // long
                         else if b >= BREADTH_SHORT_MIN { 2 }  // short
                         else { 1 };                           // iso_short

        // === SCALP EXIT ===
        if let (Some(pos), Some(TradeType::Scalp)) = (position, trade_type) {
            if let Some(sp) = scalp_params {
                let (ws, we) = coin.m1_windows[i];
                for j in ws..we {
                    if j >= coin.candles_1m.len() { break; }
                    let p = coin.candles_1m[j].c;
                    let pnl = match pos {
                        Position::Long => (p - entry_price) / entry_price,
                        Position::Short => (entry_price - p) / entry_price,
                    };
                    if pnl >= sp.scalp_tp {
                        balance += balance * SCALP_RISK * sp.scalp_tp * LEVERAGE;
                        let t = Trade {
                            pnl_pct: sp.scalp_tp * LEVERAGE * 100.0,
                            trade_type: TradeType::Scalp, dir: pos,
                            reason: "TP".into(), strat: None,
                        };
                        all_trades.push(t.clone());
                        scalp_trades.push(t);
                        position = None; trade_type = None; cooldown = 0;
                        break;
                    } else if pnl <= -sp.scalp_sl {
                        balance -= balance * SCALP_RISK * sp.scalp_sl * LEVERAGE;
                        let t = Trade {
                            pnl_pct: -sp.scalp_sl * LEVERAGE * 100.0,
                            trade_type: TradeType::Scalp, dir: pos,
                            reason: "SL".into(), strat: None,
                        };
                        all_trades.push(t.clone());
                        scalp_trades.push(t);
                        position = None; trade_type = None; cooldown = 0;
                        break;
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
                        let t = Trade { pnl_pct: -REGIME_SL * LEVERAGE * 100.0,
                            trade_type: TradeType::Regime, dir: Position::Long,
                            reason: "SL".into(), strat: None };
                        all_trades.push(t.clone()); regime_trades.push(t);
                        exited = true;
                    }
                    if !exited && price_pnl > 0.0 && candles_held >= MIN_HOLD {
                        if candle.c > ind.sma20 || ind.z > 0.5 {
                            balance += balance * REGIME_RISK * price_pnl * LEVERAGE;
                            let reason = if candle.c > ind.sma20 { "SMA" } else { "Z0" };
                            let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0,
                                trade_type: TradeType::Regime, dir: Position::Long,
                                reason: reason.into(), strat: None };
                            all_trades.push(t.clone()); regime_trades.push(t);
                            exited = true;
                        }
                    }
                }
                Position::Short => {
                    if candle.l < trough_price { trough_price = candle.l; }
                    let price_pnl = (entry_price - price) / entry_price;
                    if price_pnl <= -REGIME_SL {
                        balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE;
                        let t = Trade { pnl_pct: -REGIME_SL * LEVERAGE * 100.0,
                            trade_type: TradeType::Regime, dir: Position::Short,
                            reason: "SL".into(), strat: None };
                        all_trades.push(t.clone()); regime_trades.push(t);
                        exited = true;
                    }
                    if !exited && price_pnl > 0.0 && candles_held >= MIN_HOLD {
                        if price < ind.sma20 || ind.z < iso_params.exit_z {
                            balance += balance * REGIME_RISK * price_pnl * LEVERAGE;
                            let reason = if price < ind.sma20 { "SMA" } else { "Z0" };
                            let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0,
                                trade_type: TradeType::Regime, dir: Position::Short,
                                reason: reason.into(), strat: None };
                            all_trades.push(t.clone()); regime_trades.push(t);
                            exited = true;
                        }
                    }
                }
            }
            if exited {
                position = None; trade_type = None; cooldown = 2; candles_held = 0;
            }
        }

        // === COOLDOWN ===
        if cooldown > 0 { cooldown -= 1; }

        // === REGIME ENTRY ===
        if position.is_none() && cooldown == 0 {
            let ctx = MarketCtx {
                avg_z: if i < avg_z.len() { avg_z[i] } else { f64::NAN },
                avg_rsi: if i < avg_rsi.len() { avg_rsi[i] } else { f64::NAN },
                btc_z: if i < btc_z.len() { btc_z[i] } else { f64::NAN },
                avg_z_valid: i < avg_z.len() && !avg_z[i].is_nan(),
                avg_rsi_valid: i < avg_rsi.len() && !avg_rsi[i].is_nan(),
                btc_z_valid: i < btc_z.len() && !btc_z[i].is_nan(),
            };

            match market_mode {
                0 => { // long
                    if long_entry(ind, candle, coin.long_strat) {
                        position = Some(Position::Long);
                        trade_type = Some(TradeType::Regime);
                        entry_price = price; peak_price = candle.h; candles_held = 0;
                    } else if let Some(iso_s) = coin.iso_short_strat {
                        if iso_short_entry(ind, candle, iso_s, iso_params, &ctx) {
                            position = Some(Position::Short);
                            trade_type = Some(TradeType::Regime);
                            entry_price = price; trough_price = candle.l; candles_held = 0;
                        }
                    }
                }
                1 => { // iso_short
                    if let Some(iso_s) = coin.iso_short_strat {
                        if b <= ISO_SHORT_BREADTH_MAX || ISO_SHORT_BREADTH_MAX >= 0.50 {
                            if iso_short_entry(ind, candle, iso_s, iso_params, &ctx) {
                                position = Some(Position::Short);
                                trade_type = Some(TradeType::Regime);
                                entry_price = price; trough_price = candle.l; candles_held = 0;
                            }
                        }
                    }
                }
                2 => { // short
                    if short_entry(ind, candle, coin.short_strat) {
                        position = Some(Position::Short);
                        trade_type = Some(TradeType::Regime);
                        entry_price = price; trough_price = candle.l; candles_held = 0;
                    }
                }
                _ => {}
            }
        }

        // === SCALP ENTRY ===
        if let Some(sp) = scalp_params {
            if position.is_none() && cooldown == 0 {
                let (ws, we) = coin.m1_windows[i];
                let mut j = ws;
                while j < we && j < coin.candles_1m.len() {
                    if position.is_some() { break; }
                    let m_ind = &coin.ind_1m[j];
                    let m_candle = &coin.candles_1m[j];
                    if let Some((dir, strat_name)) = scalp_entry(m_ind, m_candle, sp) {
                        position = Some(dir);
                        trade_type = Some(TradeType::Scalp);
                        entry_price = m_candle.c;

                        // Check remaining 1m candles for TP/SL
                        for k in (j + 1)..we {
                            if k >= coin.candles_1m.len() { break; }
                            if position.is_none() { break; }
                            let p = coin.candles_1m[k].c;
                            let pnl = match dir {
                                Position::Long => (p - entry_price) / entry_price,
                                Position::Short => (entry_price - p) / entry_price,
                            };
                            if pnl >= sp.scalp_tp {
                                balance += balance * SCALP_RISK * sp.scalp_tp * LEVERAGE;
                                let t = Trade {
                                    pnl_pct: sp.scalp_tp * LEVERAGE * 100.0,
                                    trade_type: TradeType::Scalp, dir,
                                    reason: "TP".into(), strat: Some(strat_name.into()),
                                };
                                all_trades.push(t.clone()); scalp_trades.push(t);
                                position = None; trade_type = None;
                                break;
                            } else if pnl <= -sp.scalp_sl {
                                balance -= balance * SCALP_RISK * sp.scalp_sl * LEVERAGE;
                                let t = Trade {
                                    pnl_pct: -sp.scalp_sl * LEVERAGE * 100.0,
                                    trade_type: TradeType::Scalp, dir,
                                    reason: "SL".into(), strat: Some(strat_name.into()),
                                };
                                all_trades.push(t.clone()); scalp_trades.push(t);
                                position = None; trade_type = None;
                                break;
                            }
                        }
                    }
                    j += 1;
                }
            }
        }

        // Drawdown
        if balance > peak_balance { peak_balance = balance; }
        let dd = (peak_balance - balance) / peak_balance * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    // Close open position at end
    if let Some(pos) = position {
        let last_price = coin.candles_15m.last()?.c;
        let price_pnl = match pos {
            Position::Long => (last_price - entry_price) / entry_price,
            Position::Short => (entry_price - last_price) / entry_price,
        };
        match trade_type {
            Some(TradeType::Regime) => {
                balance += balance * REGIME_RISK * price_pnl * LEVERAGE;
                let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0,
                    trade_type: TradeType::Regime, dir: pos,
                    reason: "END".into(), strat: None };
                all_trades.push(t.clone()); regime_trades.push(t);
            }
            Some(TradeType::Scalp) => {
                balance += balance * SCALP_RISK * price_pnl * LEVERAGE;
                let t = Trade { pnl_pct: price_pnl * LEVERAGE * 100.0,
                    trade_type: TradeType::Scalp, dir: pos,
                    reason: "END".into(), strat: None };
                all_trades.push(t.clone()); scalp_trades.push(t);
            }
            None => {}
        }
    }

    if all_trades.is_empty() { return None; }

    // Scalp by strategy
    let mut scalp_by_strat: HashMap<String, Vec<&Trade>> = HashMap::new();
    for t in &scalp_trades {
        let s = t.strat.as_deref().unwrap_or("unknown").to_string();
        scalp_by_strat.entry(s).or_default().push(t);
    }

    Some(BacktestResult {
        balance,
        pnl: (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100.0,
        max_dd,
        all: calc_stats(&all_trades),
        regime: calc_stats(&regime_trades),
        scalp: calc_stats(&scalp_trades),
        scalp_by_strat: scalp_by_strat.iter().map(|(k, v)| {
            let owned: Vec<Trade> = v.iter().map(|t| (*t).clone()).collect();
            (k.clone(), calc_stats(&owned))
        }).collect(),
    })
}

fn load_csv(path: &str) -> Option<Vec<Candle>> {
    let content = fs::read_to_string(path).ok()?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(content.as_bytes());
    let headers = rdr.headers().ok()?.clone();

    // Find column indices - support both 'open/high/low/close/volume' and 'o/h/l/c/v'
    let find_col = |names: &[&str]| -> Option<usize> {
        for name in names {
            if let Some(pos) = headers.iter().position(|h| h == *name) {
                return Some(pos);
            }
        }
        None
    };

    let o_idx = find_col(&["open", "o"])?;
    let h_idx = find_col(&["high", "h"])?;
    let l_idx = find_col(&["low", "l"])?;
    let c_idx = find_col(&["close", "c"])?;
    let v_idx = find_col(&["volume", "v"])?;

    let mut candles = Vec::new();
    for result in rdr.records() {
        let record = result.ok()?;
        let o: f64 = record.get(o_idx)?.parse().ok()?;
        let h: f64 = record.get(h_idx)?.parse().ok()?;
        let l: f64 = record.get(l_idx)?.parse().ok()?;
        let c: f64 = record.get(c_idx)?.parse().ok()?;
        let v: f64 = record.get(v_idx)?.parse().ok()?;
        candles.push(Candle { o, h, l, c, v });
    }
    Some(candles)
}

fn build_combos() -> Vec<ScalpParams> {
    let mut combos = Vec::new();
    for &sl in SCALP_SLS {
        for &tp in SCALP_TPS {
            for &vm in VOL_SPIKE_MULTS {
                for &rsi in RSI_EXTREMES {
                    for &st in STOCH_EXTREMES {
                        for &bb in BB_SQUEEZE_FACTORS {
                            combos.push(ScalpParams {
                                scalp_sl: sl, scalp_tp: tp,
                                vol_spike_mult: vm, rsi_extreme: rsi,
                                stoch_extreme: st, bb_squeeze_factor: bb,
                            });
                        }
                    }
                }
            }
        }
    }
    combos
}

fn combo_key(p: &ScalpParams) -> String {
    format!("sl{}_tp{}_vm{}_rsi{}_st{}_bb{}",
        p.scalp_sl, p.scalp_tp, p.vol_spike_mult,
        p.rsi_extreme, p.stoch_extreme, p.bb_squeeze_factor)
}

fn main() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let s = shutdown.clone();
    ctrlc::set_handler(move || {
        eprintln!("\nSIGINT received, will save after current combo...");
        s.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("{}", "=".repeat(90));
    println!("RUN9.1 - SCALP STRATEGY GRID SEARCH (Rust + Rayon)");
    println!("{}", "=".repeat(90));

    let combos = build_combos();
    println!("Total: {} combos x {} coins", combos.len(), COINS.len());

    // Load data
    println!("\nLoading data...");
    let mut all_coins: HashMap<String, CoinData> = HashMap::new();
    let mut all_15m_candles: HashMap<String, Vec<Candle>> = HashMap::new();

    for &coin in COINS {
        let path_15m = format!("{}/{}_USDT_15m_5months.csv", DATA_CACHE_DIR, coin);
        let path_1m = format!("{}/{}_USDT_1m_5months.csv", DATA_CACHE_DIR, coin);

        let candles_15m = match load_csv(&path_15m) {
            Some(c) => c,
            None => { eprintln!("  Missing 15m data for {}", coin); continue; }
        };
        let candles_1m = match load_csv(&path_1m) {
            Some(c) => c,
            None => { eprintln!("  Missing 1m data for {}", coin); continue; }
        };

        println!("  {} - 15m: {} candles, 1m: {} candles", coin, candles_15m.len(), candles_1m.len());

        let ind_15m = compute_15m_indicators(&candles_15m);
        let ind_1m = compute_1m_indicators(&candles_1m);
        let m1_windows = build_m1_windows(candles_15m.len(), candles_1m.len());

        all_15m_candles.insert(coin.to_string(), candles_15m.clone());
        all_coins.insert(coin.to_string(), CoinData {
            candles_15m,
            candles_1m,
            ind_15m,
            ind_1m,
            long_strat: get_long_strat(coin),
            short_strat: get_short_strat(coin),
            iso_short_strat: get_iso_short_strat(coin),
            m1_windows,
        });
    }

    if all_coins.is_empty() {
        eprintln!("ERROR: No data loaded!");
        std::process::exit(1);
    }
    println!("Loaded {} coins", all_coins.len());

    // Build market breadth from 15m z-scores
    println!("\nComputing market breadth...");
    let n_15m = all_15m_candles.values().map(|c| c.len()).max().unwrap_or(0);
    let mut z_all: HashMap<String, Vec<f64>> = HashMap::new();
    let mut rsi_all: HashMap<String, Vec<f64>> = HashMap::new();
    for (coin, cd) in &all_coins {
        let zs: Vec<f64> = cd.ind_15m.iter().map(|x| x.z).collect();
        let rs: Vec<f64> = cd.ind_15m.iter().map(|x| x.rsi).collect();
        z_all.insert(coin.clone(), zs);
        rsi_all.insert(coin.clone(), rs);
    }

    let mut breadth = vec![0.0f64; n_15m];
    let mut avg_z_vec = vec![f64::NAN; n_15m];
    let mut avg_rsi_vec = vec![f64::NAN; n_15m];
    for i in 0..n_15m {
        let mut below = 0usize;
        let mut total = 0usize;
        let mut zsum = 0.0;
        let mut rsum = 0.0;
        let mut rcount = 0usize;
        for zs in z_all.values() {
            if i < zs.len() && !zs[i].is_nan() {
                if zs[i] < -1.0 { below += 1; }
                zsum += zs[i];
                total += 1;
            }
        }
        for rs in rsi_all.values() {
            if i < rs.len() && !rs[i].is_nan() {
                rsum += rs[i];
                rcount += 1;
            }
        }
        if total > 0 {
            breadth[i] = below as f64 / total as f64;
            avg_z_vec[i] = zsum / total as f64;
        }
        if rcount > 0 { avg_rsi_vec[i] = rsum / rcount as f64; }
    }
    let btc_z_vec: Vec<f64> = z_all.get("BTC").cloned().unwrap_or_default();

    let iso_params = IsoShortParams::default();

    // === BASELINE ===
    println!("\nRunning baseline (regime-only) backtests...");
    let mut baseline: HashMap<String, BacktestResult> = HashMap::new();
    for &coin in COINS {
        if let Some(cd) = all_coins.get(coin) {
            if let Some(r) = run_backtest(cd, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec,
                                          None, &iso_params) {
                println!("  {}: PF={:.2} WR={:.0}% P&L={:+.1}% trades={}",
                    coin, r.regime.pf, r.regime.wr, r.pnl, r.regime.trades);
                baseline.insert(coin.to_string(), r);
            }
        }
    }

    // === SIGNAL FREQUENCY CHECK ===
    println!("\nChecking scalp signal frequency (mid-range params)...");
    let mid_params = ScalpParams {
        scalp_sl: 0.0015, scalp_tp: 0.003,
        vol_spike_mult: 3.0, rsi_extreme: 20.0,
        stoch_extreme: 10.0, bb_squeeze_factor: 0.5,
    };
    for &coin in COINS {
        if let Some(cd) = all_coins.get(coin) {
            if let Some(r) = run_backtest(cd, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec,
                                          Some(&mid_params), &iso_params) {
                let flag = if r.scalp.trades < 20 { " *** LOW" } else { "" };
                let by_strat: String = r.scalp_by_strat.iter()
                    .map(|(s, st)| format!("{}={}", s, st.trades))
                    .collect::<Vec<_>>().join(", ");
                println!("  {}: {} scalp signals ({}){}", coin, r.scalp.trades, by_strat, flag);
            }
        }
    }

    // === GRID SEARCH (parallel) ===
    println!("\nStarting grid search: {} combos (parallel with rayon)", combos.len());
    let start = Instant::now();
    let counter = AtomicUsize::new(0);
    let total = combos.len();

    // Share read-only data
    let all_coins = Arc::new(all_coins);
    let breadth = Arc::new(breadth);
    let avg_z_vec = Arc::new(avg_z_vec);
    let avg_rsi_vec = Arc::new(avg_rsi_vec);
    let btc_z_vec = Arc::new(btc_z_vec);
    let baseline = Arc::new(baseline);

    let results: Vec<(String, ScalpParams, HashMap<String, BacktestResult>, f64)> = combos.par_iter()
        .filter_map(|params| {
            if shutdown.load(Ordering::SeqCst) { return None; }

            let mut coin_results: HashMap<String, BacktestResult> = HashMap::new();
            for &coin in COINS {
                if let Some(cd) = all_coins.get(coin) {
                    if let Some(r) = run_backtest(cd, &breadth, &avg_z_vec, &avg_rsi_vec, &btc_z_vec,
                                                  Some(params), &iso_params) {
                        coin_results.insert(coin.to_string(), r);
                    }
                }
            }

            // Calculate avg net impact
            let mut impact_sum = 0.0;
            let mut impact_count = 0;
            for (coin, r) in &coin_results {
                let base_pnl = baseline.get(coin).map(|b| b.pnl).unwrap_or(0.0);
                impact_sum += r.pnl - base_pnl;
                impact_count += 1;
            }
            let avg_impact = if impact_count > 0 { impact_sum / impact_count as f64 } else { 0.0 };

            let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
            if done % 50 == 0 || done == total {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = done as f64 / elapsed;
                let eta = (total - done) as f64 / rate / 60.0;
                eprintln!("  [{}/{}] ({:.0}%) avg_impact={:+.1}% | {:.1}/s ETA:{:.1}m",
                    done, total, done as f64 / total as f64 * 100.0,
                    avg_impact, rate, eta);
            }

            let key = combo_key(params);
            Some((key, *params, coin_results, avg_impact))
        })
        .collect();

    // === ANALYSIS ===
    println!("\n{}", "=".repeat(90));
    println!("RESULTS SUMMARY");
    println!("{}", "=".repeat(90));

    let base_pnls: Vec<f64> = baseline.values().map(|r| r.pnl).collect();
    let base_pfs: Vec<f64> = baseline.values().filter(|r| r.regime.trades > 0).map(|r| r.regime.pf).collect();
    let base_wrs: Vec<f64> = baseline.values().filter(|r| r.regime.trades > 0).map(|r| r.regime.wr).collect();
    if !base_pnls.is_empty() {
        let avg_pnl: f64 = base_pnls.iter().sum::<f64>() / base_pnls.len() as f64;
        let avg_pf: f64 = base_pfs.iter().sum::<f64>() / base_pfs.len().max(1) as f64;
        let avg_wr: f64 = base_wrs.iter().sum::<f64>() / base_wrs.len().max(1) as f64;
        println!("\n  BASELINE (regime-only):");
        println!("    Avg WR: {:.1}%  Avg PF: {:.2}  Avg P&L: {:+.1}%", avg_wr, avg_pf, avg_pnl);
    }

    let mut positive_count = 0;
    let mut best_score: f64 = -999.0;
    let mut best_idx: Option<usize> = None;

    for (i, (_, _, coin_results, avg_impact)) in results.iter().enumerate() {
        if *avg_impact > 0.0 { positive_count += 1; }

        let pfs: Vec<f64> = coin_results.values()
            .filter(|r| r.all.trades > 0).map(|r| r.all.pf).collect();
        let wrs: Vec<f64> = coin_results.values()
            .filter(|r| r.all.trades > 0).map(|r| r.all.wr).collect();
        if pfs.is_empty() { continue; }

        let avg_pf = pfs.iter().sum::<f64>() / pfs.len() as f64;
        let avg_wr = wrs.iter().sum::<f64>() / wrs.len() as f64;
        let score = avg_pf * (avg_wr / 100.0).sqrt();
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }

    println!("\n  Combos with positive net impact: {}/{} ({:.0}%)",
        positive_count, results.len(),
        if results.is_empty() { 0.0 } else { positive_count as f64 / results.len() as f64 * 100.0 });

    if let Some(bi) = best_idx {
        let (_, params, coin_results, _) = &results[bi];
        let mut impacts = Vec::new();
        let mut pnls = Vec::new();
        let mut scalp_counts = Vec::new();
        for (coin, r) in coin_results {
            let base_pnl = baseline.get(coin).map(|b| b.pnl).unwrap_or(0.0);
            impacts.push(r.pnl - base_pnl);
            pnls.push(r.pnl);
            scalp_counts.push(r.scalp.trades as f64);
        }
        let pfs: Vec<f64> = coin_results.values().filter(|r| r.all.trades > 0).map(|r| r.all.pf).collect();
        let wrs: Vec<f64> = coin_results.values().filter(|r| r.all.trades > 0).map(|r| r.all.wr).collect();

        let avg = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };

        println!("\n  BEST COMBO:");
        println!("    Params: sl={} tp={} vm={} rsi={} st={} bb={}",
            params.scalp_sl, params.scalp_tp, params.vol_spike_mult,
            params.rsi_extreme, params.stoch_extreme, params.bb_squeeze_factor);
        println!("    Avg WR: {:.1}%  Avg PF: {:.2}  Avg P&L: {:+.1}%", avg(&wrs), avg(&pfs), avg(&pnls));
        println!("    Avg net impact: {:+.1}%", avg(&impacts));
        println!("    Avg scalp trades: {:.0}", avg(&scalp_counts));

        println!("\n  PER-COIN BREAKDOWN (best combo):");
        println!("    {:<8} {:<12} {:<12} {:<10} {:<8} {:<10} Scalp PF", "Coin", "Base P&L", "With Scalp", "Impact", "Scalp#", "Scalp WR");
        println!("    {}", "-".repeat(75));
        let mut better = 0;
        let mut worse = 0;
        for &coin in COINS {
            if let Some(r) = coin_results.get(coin) {
                let base_pnl = baseline.get(coin).map(|b| b.pnl).unwrap_or(0.0);
                let impact = r.pnl - base_pnl;
                if impact > 0.0 { better += 1; } else { worse += 1; }
                println!("    {:<8} {:<12.1} {:<12.1} {:+<10.1} {:<8} {:<10.1} {:.2}",
                    coin, base_pnl, r.pnl, impact, r.scalp.trades, r.scalp.wr, r.scalp.pf);
            }
        }
        println!("\n    Better: {}  Worse: {}", better, worse);

        if positive_count < results.len() * 3 / 10 {
            println!("\n  *** EARLY STOP SIGNAL: Only {}/{} combos positive. Scalping may not add value. ***",
                positive_count, results.len());
        }
    }

    // Save results
    let mut best_per_coin: HashMap<String, serde_json::Value> = HashMap::new();
    for &coin in COINS {
        let mut best_coin_score: f64 = -999.0;
        let mut best_coin_data: Option<serde_json::Value> = None;
        for (_, params, coin_results, _) in &results {
            if let Some(r) = coin_results.get(coin) {
                if r.all.trades < 3 { continue; }
                let score = r.all.pf * (r.all.wr / 100.0).sqrt();
                if score > best_coin_score {
                    best_coin_score = score;
                    let base_pnl = baseline.get(coin).map(|b| b.pnl).unwrap_or(0.0);
                    best_coin_data = Some(serde_json::json!({
                        "params": params,
                        "pnl": r.pnl,
                        "pf": r.all.pf,
                        "wr": r.all.wr,
                        "scalp_trades": r.scalp.trades,
                        "net_impact": r.pnl - base_pnl,
                    }));
                }
            }
        }
        if let Some(d) = best_coin_data {
            best_per_coin.insert(coin.to_string(), d);
        }
    }

    let best_params = best_idx.map(|i| &results[i].1);
    let save_data = serde_json::json!({
        "best_combo_key": best_idx.map(|i| &results[i].0),
        "best_params": best_params,
        "baseline": baseline.iter().map(|(c, r)| {
            (c.clone(), serde_json::json!({"pnl": r.pnl, "regime": r.regime}))
        }).collect::<HashMap<_,_>>(),
        "positive_impact_combos": positive_count,
        "total_combos": results.len(),
        "best_per_coin": best_per_coin,
    });

    fs::write(RESULTS_FILE, serde_json::to_string_pretty(&save_data).unwrap())
        .expect("Failed to write results");
    println!("\nResults saved to {}", RESULTS_FILE);

    let elapsed = start.elapsed();
    println!("Total time: {:.1}s ({:.1} combos/s)",
        elapsed.as_secs_f64(),
        results.len() as f64 / elapsed.as_secs_f64());
}
