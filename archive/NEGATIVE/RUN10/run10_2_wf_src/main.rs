//! RUN10.2 — Walk-Forward Validation of Scalp Indicator Filters (Rust)
//!
//! Validates the indicator filters discovered in RUN10.1 using 3-window
//! walk-forward testing (2mo train → 1mo test).
//!
//! Filters under test:
//!   F1: dir_roc_3 < threshold  (counter-momentum)
//!   F2: avg_body_3 > threshold (active candles)
//!   F3: spread_pct > threshold (volatility)
//!   F4: 15m_z alignment       (HTF confirmation)
//!   F5: F1 + F2 combined
//!
//! Uses rayon for parallel processing across coins.

use chrono::NaiveDateTime;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

// ── Constants ──────────────────────────────────────────────────────────

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const RESULTS_FILE: &str = "/home/scamarena/ProjectCoin/run10_2_results.json";

const COINS: &[&str] = &[
    "DASH", "UNI", "NEAR", "ADA", "LTC", "SHIB", "LINK", "ETH",
    "DOT", "XRP", "ATOM", "SOL", "DOGE", "XLM", "AVAX", "ALGO", "BNB", "BTC",
];

// RUN9 best universal scalp params
const SCALP_SL: f64 = 0.0010;
const SCALP_TP: f64 = 0.0020;
const VOL_SPIKE_MULT: f64 = 3.5;
const RSI_EXTREME: f64 = 20.0;
const STOCH_EXTREME: f64 = 5.0;
const BB_SQUEEZE_FACTOR: f64 = 0.4;
const LEVERAGE: f64 = 5.0;
const MAX_HOLD_BARS: usize = 60;

// Walk-forward windows (YYYY-MM-DD boundaries)
struct WfWindow {
    name: &'static str,
    train_start: &'static str,
    train_end: &'static str,
    test_start: &'static str,
    test_end: &'static str,
}

const WINDOWS: &[WfWindow] = &[
    WfWindow {
        name: "W1",
        train_start: "2025-10-15",
        train_end: "2025-12-14",
        test_start: "2025-12-15",
        test_end: "2026-01-14",
    },
    WfWindow {
        name: "W2",
        train_start: "2025-11-15",
        train_end: "2026-01-14",
        test_start: "2026-01-15",
        test_end: "2026-02-14",
    },
    WfWindow {
        name: "W3",
        train_start: "2025-12-15",
        train_end: "2026-02-14",
        test_start: "2026-02-15",
        test_end: "2026-03-10",
    },
];

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

/// All 1m indicators pre-computed per candle
#[derive(Debug, Clone, Default)]
struct Ind1m {
    rsi: f64,
    rsi_slope: f64,        // rsi - rsi[3]
    stoch_k: f64,
    stoch_d: f64,
    stoch_k_prev: f64,
    stoch_d_prev: f64,
    bb_upper: f64,
    bb_lower: f64,
    bb_sma: f64,
    bb_width: f64,
    bb_width_avg: f64,
    bb_pctb: f64,          // (c - bb_lower) / (bb_upper - bb_lower)
    vol_ma: f64,
    vol_ratio: f64,
    vol_trend: f64,         // vol_ma5 / vol_ma20
    roc_3: f64,
    roc_5: f64,
    atr_ratio: f64,         // atr / close * 100
    body_ratio: f64,        // |c-o| / (h-l)
    upper_wick: f64,
    lower_wick: f64,
    is_green: bool,
    avg_body_3: f64,        // avg |c-o|/c * 100 over 3 bars
    spread_pct: f64,        // (h-l)/c * 100
    sma9: f64,
    sma20: f64,
    dist_sma9: f64,
    dist_sma20: f64,
    z_1m: f64,
    ema5: f64,
    ema12: f64,
    ema_spread: f64,        // (ema5 - ema12) / c * 100
    mfi: f64,
    vol_delta_proxy: f64,   // ((c-l)/(h-l))*2 - 1
    valid: bool,
}

/// 15m indicators (context)
#[derive(Debug, Clone, Default)]
struct Ind15m {
    z: f64,
    rsi: f64,
    adx: f64,
    sma20_slope: f64,
    dist_vwap: f64,
    valid: bool,
}

#[derive(Debug, Clone)]
struct Trade {
    direction: Dir,
    strategy: &'static str,
    outcome: Outcome,
    pnl_pct: f64,
    // Indicator snapshot at entry
    snap: Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir { Long, Short }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Outcome { Win, Loss }

/// Snapshot of indicators at scalp entry
#[derive(Debug, Clone, Default)]
struct Snapshot {
    dir_roc_3: f64,
    avg_body_3: f64,
    spread_pct: f64,
    dir_ema_spread: f64,
    atr_ratio: f64,
    vol_ratio: f64,
    vol_trend: f64,
    body_ratio: f64,
    z_15m: f64,
    rsi_15m: f64,
    adx_15m: f64,
    sma20_slope_15m: f64,
    dist_vwap_15m: f64,
    breadth: f64,
    mfi: f64,
    vol_delta_proxy: f64,
    bb_pctb: f64,
    rsi_slope: f64,
}

#[derive(Debug, Clone, Serialize)]
struct FilterResult {
    name: String,
    train_wr_base: f64,
    train_wr_filtered: f64,
    train_improvement: f64,
    train_kept_pct: f64,
    test_wr_base: f64,
    test_wr_filtered: f64,
    test_improvement: f64,
    test_kept_pct: f64,
    degradation_pct: f64,  // (train_imp - test_imp) / train_imp * 100
    oos_holds: bool,       // test_improvement > 0
}

#[derive(Debug, Clone, Serialize)]
struct WindowResult {
    window: String,
    train_trades: usize,
    test_trades: usize,
    train_wr: f64,
    test_wr: f64,
    filters: Vec<FilterResult>,
}

#[derive(Debug, Clone, Serialize)]
struct CoinWindowResult {
    coin: String,
    windows: Vec<WindowResult>,
}

#[derive(Debug, Serialize)]
struct FinalResults {
    summary: Summary,
    per_coin: Vec<CoinWindowResult>,
    aggregated_filters: Vec<AggFilter>,
}

#[derive(Debug, Serialize)]
struct Summary {
    total_coins: usize,
    windows: usize,
    total_train_trades: usize,
    total_test_trades: usize,
    avg_train_wr: f64,
    avg_test_wr: f64,
}

#[derive(Debug, Serialize)]
struct AggFilter {
    name: String,
    avg_train_improvement: f64,
    avg_test_improvement: f64,
    avg_degradation: f64,
    oos_positive_pct: f64,  // % of windows where test improvement > 0
    avg_test_kept_pct: f64,
    recommendation: String,
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
        let parse = |i: usize| -> f64 { result.get(i).unwrap_or("0").parse().unwrap_or(0.0) };
        let c = Candle {
            ts,
            o: parse(1),
            h: parse(2),
            l: parse(3),
            c: parse(4),
            v: parse(5),
        };
        if c.c > 0.0 && c.v >= 0.0 {
            candles.push(c);
        }
    }
    candles
}

fn slice_candles(candles: &[Candle], start: &str, end: &str) -> Vec<Candle> {
    let s = NaiveDateTime::parse_from_str(&format!("{} 00:00:00", start), "%Y-%m-%d %H:%M:%S").unwrap();
    let e = NaiveDateTime::parse_from_str(&format!("{} 23:59:59", end), "%Y-%m-%d %H:%M:%S").unwrap();
    candles.iter().filter(|c| c.ts >= s && c.ts <= e).cloned().collect()
}

// ── Indicator Calculation ──────────────────────────────────────────────

fn rolling_mean(vals: &[f64], period: usize, idx: usize) -> f64 {
    if idx + 1 < period { return f64::NAN; }
    let start = idx + 1 - period;
    vals[start..=idx].iter().sum::<f64>() / period as f64
}

fn rolling_std(vals: &[f64], period: usize, idx: usize) -> f64 {
    if idx + 1 < period { return f64::NAN; }
    let start = idx + 1 - period;
    let mean = vals[start..=idx].iter().sum::<f64>() / period as f64;
    let var = vals[start..=idx].iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
    var.sqrt()
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
    if avg_loss > 0.0 {
        rsi[period] = 100.0 - 100.0 / (1.0 + avg_gain / avg_loss);
    } else {
        rsi[period] = 100.0;
    }

    for i in (period + 1)..n {
        let d = closes[i] - closes[i - 1];
        let (g, l) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
        if avg_loss > 0.0 {
            rsi[i] = 100.0 - 100.0 / (1.0 + avg_gain / avg_loss);
        } else {
            rsi[i] = 100.0;
        }
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

    // Pre-compute rolling arrays
    let mut vol_ma5 = vec![f64::NAN; n];
    let mut vol_ma20 = vec![f64::NAN; n];
    let mut sma9 = vec![f64::NAN; n];
    let mut sma20 = vec![f64::NAN; n];
    let mut bb_std = vec![f64::NAN; n];

    for i in 0..n {
        vol_ma5[i] = rolling_mean(&volumes, 5, i);
        vol_ma20[i] = rolling_mean(&volumes, 20, i);
        sma9[i] = rolling_mean(&closes, 9, i);
        sma20[i] = rolling_mean(&closes, 20, i);
        bb_std[i] = rolling_std(&closes, 20, i);
    }

    // Stochastic
    let mut stoch_k = vec![f64::NAN; n];
    for i in 13..n {
        let lo = lows[i - 13..=i].iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = highs[i - 13..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = hi - lo;
        if range > 0.0 {
            stoch_k[i] = (closes[i] - lo) / range * 100.0;
        }
    }
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if stoch_k[i].is_finite() && stoch_k[i - 1].is_finite() && stoch_k[i - 2].is_finite() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i - 1] + stoch_k[i - 2]) / 3.0;
        }
    }

    // BB width avg (rolling 20 of bb_width)
    let mut bb_width_arr = vec![f64::NAN; n];
    for i in 0..n {
        if sma20[i].is_finite() && bb_std[i].is_finite() {
            let upper = sma20[i] + 2.0 * bb_std[i];
            let lower = sma20[i] - 2.0 * bb_std[i];
            bb_width_arr[i] = upper - lower;
        }
    }
    let mut bb_width_avg = vec![f64::NAN; n];
    for i in 19..n {
        let mut sum = 0.0;
        let mut cnt = 0;
        for j in (i - 19)..=i {
            if bb_width_arr[j].is_finite() {
                sum += bb_width_arr[j];
                cnt += 1;
            }
        }
        if cnt >= 10 { bb_width_avg[i] = sum / cnt as f64; }
    }

    // ATR
    let mut atr = vec![f64::NAN; n];
    {
        let mut tr_sum = 0.0;
        for i in 1..n {
            let tr = (highs[i] - lows[i])
                .max((highs[i] - closes[i - 1]).abs())
                .max((lows[i] - closes[i - 1]).abs());
            if i < 14 {
                tr_sum += tr;
            } else if i == 14 {
                tr_sum += tr;
                atr[i] = tr_sum / 14.0;
            } else if atr[i - 1].is_finite() {
                atr[i] = (atr[i - 1] * 13.0 + tr) / 14.0;
            }
        }
    }

    // MFI
    let mut mfi_arr = vec![f64::NAN; n];
    {
        let typical: Vec<f64> = candles.iter().map(|c| (c.h + c.l + c.c) / 3.0).collect();
        for i in 14..n {
            let mut pos = 0.0;
            let mut neg = 0.0;
            for j in (i - 13)..=i {
                if j == 0 { continue; }
                let mf = typical[j] * volumes[j];
                if typical[j] > typical[j - 1] {
                    pos += mf;
                } else {
                    neg += mf;
                }
            }
            if neg > 0.0 {
                mfi_arr[i] = 100.0 - 100.0 / (1.0 + pos / neg);
            } else {
                mfi_arr[i] = 100.0;
            }
        }
    }

    // Body info for avg_body_3
    let body_pct: Vec<f64> = candles.iter().map(|c| (c.c - c.o).abs() / c.c * 100.0).collect();

    for i in 0..n {
        let c = &candles[i];
        let ind = &mut inds[i];
        ind.rsi = rsi[i];
        ind.rsi_slope = if i >= 3 && rsi[i].is_finite() && rsi[i - 3].is_finite() {
            rsi[i] - rsi[i - 3]
        } else {
            f64::NAN
        };
        ind.stoch_k = stoch_k[i];
        ind.stoch_d = stoch_d[i];
        ind.stoch_k_prev = if i >= 1 { stoch_k[i - 1] } else { f64::NAN };
        ind.stoch_d_prev = if i >= 1 { stoch_d[i - 1] } else { f64::NAN };
        ind.bb_sma = sma20[i];
        let std_val = bb_std[i];
        ind.bb_upper = if sma20[i].is_finite() && std_val.is_finite() { sma20[i] + 2.0 * std_val } else { f64::NAN };
        ind.bb_lower = if sma20[i].is_finite() && std_val.is_finite() { sma20[i] - 2.0 * std_val } else { f64::NAN };
        ind.bb_width = bb_width_arr[i];
        ind.bb_width_avg = bb_width_avg[i];

        let bb_range = ind.bb_upper - ind.bb_lower;
        ind.bb_pctb = if bb_range > 0.0 && bb_range.is_finite() {
            (c.c - ind.bb_lower) / bb_range
        } else {
            f64::NAN
        };

        ind.vol_ma = vol_ma20[i];
        ind.vol_ratio = if vol_ma20[i].is_finite() && vol_ma20[i] > 0.0 {
            c.v / vol_ma20[i]
        } else {
            f64::NAN
        };
        ind.vol_trend = if vol_ma20[i].is_finite() && vol_ma20[i] > 0.0 && vol_ma5[i].is_finite() {
            vol_ma5[i] / vol_ma20[i]
        } else {
            f64::NAN
        };

        ind.roc_3 = if i >= 3 && closes[i - 3] > 0.0 {
            (c.c - closes[i - 3]) / closes[i - 3] * 100.0
        } else {
            f64::NAN
        };
        ind.roc_5 = if i >= 5 && closes[i - 5] > 0.0 {
            (c.c - closes[i - 5]) / closes[i - 5] * 100.0
        } else {
            f64::NAN
        };
        ind.atr_ratio = if atr[i].is_finite() && c.c > 0.0 {
            atr[i] / c.c * 100.0
        } else {
            f64::NAN
        };

        let full_range = c.h - c.l;
        let body = (c.c - c.o).abs();
        ind.body_ratio = if full_range > 0.0 { body / full_range } else { 0.0 };
        ind.upper_wick = if full_range > 0.0 {
            (c.h - c.c.max(c.o)) / full_range
        } else {
            0.0
        };
        ind.lower_wick = if full_range > 0.0 {
            (c.c.min(c.o) - c.l) / full_range
        } else {
            0.0
        };
        ind.is_green = c.c > c.o;
        ind.spread_pct = if c.c > 0.0 { full_range / c.c * 100.0 } else { 0.0 };

        ind.avg_body_3 = if i >= 2 {
            (body_pct[i] + body_pct[i - 1] + body_pct[i - 2]) / 3.0
        } else {
            f64::NAN
        };

        ind.sma9 = sma9[i];
        ind.sma20 = sma20[i];
        ind.dist_sma9 = if sma9[i].is_finite() && sma9[i] > 0.0 {
            (c.c - sma9[i]) / sma9[i] * 100.0
        } else {
            f64::NAN
        };
        ind.dist_sma20 = if sma20[i].is_finite() && sma20[i] > 0.0 {
            (c.c - sma20[i]) / sma20[i] * 100.0
        } else {
            f64::NAN
        };
        ind.z_1m = if sma20[i].is_finite() && std_val.is_finite() && std_val > 0.0 {
            (c.c - sma20[i]) / std_val
        } else {
            f64::NAN
        };
        ind.ema5 = ema5[i];
        ind.ema12 = ema12[i];
        ind.ema_spread = if ema5[i].is_finite() && ema12[i].is_finite() && c.c > 0.0 {
            (ema5[i] - ema12[i]) / c.c * 100.0
        } else {
            f64::NAN
        };
        ind.mfi = mfi_arr[i];
        ind.vol_delta_proxy = if full_range > 0.0 {
            ((c.c - c.l) / full_range) * 2.0 - 1.0
        } else {
            0.0
        };
        ind.valid = ind.rsi.is_finite() && ind.vol_ma.is_finite() && ind.vol_ma > 0.0;
    }
    inds
}

fn compute_15m_indicators(candles: &[Candle]) -> Vec<(NaiveDateTime, Ind15m)> {
    let n = candles.len();
    if n < 30 { return Vec::new(); }

    let closes: Vec<f64> = candles.iter().map(|c| c.c).collect();
    let highs: Vec<f64> = candles.iter().map(|c| c.h).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.l).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.v).collect();
    let rsi = compute_rsi(&closes, 14);

    let mut result = Vec::with_capacity(n);

    for i in 0..n {
        let sma20 = rolling_mean(&closes, 20, i);
        let std20 = rolling_std(&closes, 20, i);
        let z = if sma20.is_finite() && std20.is_finite() && std20 > 0.0 {
            (closes[i] - sma20) / std20
        } else {
            f64::NAN
        };

        let sma20_slope = if i >= 3 {
            let prev = rolling_mean(&closes, 20, i - 3);
            if prev.is_finite() && sma20.is_finite() && prev > 0.0 {
                (sma20 - prev) / prev * 100.0
            } else {
                f64::NAN
            }
        } else {
            f64::NAN
        };

        // Simplified ADX
        let adx = if i >= 28 {
            // Use ATR-based approximation
            let mut sum_dx = 0.0;
            let mut cnt = 0;
            for j in (i - 13)..=i {
                if j == 0 { continue; }
                let tr = (highs[j] - lows[j])
                    .max((highs[j] - closes[j - 1]).abs())
                    .max((lows[j] - closes[j - 1]).abs());
                let plus_dm = if highs[j] - highs[j - 1] > lows[j - 1] - lows[j] && highs[j] - highs[j - 1] > 0.0 {
                    highs[j] - highs[j - 1]
                } else {
                    0.0
                };
                let minus_dm = if lows[j - 1] - lows[j] > highs[j] - highs[j - 1] && lows[j - 1] - lows[j] > 0.0 {
                    lows[j - 1] - lows[j]
                } else {
                    0.0
                };
                if tr > 0.0 {
                    let plus_di = plus_dm / tr;
                    let minus_di = minus_dm / tr;
                    let di_sum = plus_di + minus_di;
                    if di_sum > 0.0 {
                        sum_dx += (plus_di - minus_di).abs() / di_sum;
                        cnt += 1;
                    }
                }
            }
            if cnt > 0 { sum_dx / cnt as f64 * 100.0 } else { f64::NAN }
        } else {
            f64::NAN
        };

        // VWAP (rolling 20)
        let dist_vwap = if i >= 19 {
            let mut tp_v = 0.0;
            let mut v_sum = 0.0;
            for j in (i - 19)..=i {
                let tp = (highs[j] + lows[j] + closes[j]) / 3.0;
                tp_v += tp * volumes[j];
                v_sum += volumes[j];
            }
            if v_sum > 0.0 {
                let vwap = tp_v / v_sum;
                (closes[i] - vwap) / vwap * 100.0
            } else {
                f64::NAN
            }
        } else {
            f64::NAN
        };

        let valid = z.is_finite() && rsi[i].is_finite();
        result.push((candles[i].ts, Ind15m { z, rsi: rsi[i], adx, sma20_slope, dist_vwap, valid }));
    }
    result
}

// ── Scalp Entry Detection ──────────────────────────────────────────────

fn scalp_entry(ind: &Ind1m, price: f64) -> Option<(Dir, &'static str)> {
    if !ind.valid { return None; }

    let vol_r = ind.vol_ratio;

    // 1. scalp_vol_spike_rev
    if vol_r > VOL_SPIKE_MULT {
        if ind.rsi < RSI_EXTREME {
            return Some((Dir::Long, "vol_spike_rev"));
        }
        if ind.rsi > 100.0 - RSI_EXTREME {
            return Some((Dir::Short, "vol_spike_rev"));
        }
    }

    // 2. scalp_stoch_cross
    if ind.stoch_k.is_finite() && ind.stoch_d.is_finite()
        && ind.stoch_k_prev.is_finite() && ind.stoch_d_prev.is_finite()
    {
        if ind.stoch_k_prev <= ind.stoch_d_prev
            && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < STOCH_EXTREME
            && ind.stoch_d < STOCH_EXTREME
        {
            return Some((Dir::Long, "stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev
            && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > 100.0 - STOCH_EXTREME
            && ind.stoch_d > 100.0 - STOCH_EXTREME
        {
            return Some((Dir::Short, "stoch_cross"));
        }
    }

    // 3. scalp_bb_squeeze_break
    if ind.bb_width_avg.is_finite() && ind.bb_width_avg > 0.0
        && ind.bb_upper.is_finite()
    {
        let squeeze = ind.bb_width < ind.bb_width_avg * BB_SQUEEZE_FACTOR;
        if squeeze && vol_r > 2.0 {
            if price > ind.bb_upper {
                return Some((Dir::Long, "bb_squeeze"));
            }
            if price < ind.bb_lower {
                return Some((Dir::Short, "bb_squeeze"));
            }
        }
    }

    None
}

// ── Trade Collection ───────────────────────────────────────────────────

fn collect_trades(
    candles_1m: &[Candle],
    inds_1m: &[Ind1m],
    inds_15m: &[(NaiveDateTime, Ind15m)],
    breadth_map: &HashMap<NaiveDateTime, f64>,
) -> Vec<Trade> {
    let n = candles_1m.len();
    let mut trades = Vec::new();
    let mut i = 0;

    while i < n {
        let ind = &inds_1m[i];
        let price = candles_1m[i].c;

        let Some((direction, strategy)) = scalp_entry(ind, price) else {
            i += 1;
            continue;
        };

        let entry_price = price;
        let entry_ts = candles_1m[i].ts;

        // Find closest 15m indicator
        let ind_15m = find_closest_15m(inds_15m, entry_ts);

        // Find closest breadth
        let breadth_val = find_closest_breadth(breadth_map, entry_ts);

        // Build snapshot
        let sign = if direction == Dir::Long { 1.0 } else { -1.0 };
        let snap = Snapshot {
            dir_roc_3: if ind.roc_3.is_finite() { ind.roc_3 * sign } else { f64::NAN },
            avg_body_3: ind.avg_body_3,
            spread_pct: ind.spread_pct,
            dir_ema_spread: if ind.ema_spread.is_finite() { ind.ema_spread * sign } else { f64::NAN },
            atr_ratio: ind.atr_ratio,
            vol_ratio: ind.vol_ratio,
            vol_trend: ind.vol_trend,
            body_ratio: ind.body_ratio,
            z_15m: ind_15m.as_ref().map_or(f64::NAN, |x| x.z),
            rsi_15m: ind_15m.as_ref().map_or(f64::NAN, |x| x.rsi),
            adx_15m: ind_15m.as_ref().map_or(f64::NAN, |x| x.adx),
            sma20_slope_15m: ind_15m.as_ref().map_or(f64::NAN, |x| x.sma20_slope),
            dist_vwap_15m: ind_15m.as_ref().map_or(f64::NAN, |x| x.dist_vwap),
            breadth: breadth_val,
            mfi: ind.mfi,
            vol_delta_proxy: ind.vol_delta_proxy,
            bb_pctb: ind.bb_pctb,
            rsi_slope: ind.rsi_slope,
        };

        // Simulate trade
        let mut outcome = Outcome::Loss;
        let mut pnl_pct = 0.0;
        let mut bars_held = 0;

        for j in (i + 1)..((i + MAX_HOLD_BARS + 1).min(n)) {
            let p = candles_1m[j].c;
            let pnl = if direction == Dir::Long {
                (p - entry_price) / entry_price
            } else {
                (entry_price - p) / entry_price
            };
            bars_held = j - i;

            if pnl >= SCALP_TP {
                outcome = Outcome::Win;
                pnl_pct = SCALP_TP * LEVERAGE * 100.0;
                break;
            } else if pnl <= -SCALP_SL {
                outcome = Outcome::Loss;
                pnl_pct = -SCALP_SL * LEVERAGE * 100.0;
                break;
            }

            if j == (i + MAX_HOLD_BARS).min(n - 1) {
                pnl_pct = pnl * LEVERAGE * 100.0;
                outcome = if pnl > 0.0 { Outcome::Win } else { Outcome::Loss };
            }
        }

        trades.push(Trade { direction, strategy, outcome, pnl_pct, snap });
        i += bars_held.max(1) + 1;
    }
    trades
}

fn find_closest_15m(inds: &[(NaiveDateTime, Ind15m)], ts: NaiveDateTime) -> Option<Ind15m> {
    // Binary search for last 15m candle <= ts
    let idx = inds.partition_point(|(t, _)| *t <= ts);
    if idx > 0 {
        let (_, ind) = &inds[idx - 1];
        if ind.valid { Some(ind.clone()) } else { None }
    } else {
        None
    }
}

fn find_closest_breadth(map: &HashMap<NaiveDateTime, f64>, ts: NaiveDateTime) -> f64 {
    // Quantize ts to 15m boundary
    let mins = ts.and_utc().timestamp() / 60;
    let q = (mins / 15) * 15;
    let key = chrono::DateTime::from_timestamp(q * 60, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or(ts);
    map.get(&key).copied().unwrap_or(f64::NAN)
}

// ── Filter Logic ───────────────────────────────────────────────────────

struct Filter {
    name: &'static str,
    passes: fn(&Snapshot, f64) -> bool,
}

fn find_optimal_threshold(
    trades: &[Trade],
    extract: fn(&Snapshot) -> f64,
    higher_is_better: bool,
) -> f64 {
    // Find threshold that maximizes WR improvement while keeping ≥50% trades
    let mut vals: Vec<(f64, bool)> = trades
        .iter()
        .map(|t| (extract(&t.snap), t.outcome == Outcome::Win))
        .filter(|(v, _)| v.is_finite())
        .collect();
    vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if vals.is_empty() { return f64::NAN; }

    let total = vals.len();
    let base_wins: usize = vals.iter().filter(|(_, w)| *w).count();
    let base_wr = base_wins as f64 / total as f64;

    let mut best_threshold = f64::NAN;
    let mut best_improvement = 0.0;

    // Test percentile thresholds: 20%, 25%, 30%, 33%, 40%
    for pct in [20, 25, 30, 33, 40] {
        let cut_idx = total * pct / 100;
        if cut_idx >= total { continue; }

        if higher_is_better {
            // Keep top (100-pct)%
            let threshold = vals[cut_idx].0;
            let kept = &vals[cut_idx..];
            if kept.len() < 20 { continue; }
            let wr = kept.iter().filter(|(_, w)| *w).count() as f64 / kept.len() as f64;
            let imp = wr - base_wr;
            if imp > best_improvement {
                best_improvement = imp;
                best_threshold = threshold;
            }
        } else {
            // Keep bottom (100-pct)%
            let cut_high = total - cut_idx;
            let threshold = vals[cut_high].0;
            let kept = &vals[..cut_high];
            if kept.len() < 20 { continue; }
            let wr = kept.iter().filter(|(_, w)| *w).count() as f64 / kept.len() as f64;
            let imp = wr - base_wr;
            if imp > best_improvement {
                best_improvement = imp;
                best_threshold = threshold;
            }
        }
    }
    best_threshold
}

fn evaluate_filter(
    trades: &[Trade],
    pass_fn: &dyn Fn(&Snapshot) -> bool,
) -> (f64, f64, f64) {
    // Returns (base_wr, filtered_wr, kept_pct)
    let total = trades.len() as f64;
    if total == 0.0 { return (0.0, 0.0, 0.0); }

    let base_wins = trades.iter().filter(|t| t.outcome == Outcome::Win).count() as f64;
    let base_wr = base_wins / total * 100.0;

    let kept: Vec<&Trade> = trades.iter().filter(|t| pass_fn(&t.snap)).collect();
    let kept_total = kept.len() as f64;
    if kept_total == 0.0 { return (base_wr, 0.0, 0.0); }

    let kept_wins = kept.iter().filter(|t| t.outcome == Outcome::Win).count() as f64;
    let filtered_wr = kept_wins / kept_total * 100.0;
    let kept_pct = kept_total / total * 100.0;

    (base_wr, filtered_wr, kept_pct)
}

// ── Walk-Forward per Coin ──────────────────────────────────────────────

fn run_coin(
    coin: &str,
    all_1m: &[Candle],
    all_15m: &[Candle],
    all_15m_data: &HashMap<String, Vec<Candle>>,
) -> CoinWindowResult {
    let mut windows_results = Vec::new();

    for w in WINDOWS {
        // Slice data for train and test
        let train_1m = slice_candles(all_1m, w.train_start, w.train_end);
        let test_1m = slice_candles(all_1m, w.test_start, w.test_end);
        let train_15m = slice_candles(all_15m, w.train_start, w.train_end);
        let test_15m = slice_candles(all_15m, w.test_start, w.test_end);

        if train_1m.len() < 100 || test_1m.len() < 100 { continue; }

        // Compute indicators
        let train_1m_ind = compute_1m_indicators(&train_1m);
        let test_1m_ind = compute_1m_indicators(&test_1m);
        let train_15m_ind = compute_15m_indicators(&train_15m);
        let test_15m_ind = compute_15m_indicators(&test_15m);

        // Build breadth maps for train and test
        let train_breadth = build_breadth_map(all_15m_data, w.train_start, w.train_end);
        let test_breadth = build_breadth_map(all_15m_data, w.test_start, w.test_end);

        // Collect trades
        let train_trades = collect_trades(&train_1m, &train_1m_ind, &train_15m_ind, &train_breadth);
        let test_trades = collect_trades(&test_1m, &test_1m_ind, &test_15m_ind, &test_breadth);

        if train_trades.len() < 10 || test_trades.len() < 10 { continue; }

        let train_wr = train_trades.iter().filter(|t| t.outcome == Outcome::Win).count() as f64
            / train_trades.len() as f64 * 100.0;
        let test_wr = test_trades.iter().filter(|t| t.outcome == Outcome::Win).count() as f64
            / test_trades.len() as f64 * 100.0;

        // === DISCOVER FILTER THRESHOLDS ON TRAIN ===

        // F1: dir_roc_3 < threshold (counter-momentum, lower is better → keep_below)
        let thresh_roc3 = find_optimal_threshold(&train_trades, |s| s.dir_roc_3, false);

        // F2: avg_body_3 > threshold (active candles, higher is better)
        let thresh_body3 = find_optimal_threshold(&train_trades, |s| s.avg_body_3, true);

        // F3: spread_pct > threshold (volatility, higher is better)
        let thresh_spread = find_optimal_threshold(&train_trades, |s| s.spread_pct, true);

        // F4: 15m_z alignment
        // For longs: z_15m > threshold, for shorts: z_15m > threshold (overbought for short reversal)
        // We use direction-adjusted: long wants higher z (not too bearish), short wants higher z (overbought)
        // Actually from the data: long correlates +0.12 with z (higher z = more wins),
        // short correlates -0.15 with z (lower z = more wins for shorts, i.e. they want z HIGH for shorts because
        // that means overbought). So for both we want z to be "appropriate" for direction.
        // Let's just find the threshold on 15m_z that splits best, direction-aware
        let thresh_z15m_long = find_optimal_threshold(
            &train_trades.iter().filter(|t| t.direction == Dir::Long).cloned().collect::<Vec<_>>(),
            |s| s.z_15m,
            true,  // higher z = better for longs
        );
        let thresh_z15m_short = find_optimal_threshold(
            &train_trades.iter().filter(|t| t.direction == Dir::Short).cloned().collect::<Vec<_>>(),
            |s| s.z_15m,
            true,  // higher z = better for shorts (overbought = reversal works)
        );

        // F5: atr_ratio > threshold (higher volatility, higher is better)
        let thresh_atr = find_optimal_threshold(&train_trades, |s| s.atr_ratio, true);

        // === EVALUATE EACH FILTER ON TRAIN AND TEST ===

        let mut filters = Vec::new();

        // F1: dir_roc_3
        if thresh_roc3.is_finite() {
            let t = thresh_roc3;
            let pass = |s: &Snapshot| -> bool { s.dir_roc_3.is_finite() && s.dir_roc_3 < t };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F1:dir_roc_3<{:.4}", t),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        // F2: avg_body_3
        if thresh_body3.is_finite() {
            let t = thresh_body3;
            let pass = |s: &Snapshot| -> bool { s.avg_body_3.is_finite() && s.avg_body_3 > t };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F2:avg_body_3>{:.4}", t),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        // F3: spread_pct
        if thresh_spread.is_finite() {
            let t = thresh_spread;
            let pass = |s: &Snapshot| -> bool { s.spread_pct.is_finite() && s.spread_pct > t };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F3:spread_pct>{:.4}", t),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        // F4: 15m_z alignment (direction-specific)
        if thresh_z15m_long.is_finite() || thresh_z15m_short.is_finite() {
            let tl = thresh_z15m_long;
            let ts_val = thresh_z15m_short;
            let pass = move |s: &Snapshot| -> bool {
                // We can't know direction from snapshot alone, so use z_15m directly:
                // For this combined filter: just check if z is above thresholds
                // Since we found thresholds per direction on train, use the safer one
                s.z_15m.is_finite()
            };
            // Per-direction evaluation
            let long_trades_train: Vec<Trade> = train_trades.iter().filter(|t| t.direction == Dir::Long).cloned().collect();
            let long_trades_test: Vec<Trade> = test_trades.iter().filter(|t| t.direction == Dir::Long).cloned().collect();
            let short_trades_train: Vec<Trade> = train_trades.iter().filter(|t| t.direction == Dir::Short).cloned().collect();
            let short_trades_test: Vec<Trade> = test_trades.iter().filter(|t| t.direction == Dir::Short).cloned().collect();

            if thresh_z15m_long.is_finite() && !long_trades_train.is_empty() && !long_trades_test.is_empty() {
                let t = thresh_z15m_long;
                let pass_l = |s: &Snapshot| -> bool { s.z_15m.is_finite() && s.z_15m > t };
                let (tr_b, tr_f, tr_k) = evaluate_filter(&long_trades_train, &pass_l);
                let (te_b, te_f, te_k) = evaluate_filter(&long_trades_test, &pass_l);
                let tr_imp = tr_f - tr_b;
                let te_imp = te_f - te_b;
                let deg = if tr_imp.abs() > 0.01 { (tr_imp - te_imp) / tr_imp * 100.0 } else { 0.0 };
                filters.push(FilterResult {
                    name: format!("F4a:LONG_z15m>{:.2}", t),
                    train_wr_base: tr_b, train_wr_filtered: tr_f,
                    train_improvement: tr_imp, train_kept_pct: tr_k,
                    test_wr_base: te_b, test_wr_filtered: te_f,
                    test_improvement: te_imp, test_kept_pct: te_k,
                    degradation_pct: deg, oos_holds: te_imp > 0.0,
                });
            }
            if thresh_z15m_short.is_finite() && !short_trades_train.is_empty() && !short_trades_test.is_empty() {
                let t = thresh_z15m_short;
                let pass_s = |s: &Snapshot| -> bool { s.z_15m.is_finite() && s.z_15m > t };
                let (tr_b, tr_f, tr_k) = evaluate_filter(&short_trades_train, &pass_s);
                let (te_b, te_f, te_k) = evaluate_filter(&short_trades_test, &pass_s);
                let tr_imp = tr_f - tr_b;
                let te_imp = te_f - te_b;
                let deg = if tr_imp.abs() > 0.01 { (tr_imp - te_imp) / tr_imp * 100.0 } else { 0.0 };
                filters.push(FilterResult {
                    name: format!("F4b:SHORT_z15m>{:.2}", t),
                    train_wr_base: tr_b, train_wr_filtered: tr_f,
                    train_improvement: tr_imp, train_kept_pct: tr_k,
                    test_wr_base: te_b, test_wr_filtered: te_f,
                    test_improvement: te_imp, test_kept_pct: te_k,
                    degradation_pct: deg, oos_holds: te_imp > 0.0,
                });
            }
        }

        // F5: atr_ratio
        if thresh_atr.is_finite() {
            let t = thresh_atr;
            let pass = |s: &Snapshot| -> bool { s.atr_ratio.is_finite() && s.atr_ratio > t };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F5:atr_ratio>{:.4}", t),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        // F6: COMBINED dir_roc_3 + avg_body_3
        if thresh_roc3.is_finite() && thresh_body3.is_finite() {
            let tr = thresh_roc3;
            let tb = thresh_body3;
            let pass = move |s: &Snapshot| -> bool {
                s.dir_roc_3.is_finite() && s.dir_roc_3 < tr
                    && s.avg_body_3.is_finite() && s.avg_body_3 > tb
            };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F6:roc3<{:.3}+body3>{:.3}", tr, tb),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        // F7: COMBINED dir_roc_3 + avg_body_3 + spread_pct
        if thresh_roc3.is_finite() && thresh_body3.is_finite() && thresh_spread.is_finite() {
            let tr = thresh_roc3;
            let tb = thresh_body3;
            let tsp = thresh_spread;
            let pass = move |s: &Snapshot| -> bool {
                s.dir_roc_3.is_finite() && s.dir_roc_3 < tr
                    && s.avg_body_3.is_finite() && s.avg_body_3 > tb
                    && s.spread_pct.is_finite() && s.spread_pct > tsp
            };
            let (train_base, train_filt, train_kept) = evaluate_filter(&train_trades, &pass);
            let (test_base, test_filt, test_kept) = evaluate_filter(&test_trades, &pass);
            let train_imp = train_filt - train_base;
            let test_imp = test_filt - test_base;
            let deg = if train_imp.abs() > 0.01 { (train_imp - test_imp) / train_imp * 100.0 } else { 0.0 };
            filters.push(FilterResult {
                name: format!("F7:triple_combo"),
                train_wr_base: train_base, train_wr_filtered: train_filt,
                train_improvement: train_imp, train_kept_pct: train_kept,
                test_wr_base: test_base, test_wr_filtered: test_filt,
                test_improvement: test_imp, test_kept_pct: test_kept,
                degradation_pct: deg, oos_holds: test_imp > 0.0,
            });
        }

        windows_results.push(WindowResult {
            window: w.name.to_string(),
            train_trades: train_trades.len(),
            test_trades: test_trades.len(),
            train_wr,
            test_wr,
            filters,
        });
    }

    CoinWindowResult { coin: coin.to_string(), windows: windows_results }
}

fn build_breadth_map(
    all_15m_data: &HashMap<String, Vec<Candle>>,
    start: &str,
    end: &str,
) -> HashMap<NaiveDateTime, f64> {
    // Collect z-scores per timestamp for all coins, compute breadth
    let mut z_by_ts: HashMap<NaiveDateTime, Vec<f64>> = HashMap::new();

    for (_, candles) in all_15m_data {
        let sliced = slice_candles(candles, start, end);
        let inds = compute_15m_indicators(&sliced);
        for (ts, ind) in &inds {
            if ind.z.is_finite() {
                z_by_ts.entry(*ts).or_default().push(ind.z);
            }
        }
    }

    let mut breadth_map = HashMap::new();
    for (ts, zs) in &z_by_ts {
        let below = zs.iter().filter(|&&z| z < -1.0).count() as f64;
        let total = zs.len() as f64;
        if total > 0.0 {
            breadth_map.insert(*ts, below / total);
        }
    }
    breadth_map
}

// ── Main ───────────────────────────────────────────────────────────────

fn main() {
    println!("{}", "=".repeat(90));
    println!("RUN10.2 — Walk-Forward Validation of Scalp Filters (Rust)");
    println!("{}", "=".repeat(90));
    println!("Scalp params: SL={:.2}% TP={:.2}% VMult={} RSI_ext={} Stoch_ext={} BB_sq={}",
             SCALP_SL * 100.0, SCALP_TP * 100.0, VOL_SPIKE_MULT, RSI_EXTREME, STOCH_EXTREME, BB_SQUEEZE_FACTOR);
    println!("Windows: {} | Coins: {}", WINDOWS.len(), COINS.len());
    println!("Filters: F1=dir_roc_3, F2=avg_body_3, F3=spread_pct, F4=15m_z, F5=atr_ratio, F6=combo, F7=triple");
    println!("{}", "=".repeat(90));

    // Load all 15m data first (needed for breadth)
    println!("\nLoading 15m data for breadth...");
    let mut all_15m_data: HashMap<String, Vec<Candle>> = HashMap::new();
    for &coin in COINS {
        let candles = load_candles(coin, "15m");
        if !candles.is_empty() {
            println!("  {}: {} candles", coin, candles.len());
            all_15m_data.insert(coin.to_string(), candles);
        }
    }

    // Load all 1m data
    println!("\nLoading 1m data...");
    let mut all_1m_data: HashMap<String, Vec<Candle>> = HashMap::new();
    for &coin in COINS {
        let candles = load_candles(coin, "1m");
        if !candles.is_empty() {
            println!("  {}: {} candles", coin, candles.len());
            all_1m_data.insert(coin.to_string(), candles);
        }
    }

    println!("\nRunning walk-forward validation ({} coins in parallel)...\n", COINS.len());

    // Process coins in parallel
    let coin_results: Vec<CoinWindowResult> = COINS
        .par_iter()
        .filter_map(|&coin| {
            let candles_1m = all_1m_data.get(coin)?;
            let candles_15m = all_15m_data.get(coin)?;
            let result = run_coin(coin, candles_1m, candles_15m, &all_15m_data);
            // Print progress
            let total_train: usize = result.windows.iter().map(|w| w.train_trades).sum();
            let total_test: usize = result.windows.iter().map(|w| w.test_trades).sum();
            println!("  {} done: {} train / {} test trades across {} windows",
                     coin, total_train, total_test, result.windows.len());
            Some(result)
        })
        .collect();

    // === PRINT RESULTS ===

    println!("\n{}", "=".repeat(90));
    println!("RESULTS");
    println!("{}", "=".repeat(90));

    let mut total_train = 0;
    let mut total_test = 0;
    let mut sum_train_wr = 0.0;
    let mut sum_test_wr = 0.0;
    let mut wr_count = 0;

    // Aggregate filter results across all coins and windows
    let mut filter_agg: HashMap<String, Vec<(f64, f64, f64, bool)>> = HashMap::new();
    // key = filter name prefix (F1, F2, etc.), value = [(train_imp, test_imp, test_kept_pct, oos_holds)]

    for cr in &coin_results {
        println!("\n--- {} ---", cr.coin);
        for wr in &cr.windows {
            total_train += wr.train_trades;
            total_test += wr.test_trades;
            sum_train_wr += wr.train_wr;
            sum_test_wr += wr.test_wr;
            wr_count += 1;

            println!("  {}: {}tr/{}te, WR {:.1}%/{:.1}%",
                     wr.window, wr.train_trades, wr.test_trades, wr.train_wr, wr.test_wr);

            for f in &wr.filters {
                let prefix = f.name.split(':').next().unwrap_or(&f.name).to_string();
                let oos = if f.oos_holds { "✓" } else { "✗" };
                println!("    {} train:{:+.1}% test:{:+.1}% kept:{:.0}% deg:{:.0}% {}",
                         f.name, f.train_improvement, f.test_improvement,
                         f.test_kept_pct, f.degradation_pct, oos);

                filter_agg.entry(prefix).or_default().push((
                    f.train_improvement,
                    f.test_improvement,
                    f.test_kept_pct,
                    f.oos_holds,
                ));
            }
        }
    }

    // === AGGREGATED FILTER SUMMARY ===

    println!("\n{}", "=".repeat(90));
    println!("AGGREGATED FILTER PERFORMANCE");
    println!("{}", "=".repeat(90));
    println!("{:<12} {:>10} {:>10} {:>8} {:>8} {:>8}  {}",
             "Filter", "Avg Train", "Avg Test", "Avg Deg", "OOS+%", "Kept%", "Verdict");
    println!("{}", "-".repeat(80));

    let mut agg_filters = Vec::new();
    let mut sorted_keys: Vec<String> = filter_agg.keys().cloned().collect();
    sorted_keys.sort();

    for key in &sorted_keys {
        let vals = &filter_agg[key];
        if vals.is_empty() { continue; }
        let n = vals.len() as f64;
        let avg_train: f64 = vals.iter().map(|v| v.0).sum::<f64>() / n;
        let avg_test: f64 = vals.iter().map(|v| v.1).sum::<f64>() / n;
        let avg_kept: f64 = vals.iter().map(|v| v.2).sum::<f64>() / n;
        let oos_positive = vals.iter().filter(|v| v.3).count() as f64 / n * 100.0;
        let avg_deg = if avg_train.abs() > 0.01 {
            (avg_train - avg_test) / avg_train * 100.0
        } else {
            0.0
        };

        let verdict = if avg_test > 1.0 && oos_positive > 60.0 {
            "ACCEPT"
        } else if avg_test > 0.5 && oos_positive > 50.0 {
            "MARGINAL"
        } else {
            "REJECT"
        };

        println!("{:<12} {:>+9.1}% {:>+9.1}% {:>7.0}% {:>7.0}% {:>7.0}%  {}",
                 key, avg_train, avg_test, avg_deg, oos_positive, avg_kept, verdict);

        agg_filters.push(AggFilter {
            name: key.clone(),
            avg_train_improvement: (avg_train * 10.0).round() / 10.0,
            avg_test_improvement: (avg_test * 10.0).round() / 10.0,
            avg_degradation: (avg_deg * 10.0).round() / 10.0,
            oos_positive_pct: (oos_positive * 10.0).round() / 10.0,
            avg_test_kept_pct: (avg_kept * 10.0).round() / 10.0,
            recommendation: verdict.to_string(),
        });
    }

    // === SAVE RESULTS ===
    let avg_train_wr = if wr_count > 0 { sum_train_wr / wr_count as f64 } else { 0.0 };
    let avg_test_wr = if wr_count > 0 { sum_test_wr / wr_count as f64 } else { 0.0 };

    let results = FinalResults {
        summary: Summary {
            total_coins: coin_results.len(),
            windows: WINDOWS.len(),
            total_train_trades: total_train,
            total_test_trades: total_test,
            avg_train_wr: (avg_train_wr * 10.0).round() / 10.0,
            avg_test_wr: (avg_test_wr * 10.0).round() / 10.0,
        },
        per_coin: coin_results,
        aggregated_filters: agg_filters,
    };

    let json = serde_json::to_string_pretty(&results).unwrap();
    fs::write(RESULTS_FILE, &json).unwrap();
    println!("\nResults saved to {}", RESULTS_FILE);
    println!("Done.");
}
