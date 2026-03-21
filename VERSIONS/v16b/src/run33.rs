/// RUN33 — Uptrend Short Block Filter (v2: comprehensive indicator sweep)
///
/// The original ADX-based approach (v1) blocked ZERO trades — ADX never
/// coincides with high z-score because they're inversely correlated on the
/// same timeframe. High ADX = sustained trend → z stays moderate.
/// High z = transient spike → ADX hasn't responded yet.
///
/// v2 uses longer-timeframe trend indicators (SMA50/100/200, EMA cross,
/// multi-bar returns, z50, bullish structure) that CAN coexist with
/// high z-score to detect "this spike is happening in a larger uptrend."
///
/// Grid: 30 combos (1 baseline + 29 uptrend filters)
/// Decision gate: ΔP&L > 0 AND positive_days% ≥ 55% → proceed to RUN33.2
///
/// Run: cargo run --release --features run33 -- --run33

use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Constants (matching config.rs) ───────────────────────────────────────────
const SL: f64          = 0.003;
const MIN_HOLD: usize  = 2;
const FEE: f64         = 0.001;
const SLIP: f64        = 0.0005;
const RISK: f64        = 0.10;
const LEVERAGE: f64    = 5.0;
const INITIAL_BAL: f64 = 100.0;

const BREADTH_LONG_MAX:  f64 = 0.20;
const BREADTH_SHORT_MIN: f64 = 0.50;

const ISO_Z_THRESHOLD:    f64 = 1.5;
const ISO_Z_SPREAD:       f64 = 1.5;
const ISO_RSI_THRESHOLD:  f64 = 75.0;
const ISO_SQUEEZE_FACTOR: f64 = 0.8;

const OU_WINDOW:        usize = 100;
const OU_MIN_HALFLIFE:  f64   = 10.0;
const OU_MAX_HALFLIFE:  f64   = 100.0;
const OU_DEV_THRESHOLD: f64   = 2.0;

// ── Strategy enums ────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum LongStrat { VwapRev, BbBounce, AdrRev, DualRsi, OuMeanRev }

#[derive(Clone, Copy)]
enum ShortStrat { MeanRev, AdrRev, BbBounce, VwapRev }

#[derive(Clone, Copy)]
enum IsoStrat { RelativeZ, RsiExtreme, Divergence }

#[derive(Clone, Copy)]
enum Comp { LaguerreRsi(u8), Kalman, Kst, None }

#[derive(Clone, Copy, PartialEq)]
enum Mode { Long, IsoShort, Short }

#[derive(Clone, Copy, PartialEq)]
enum Dir { Long, Short }

// ── Coin config (exactly matching config.rs) ──────────────────────────────────
struct CoinCfg {
    name:   &'static str,
    long:   LongStrat,
    short:  ShortStrat,
    iso:    IsoStrat,
    comp:   Comp,
    comp_z: f64,
}

const COIN_CFGS: [CoinCfg; 18] = [
    CoinCfg { name: "DASH", long: LongStrat::OuMeanRev, short: ShortStrat::MeanRev, iso: IsoStrat::Divergence,  comp: Comp::Kst,            comp_z: -0.5 },
    CoinCfg { name: "UNI",  long: LongStrat::VwapRev,   short: ShortStrat::AdrRev,  iso: IsoStrat::RelativeZ,  comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "NEAR", long: LongStrat::VwapRev,   short: ShortStrat::AdrRev,  iso: IsoStrat::RsiExtreme, comp: Comp::LaguerreRsi(3), comp_z: -0.5 },
    CoinCfg { name: "ADA",  long: LongStrat::VwapRev,   short: ShortStrat::BbBounce,iso: IsoStrat::Divergence, comp: Comp::LaguerreRsi(0), comp_z: -1.0 },
    CoinCfg { name: "LTC",  long: LongStrat::VwapRev,   short: ShortStrat::MeanRev, iso: IsoStrat::RsiExtreme, comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "SHIB", long: LongStrat::VwapRev,   short: ShortStrat::VwapRev, iso: IsoStrat::RsiExtreme, comp: Comp::LaguerreRsi(3), comp_z: -1.0 },
    CoinCfg { name: "LINK", long: LongStrat::VwapRev,   short: ShortStrat::BbBounce,iso: IsoStrat::RelativeZ,  comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "ETH",  long: LongStrat::VwapRev,   short: ShortStrat::AdrRev,  iso: IsoStrat::RsiExtreme, comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "DOT",  long: LongStrat::VwapRev,   short: ShortStrat::VwapRev, iso: IsoStrat::RelativeZ,  comp: Comp::LaguerreRsi(2), comp_z: -1.5 },
    CoinCfg { name: "XRP",  long: LongStrat::VwapRev,   short: ShortStrat::BbBounce,iso: IsoStrat::RsiExtreme, comp: Comp::None,           comp_z:  0.0 },
    CoinCfg { name: "ATOM", long: LongStrat::VwapRev,   short: ShortStrat::AdrRev,  iso: IsoStrat::RelativeZ,  comp: Comp::LaguerreRsi(1), comp_z: -1.5 },
    CoinCfg { name: "SOL",  long: LongStrat::VwapRev,   short: ShortStrat::AdrRev,  iso: IsoStrat::RsiExtreme, comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "DOGE", long: LongStrat::BbBounce,  short: ShortStrat::BbBounce,iso: IsoStrat::Divergence, comp: Comp::None,           comp_z:  0.0 },
    CoinCfg { name: "XLM",  long: LongStrat::DualRsi,   short: ShortStrat::MeanRev, iso: IsoStrat::RelativeZ,  comp: Comp::LaguerreRsi(1), comp_z: -1.5 },
    CoinCfg { name: "AVAX", long: LongStrat::AdrRev,    short: ShortStrat::BbBounce,iso: IsoStrat::RelativeZ,  comp: Comp::Kalman,         comp_z: -1.5 },
    CoinCfg { name: "ALGO", long: LongStrat::AdrRev,    short: ShortStrat::AdrRev,  iso: IsoStrat::RsiExtreme, comp: Comp::LaguerreRsi(3), comp_z: -1.5 },
    CoinCfg { name: "BNB",  long: LongStrat::VwapRev,   short: ShortStrat::VwapRev, iso: IsoStrat::Divergence, comp: Comp::Kst,            comp_z: -1.0 },
    CoinCfg { name: "BTC",  long: LongStrat::BbBounce,  short: ShortStrat::AdrRev,  iso: IsoStrat::RsiExtreme, comp: Comp::Kst,            comp_z: -0.5 },
];
const N_COINS: usize = 18;

// ── Bar and Indicator structs ─────────────────────────────────────────────────
#[derive(Clone)]
struct Bar { o: f64, h: f64, l: f64, c: f64, v: f64 }

#[derive(Clone)]
struct Ind {
    // Core indicators (from run32)
    z:            f64,
    sma20:        f64,
    sma9:         f64,
    std20:        f64,
    bb_lo:        f64,
    bb_hi:        f64,
    bb_width:     f64,
    bb_width_avg: f64,
    vol:          f64,
    vol_ma:       f64,
    adr_lo:       f64,
    adr_hi:       f64,
    rsi14:        f64,
    rsi7:         f64,
    vwap:         f64,
    adx:          f64,
    ou_halflife:  f64,
    ou_deviation: f64,
    // Complement indicators
    lrsi_05: f64, lrsi_05_prev: f64,
    lrsi_06: f64, lrsi_06_prev: f64,
    lrsi_07: f64, lrsi_07_prev: f64,
    lrsi_08: f64, lrsi_08_prev: f64,
    kalman_est: f64, kalman_var: f64,
    kst: f64, kst_prev: f64,
    kst_signal: f64, kst_signal_prev: f64,
    // RUN33 v2: comprehensive uptrend detection
    p:            f64,   // close price
    sma50:        f64,
    sma100:       f64,
    sma200:       f64,
    ema12:        f64,
    ema26:        f64,
    ret48:        f64,   // c[i]/c[i-48] - 1.0  (12h return)
    ret96:        f64,   // c[i]/c[i-96] - 1.0  (24h return)
    z50:          f64,   // (c - sma50) / std50
    sma50_slope:  f64,   // sma50[i] - sma50[i-5]
    bull_ratio:   f64,   // fraction of last 20 bars where close > open
    higher_lows:  bool,  // l[i] > l[i-1] > l[i-2]
    valid:        bool,
}

// ── UpblockCfg ────────────────────────────────────────────────────────────────
const N_GRID: usize = 30;

const LABELS: [&str; N_GRID] = [
    "baseline",           //  0
    "p>sma50",            //  1  — price above 50-bar SMA
    "p>sma100",           //  2  — price above 100-bar SMA
    "p>sma200",           //  3  — price above 200-bar SMA
    "sma20>sma50",        //  4  — short MA above medium MA
    "sma20>sma100",       //  5  — short MA above long MA
    "sma50>sma100",       //  6  — medium MA above long MA
    "sma50_rising",       //  7  — 50-bar MA slope positive (5-bar Δ)
    "ema12>ema26",        //  8  — MACD line positive
    "ret48>0%",           //  9  — 12h return positive
    "ret48>1%",           // 10  — 12h return > 1%
    "ret48>2%",           // 11  — 12h return > 2%
    "ret96>0%",           // 12  — 24h return positive
    "ret96>2%",           // 13  — 24h return > 2%
    "z50>0",              // 14  — above 50-bar z-score mean
    "z50>0.5",            // 15  — moderately above 50-bar mean
    "rsi>55",             // 16  — RSI in bullish zone
    "rsi>60",             // 17  — RSI firmly bullish
    "bull>55%",           // 18  — >55% of last 20 bars bullish
    "bull>60%",           // 19  — >60% of last 20 bars bullish
    "hlows3",             // 20  — 3 consecutive higher lows
    "p>50+ret48>0",       // 21  — above SMA50 AND 12h return positive
    "sma20>50+rsi>55",    // 22  — MA alignment AND RSI bullish
    "p>100+ret96>0",      // 23  — above SMA100 AND 24h return positive
    "ema12>26+ret48>0",   // 24  — MACD positive AND 12h return positive
    "p>50+sma20>50",      // 25  — price AND SMA20 both above SMA50
    "50rise+ret48>0",     // 26  — SMA50 rising AND 12h return positive
    "p>50+rsi>55",        // 27  — above SMA50 AND RSI bullish
    "ret48>0+rsi>55",     // 28  — 12h positive AND RSI bullish
    "p>50+bull>55%",      // 29  — above SMA50 AND >55% bullish bars
];

#[derive(Clone, Copy)]
struct UpblockCfg { id: u8 }

impl UpblockCfg {
    fn blocks_short(&self, ind: &Ind) -> bool {
        match self.id {
            0  => false,
            // Category A: MA position
            1  => ind.sma50 > 0.0 && ind.p > ind.sma50,
            2  => !ind.sma100.is_nan() && ind.sma100 > 0.0 && ind.p > ind.sma100,
            3  => !ind.sma200.is_nan() && ind.sma200 > 0.0 && ind.p > ind.sma200,
            4  => ind.sma50 > 0.0 && ind.sma20 > ind.sma50,
            5  => !ind.sma100.is_nan() && ind.sma100 > 0.0 && ind.sma20 > ind.sma100,
            // Category B: MA cross / slope
            6  => ind.sma50 > 0.0 && !ind.sma100.is_nan() && ind.sma100 > 0.0 && ind.sma50 > ind.sma100,
            7  => ind.sma50_slope > 0.0,
            8  => !ind.ema12.is_nan() && !ind.ema26.is_nan() && ind.ema12 > ind.ema26,
            // Category C: momentum / return
            9  => ind.ret48 > 0.0,
            10 => ind.ret48 > 0.01,
            11 => ind.ret48 > 0.02,
            12 => ind.ret96 > 0.0,
            13 => ind.ret96 > 0.02,
            // Category D: z-score / RSI
            14 => !ind.z50.is_nan() && ind.z50 > 0.0,
            15 => !ind.z50.is_nan() && ind.z50 > 0.5,
            // RSI: short signals require z>1.5 (price above SMA), so RSI is typically elevated;
            // RSI>55 will block most shorts, RSI>60 fewer — we want to see the discrimination.
            16 => !ind.rsi14.is_nan() && ind.rsi14 > 55.0,
            17 => !ind.rsi14.is_nan() && ind.rsi14 > 60.0,
            // Category E: structure
            18 => ind.bull_ratio > 0.55,
            19 => ind.bull_ratio > 0.60,
            20 => ind.higher_lows,
            // Category F: combinations (require 2 conditions)
            21 => ind.sma50 > 0.0 && ind.p > ind.sma50 && ind.ret48 > 0.0,
            22 => ind.sma50 > 0.0 && ind.sma20 > ind.sma50 && !ind.rsi14.is_nan() && ind.rsi14 > 55.0,
            23 => !ind.sma100.is_nan() && ind.sma100 > 0.0 && ind.p > ind.sma100 && ind.ret96 > 0.0,
            24 => !ind.ema12.is_nan() && !ind.ema26.is_nan() && ind.ema12 > ind.ema26 && ind.ret48 > 0.0,
            25 => ind.sma50 > 0.0 && ind.p > ind.sma50 && ind.sma20 > ind.sma50,
            26 => ind.sma50_slope > 0.0 && ind.ret48 > 0.0,
            27 => ind.sma50 > 0.0 && ind.p > ind.sma50 && !ind.rsi14.is_nan() && ind.rsi14 > 55.0,
            28 => ind.ret48 > 0.0 && !ind.rsi14.is_nan() && ind.rsi14 > 55.0,
            29 => ind.sma50 > 0.0 && ind.p > ind.sma50 && ind.bull_ratio > 0.55,
            _  => false,
        }
    }

    fn label(&self) -> &'static str { LABELS[self.id as usize] }
}

// ── Rolling helpers ───────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += data[i];
        if i >= w { sum -= data[i - w]; }
        if i + 1 >= w { out[i] = sum / w as f64; }
    }
    out
}

fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        let s = &data[i + 1 - w..=i];
        let m = s.iter().sum::<f64>() / w as f64;
        let v = s.iter().map(|x| (x - m).powi(2)).sum::<f64>() / w as f64;
        out[i] = v.sqrt();
    }
    out
}

fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

fn rsum(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += data[i];
        if i >= w { sum -= data[i - w]; }
        if i + 1 >= w { out[i] = sum; }
    }
    out
}

fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len();
    let mut out = vec![f64::NAN; n];
    if n < period + 1 { return out; }
    let mut gains  = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = c[i] - c[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
    }
    let ag = rmean(&gains, period);
    let al = rmean(&losses, period);
    for i in 0..n {
        if !ag[i].is_nan() && !al[i].is_nan() {
            out[i] = if al[i] == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag[i] / al[i]) };
        }
    }
    out
}

fn ema_calc(data: &[f64], span: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut prev = f64::NAN;
    for i in 0..n {
        if data[i].is_nan() { out[i] = prev; continue; }
        prev = if prev.is_nan() { data[i] } else { alpha * data[i] + (1.0 - alpha) * prev };
        out[i] = prev;
    }
    out
}

fn adx_calc(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    let n = h.len();
    let mut pdm = vec![0.0; n];
    let mut mdm = vec![0.0; n];
    let mut tr  = vec![0.0; n];
    for i in 1..n {
        let hh = h[i] - h[i - 1];
        let ll = l[i - 1] - l[i];
        if hh > ll && hh > 0.0 { pdm[i] = hh; }
        if ll > hh && ll > 0.0 { mdm[i] = ll; }
        tr[i] = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
    }
    let atr = rmean(&tr,  14);
    let pdi = rmean(&pdm, 14);
    let mdi = rmean(&mdm, 14);
    let mut dx = vec![f64::NAN; n];
    for i in 0..n {
        if !atr[i].is_nan() && atr[i] > 0.0 {
            let p = 100.0 * pdi[i] / atr[i];
            let m = 100.0 * mdi[i] / atr[i];
            let s = p + m;
            if s > 0.0 { dx[i] = 100.0 * (p - m).abs() / s; }
        }
    }
    rmean(&dx, 14)
}

fn laguerre_rsi_all(c: &[f64], gamma: f64) -> Vec<(f64, f64)> {
    let n = c.len();
    let mut out = vec![(f64::NAN, f64::NAN); n];
    let mut l0 = 0.0_f64;
    let mut l1 = 0.0_f64;
    let mut l2 = 0.0_f64;
    let mut l3 = 0.0_f64;
    let mut prev_lrsi = f64::NAN;
    for i in 0..n {
        let pl0 = l0; let pl1 = l1; let pl2 = l2;
        l0 = (1.0 - gamma) * c[i] + gamma * l0;
        l1 = -gamma * l0 + pl0 + gamma * l1;
        l2 = -gamma * l1 + pl1 + gamma * l2;
        l3 = -gamma * l2 + pl2 + gamma * l3;
        let mut cu = 0.0; let mut cd = 0.0;
        let d0 = l0 - l1; let d1 = l1 - l2; let d2 = l2 - l3;
        if d0 > 0.0 { cu += d0; } else { cd -= d0; }
        if d1 > 0.0 { cu += d1; } else { cd -= d1; }
        if d2 > 0.0 { cu += d2; } else { cd -= d2; }
        if i >= 3 {
            let cur = if cu + cd > 0.0 { cu / (cu + cd) * 100.0 } else { 50.0 };
            out[i] = (cur, prev_lrsi);
            prev_lrsi = cur;
        }
    }
    out
}

fn kalman_all(c: &[f64], q: f64, r: f64) -> Vec<(f64, f64)> {
    let n = c.len();
    let mut out = vec![(f64::NAN, f64::NAN); n];
    if n == 0 { return out; }
    let mut x = c[0];
    let mut p = 1.0_f64;
    out[0] = (x, p);
    for i in 1..n {
        let p_pred = p + q;
        let k = p_pred / (p_pred + r);
        x = x + k * (c[i] - x);
        p = (1.0 - k) * p_pred;
        out[i] = (x, p);
    }
    out
}

fn kst_all(c: &[f64]) -> Vec<(f64, f64, f64, f64)> {
    let n = c.len();
    let mut out = vec![(f64::NAN, f64::NAN, f64::NAN, f64::NAN); n];
    if n < 60 { return out; }
    let mut roc10 = vec![f64::NAN; n];
    let mut roc15 = vec![f64::NAN; n];
    let mut roc20 = vec![f64::NAN; n];
    let mut roc30 = vec![f64::NAN; n];
    for i in 10..n { if c[i-10] > 0.0 { roc10[i] = (c[i]-c[i-10])/c[i-10]*100.0; } }
    for i in 15..n { if c[i-15] > 0.0 { roc15[i] = (c[i]-c[i-15])/c[i-15]*100.0; } }
    for i in 20..n { if c[i-20] > 0.0 { roc20[i] = (c[i]-c[i-20])/c[i-20]*100.0; } }
    for i in 30..n { if c[i-30] > 0.0 { roc30[i] = (c[i]-c[i-30])/c[i-30]*100.0; } }
    let sr10 = rmean(&roc10, 10);
    let sr15 = rmean(&roc15, 10);
    let sr20 = rmean(&roc20, 10);
    let sr30 = rmean(&roc30, 15);
    let mut kst_v = vec![f64::NAN; n];
    for i in 0..n {
        if !sr10[i].is_nan()&&!sr15[i].is_nan()&&!sr20[i].is_nan()&&!sr30[i].is_nan() {
            kst_v[i] = sr10[i]*1.0 + sr15[i]*2.0 + sr20[i]*3.0 + sr30[i]*4.0;
        }
    }
    let sig_v = rmean(&kst_v, 9);
    for i in 1..n {
        out[i] = (kst_v[i], sig_v[i], kst_v[i-1], sig_v[i-1]);
    }
    out
}

fn ou_at(i: usize, c: &[f64], sma20: &[f64]) -> (f64, f64) {
    if i < OU_WINDOW + 1 { return (f64::NAN, f64::NAN); }
    let start = i - OU_WINDOW + 1;
    let mut sx = 0.0; let mut sy = 0.0; let mut sxy = 0.0; let mut sx2 = 0.0; let mut cnt = 0u32;
    for j in start..=i {
        if sma20[j].is_nan() || sma20[j-1].is_nan() { continue; }
        let spread = c[j-1] - sma20[j-1];
        let delta  = (c[j] - sma20[j]) - spread;
        sx += spread; sy += delta; sxy += spread*delta; sx2 += spread*spread; cnt += 1;
    }
    if cnt < 50 { return (f64::NAN, f64::NAN); }
    let nf = cnt as f64;
    let denom = nf * sx2 - sx * sx;
    if denom.abs() < 1e-15 { return (f64::NAN, f64::NAN); }
    let b = (nf * sxy - sx * sy) / denom;
    if b >= 0.0 { return (f64::NAN, f64::NAN); }
    let hl = -(2.0_f64.ln()) / b;
    (hl, c[i] - sma20[i])
}

// ── Batch indicator computation ───────────────────────────────────────────────
fn compute_indicators(bars: &[Bar], is_dash: bool) -> Vec<Ind> {
    let n = bars.len();
    let c: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let h: Vec<f64> = bars.iter().map(|b| b.h).collect();
    let l: Vec<f64> = bars.iter().map(|b| b.l).collect();
    let v: Vec<f64> = bars.iter().map(|b| b.v).collect();

    // Core indicators
    let sma20  = rmean(&c, 20);
    let sma9   = rmean(&c, 9);
    let std20  = rstd(&c, 20);
    let vol_ma = rmean(&v, 20);
    let adr_lo = rmin(&l, 24);
    let adr_hi = rmax(&h, 24);
    let rsi14  = rsi_calc(&c, 14);
    let rsi7   = rsi_calc(&c, 7);
    let adx    = adx_calc(&h, &l, &c);

    let tp: Vec<f64>  = (0..n).map(|i| (h[i]+l[i]+c[i])/3.0).collect();
    let tpv: Vec<f64> = (0..n).map(|i| tp[i]*v[i]).collect();
    let tpv_sum = rsum(&tpv, 20);
    let v_sum   = rsum(&v,   20);

    let bb_width_raw: Vec<f64> = (0..n).map(|i| {
        if !std20[i].is_nan() { 4.0*std20[i] } else { f64::NAN }
    }).collect();
    let bb_width_avg = rmean(&bb_width_raw, 20);

    // Complement indicators
    let lrsi05 = laguerre_rsi_all(&c, 0.5);
    let lrsi06 = laguerre_rsi_all(&c, 0.6);
    let lrsi07 = laguerre_rsi_all(&c, 0.7);
    let lrsi08 = laguerre_rsi_all(&c, 0.8);
    let kalman = kalman_all(&c, 0.0001, 1.0);
    let kst    = kst_all(&c);

    // RUN33 v2: additional trend indicators
    let sma50  = rmean(&c, 50);
    let sma100 = rmean(&c, 100);
    let sma200 = rmean(&c, 200);
    let std50  = rstd(&c, 50);
    let ema12v = ema_calc(&c, 12);
    let ema26v = ema_calc(&c, 26);

    // Bullish bar ratio: fraction of last 20 bars where close > open
    let bull_flags: Vec<f64> = (0..n).map(|i| if c[i] > bars[i].o { 1.0 } else { 0.0 }).collect();
    let bull_sum = rsum(&bull_flags, 20);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let vwap = if !tpv_sum[i].is_nan() && !v_sum[i].is_nan() && v_sum[i] > 0.0 {
            tpv_sum[i] / v_sum[i]
        } else { f64::NAN };

        let (ou_hl, ou_dev) = if is_dash { ou_at(i, &c, &sma20) } else { (f64::NAN, f64::NAN) };

        let valid = i >= 50
            && !sma20[i].is_nan()
            && !std20[i].is_nan() && std20[i] > 0.0
            && !rsi14[i].is_nan()
            && vol_ma[i] > 0.0;

        let z = if !sma20[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            (c[i] - sma20[i]) / std20[i]
        } else { f64::NAN };

        // RUN33 v2 indicators
        let ret48 = if i >= 48 && c[i-48] > 0.0 { c[i]/c[i-48] - 1.0 } else { 0.0 };
        let ret96 = if i >= 96 && c[i-96] > 0.0 { c[i]/c[i-96] - 1.0 } else { 0.0 };

        let z50 = if !sma50[i].is_nan() && !std50[i].is_nan() && std50[i] > 0.0 {
            (c[i] - sma50[i]) / std50[i]
        } else { f64::NAN };

        let sma50_slope = if i >= 5 && !sma50[i].is_nan() && !sma50[i-5].is_nan() {
            sma50[i] - sma50[i-5]
        } else { 0.0 };

        let bull_ratio = if !bull_sum[i].is_nan() { bull_sum[i] / 20.0 } else { 0.0 };

        let higher_lows = i >= 2 && l[i] > l[i-1] && l[i-1] > l[i-2];

        out.push(Ind {
            z, sma20: sma20[i], sma9: sma9[i], std20: std20[i],
            bb_lo: if !sma20[i].is_nan() { sma20[i] - 2.0*std20[i].max(0.0) } else { f64::NAN },
            bb_hi: if !sma20[i].is_nan() { sma20[i] + 2.0*std20[i].max(0.0) } else { f64::NAN },
            bb_width: bb_width_raw[i], bb_width_avg: bb_width_avg[i],
            vol: v[i], vol_ma: vol_ma[i],
            adr_lo: adr_lo[i], adr_hi: adr_hi[i],
            rsi14: rsi14[i], rsi7: rsi7[i],
            vwap,
            adx: adx[i],
            ou_halflife: ou_hl, ou_deviation: ou_dev,
            lrsi_05: lrsi05[i].0, lrsi_05_prev: lrsi05[i].1,
            lrsi_06: lrsi06[i].0, lrsi_06_prev: lrsi06[i].1,
            lrsi_07: lrsi07[i].0, lrsi_07_prev: lrsi07[i].1,
            lrsi_08: lrsi08[i].0, lrsi_08_prev: lrsi08[i].1,
            kalman_est: kalman[i].0, kalman_var: kalman[i].1,
            kst: kst[i].0, kst_signal: kst[i].1,
            kst_prev: kst[i].2, kst_signal_prev: kst[i].3,
            p: c[i],
            sma50: sma50[i], sma100: sma100[i], sma200: sma200[i],
            ema12: ema12v[i], ema26: ema26v[i],
            ret48, ret96, z50, sma50_slope, bull_ratio, higher_lows,
            valid,
        });
    }
    out
}

// ── CSV loader ────────────────────────────────────────────────────────────────
fn load_15m(coin: &str) -> (Vec<Bar>, Vec<u32>) {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_1year.csv",
        coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return (vec![], vec![]); }
    };
    let mut bars = Vec::with_capacity(36_000);
    let mut days = Vec::with_capacity(36_000);
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let ts  = it.next().unwrap_or("");
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if o.is_nan()||h.is_nan()||l.is_nan()||c.is_nan()||v.is_nan() { continue; }
        let day_key: u32 = if ts.len() >= 10 {
            let y: u32 = ts[0..4].parse().unwrap_or(0);
            let m: u32 = ts[5..7].parse().unwrap_or(0);
            let d: u32 = ts[8..10].parse().unwrap_or(0);
            y*10000 + m*100 + d
        } else { 0 };
        bars.push(Bar { o, h, l, c, v });
        days.push(day_key);
    }
    (bars, days)
}

// ── Strategy signal checks ────────────────────────────────────────────────────
fn long_signal(ind: &Ind, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    let p = ind.sma20 + ind.z * ind.std20;
    if p > ind.sma20 || ind.z > 0.5 { return false; }
    match strat {
        LongStrat::VwapRev => {
            !ind.vwap.is_nan() && ind.z < -1.5 && p < ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            !ind.bb_lo.is_nan() && ind.vol > ind.vol_ma * 1.3 && p <= ind.bb_lo * 1.02
        }
        LongStrat::AdrRev => {
            !ind.adr_lo.is_nan() && !ind.adr_hi.is_nan() && {
                let range = ind.adr_hi - ind.adr_lo;
                range > 0.0 && p <= ind.adr_lo + range * 0.25 && ind.vol > ind.vol_ma * 1.1
            }
        }
        LongStrat::DualRsi => {
            ind.rsi14 < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20
        }
        LongStrat::OuMeanRev => {
            !ind.ou_halflife.is_nan() && !ind.ou_deviation.is_nan()
                && ind.ou_halflife >= OU_MIN_HALFLIFE && ind.ou_halflife <= OU_MAX_HALFLIFE
                && ind.std20 > 0.0
                && (ind.ou_deviation / ind.std20) < -OU_DEV_THRESHOLD
        }
    }
}

fn short_signal(ind: &Ind, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    let p = ind.sma20 + ind.z * ind.std20;
    if p < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        ShortStrat::VwapRev => {
            !ind.vwap.is_nan() && ind.z > 1.5 && p > ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        ShortStrat::BbBounce => {
            !ind.bb_hi.is_nan() && p >= ind.bb_hi * 0.98 && ind.vol > ind.vol_ma * 1.3
        }
        ShortStrat::MeanRev => ind.z > 1.5,
        ShortStrat::AdrRev => {
            !ind.adr_hi.is_nan() && !ind.adr_lo.is_nan() && {
                let range = ind.adr_hi - ind.adr_lo;
                range > 0.0 && p >= ind.adr_hi - range * 0.25 && ind.vol > ind.vol_ma * 1.1
            }
        }
    }
}

fn iso_signal(ind: &Ind, strat: IsoStrat, avg_z: f64, avg_rsi: f64, btc_z: f64) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    let p = ind.sma20 + ind.z * ind.std20;
    if p < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        IsoStrat::RelativeZ  => avg_z.is_finite() && ind.z > avg_z + ISO_Z_SPREAD,
        IsoStrat::RsiExtreme => !ind.rsi14.is_nan() && ind.rsi14 > ISO_RSI_THRESHOLD && avg_rsi < 55.0,
        IsoStrat::Divergence => ind.z > ISO_Z_THRESHOLD && btc_z < 0.0,
    }
}

fn comp_signal(ind: &Ind, comp: Comp, comp_z: f64) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if matches!(comp, Comp::None) { return false; }
    if ind.z > comp_z { return false; }
    let p = ind.sma20 + ind.z * ind.std20;
    if p > ind.sma20 { return false; }
    match comp {
        Comp::LaguerreRsi(g) => {
            let (cur, prev) = match g {
                0 => (ind.lrsi_05, ind.lrsi_05_prev),
                1 => (ind.lrsi_06, ind.lrsi_06_prev),
                2 => (ind.lrsi_07, ind.lrsi_07_prev),
                _ => (ind.lrsi_08, ind.lrsi_08_prev),
            };
            !cur.is_nan() && !prev.is_nan() && cur < 20.0 && cur > prev
        }
        Comp::Kalman => {
            !ind.kalman_est.is_nan() && !ind.kalman_var.is_nan() && {
                let sd = ind.kalman_var.sqrt();
                p < ind.kalman_est - 2.0 * sd && p < ind.sma20
            }
        }
        Comp::Kst => {
            !ind.kst.is_nan() && !ind.kst_signal.is_nan()
                && !ind.kst_prev.is_nan() && !ind.kst_signal_prev.is_nan()
                && ind.kst_prev <= ind.kst_signal_prev && ind.kst > ind.kst_signal
        }
        Comp::None => false,
    }
}

fn is_squeeze(ind: &Ind) -> bool {
    !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0
        && ind.bb_width < ind.bb_width_avg * ISO_SQUEEZE_FACTOR
}

// ── Position ──────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct Position {
    dir:       Dir,
    entry:     f64,
    bars_held: usize,
}

// ── SimResult ─────────────────────────────────────────────────────────────────
struct SimResult {
    daily_pnl:    HashMap<u32, f64>,
    total_trades: usize,
    total_wins:   usize,
    blocked:      usize,
    coin_trades:  Vec<usize>,
    coin_wins:    Vec<usize>,
    coin_pnl:     Vec<f64>,
}

// ── Simulation ────────────────────────────────────────────────────────────────
fn simulate(
    all_inds:     &[Vec<Ind>],
    all_bars:     &[Vec<Bar>],
    all_days:     &[Vec<u32>],
    ordered_days: &[u32],
    upblock:      UpblockCfg,
) -> SimResult {
    let n_bars   = all_inds[0].len();
    let ref_days = &all_days[0];
    let mut daily_pnl: HashMap<u32, f64> = ordered_days.iter().map(|&d| (d, 0.0)).collect();
    let mut total_trades = 0usize;
    let mut total_wins   = 0usize;
    let mut blocked      = 0usize;
    let mut coin_trades  = vec![0usize; N_COINS];
    let mut coin_wins    = vec![0usize; N_COINS];
    let mut coin_pnl     = vec![0.0f64; N_COINS];

    let mut positions: Vec<Option<Position>> = vec![None; N_COINS];
    let mut balances:  Vec<f64>  = vec![INITIAL_BAL; N_COINS];
    let mut cooldowns: Vec<usize> = vec![0; N_COINS];

    let mut current_day = if !ref_days.is_empty() { ref_days[0] } else { 0 };
    let btc_idx = COIN_CFGS.iter().position(|c| c.name == "BTC").unwrap_or(17);

    for bar_i in 1..n_bars {
        let bar_day = ref_days[bar_i];

        // ── Day boundary ─────────────────────────────────────────────────────
        if bar_day != current_day {
            for ci in 0..N_COINS {
                if let Some(ref pos) = positions[ci] {
                    let px = all_bars[ci][bar_i - 1].c;
                    let pf = if pos.dir == Dir::Long {
                        (px / pos.entry - 1.0) - FEE - SLIP
                    } else {
                        -(px / pos.entry - 1.0) - FEE - SLIP
                    };
                    let notional  = balances[ci] * RISK * LEVERAGE;
                    let trade_pnl = pf * notional;
                    *daily_pnl.entry(current_day).or_insert(0.0) += trade_pnl;
                    coin_pnl[ci] += trade_pnl;
                    total_trades += 1; coin_trades[ci] += 1;
                    if pf > 0.0 { total_wins += 1; coin_wins[ci] += 1; }
                    positions[ci] = None;
                }
                balances[ci]  = INITIAL_BAL;
                cooldowns[ci] = 0;
            }
            current_day = bar_day;
        }

        // ── Cross-coin context ────────────────────────────────────────────────
        let mut zs   = Vec::with_capacity(N_COINS);
        let mut rsis = Vec::with_capacity(N_COINS);
        for ci in 0..N_COINS {
            let ind = &all_inds[ci][bar_i];
            if ind.valid && !ind.z.is_nan()     { zs.push(ind.z); }
            if ind.valid && !ind.rsi14.is_nan() { rsis.push(ind.rsi14); }
        }
        let n_valid   = zs.len();
        let n_bearish = zs.iter().filter(|&&z| z < -1.0).count();
        let breadth   = if n_valid > 0 { n_bearish as f64 / n_valid as f64 } else { 0.0 };
        let avg_z     = if n_valid > 0 { zs.iter().sum::<f64>() / n_valid as f64 } else { 0.0 };
        let avg_rsi   = if !rsis.is_empty() { rsis.iter().sum::<f64>() / rsis.len() as f64 } else { 50.0 };
        let btc_z     = all_inds[btc_idx][bar_i].z;
        let mode = if breadth <= BREADTH_LONG_MAX {
            Mode::Long
        } else if breadth >= BREADTH_SHORT_MIN {
            Mode::Short
        } else {
            Mode::IsoShort
        };

        // ── Check exits ───────────────────────────────────────────────────────
        for ci in 0..N_COINS {
            if let Some(ref pos) = positions[ci] {
                let ind   = &all_inds[ci][bar_i];
                if !ind.valid { continue; }
                let price = all_bars[ci][bar_i].c;
                let entry = pos.entry;
                let pnl   = if pos.dir == Dir::Long {
                    (price - entry) / entry
                } else {
                    (entry - price) / entry
                };
                let held     = pos.bars_held;
                let notional = balances[ci] * RISK * LEVERAGE;

                let mut closed = false;
                let mut pf     = 0.0;

                if pnl <= -SL {
                    pf = -(SL + FEE + SLIP);
                    closed = true;
                } else if pnl > 0.0 && held >= MIN_HOLD {
                    let exit_long  = pos.dir == Dir::Long  && (price > ind.sma20 || ind.z > 0.5);
                    let exit_short = pos.dir == Dir::Short && (price < ind.sma20 || ind.z < -0.5);
                    if exit_long || exit_short {
                        pf = pnl - FEE - SLIP;
                        closed = true;
                    }
                }

                if closed {
                    let trade_pnl = pf * notional;
                    *daily_pnl.entry(bar_day).or_insert(0.0) += trade_pnl;
                    coin_pnl[ci] += trade_pnl;
                    total_trades += 1; coin_trades[ci] += 1;
                    if pf > 0.0 { total_wins += 1; coin_wins[ci] += 1; }
                    positions[ci] = None;
                    cooldowns[ci] = 2;
                } else {
                    positions[ci].as_mut().unwrap().bars_held += 1;
                }
            }
            if cooldowns[ci] > 0 { cooldowns[ci] -= 1; }
        }

        // ── Check entries ─────────────────────────────────────────────────────
        if bar_i + 1 >= n_bars { continue; }
        for ci in 0..N_COINS {
            if positions[ci].is_some() || cooldowns[ci] > 0 { continue; }
            let ind = &all_inds[ci][bar_i];
            if !ind.valid { continue; }
            let cfg = &COIN_CFGS[ci];
            if is_squeeze(ind) { continue; }

            let entry_price = all_bars[ci][bar_i + 1].o;
            if entry_price <= 0.0 { continue; }

            let dir: Option<Dir> = match mode {
                Mode::Long => {
                    if long_signal(ind, cfg.long) {
                        Some(Dir::Long)
                    } else if comp_signal(ind, cfg.comp, cfg.comp_z) {
                        Some(Dir::Long)
                    } else if iso_signal(ind, cfg.iso, avg_z, avg_rsi, btc_z) {
                        if upblock.blocks_short(ind) { blocked += 1; None } else { Some(Dir::Short) }
                    } else {
                        None
                    }
                }
                Mode::IsoShort => {
                    if iso_signal(ind, cfg.iso, avg_z, avg_rsi, btc_z) {
                        if upblock.blocks_short(ind) { blocked += 1; None } else { Some(Dir::Short) }
                    } else {
                        None
                    }
                }
                Mode::Short => {
                    if short_signal(ind, cfg.short) {
                        if upblock.blocks_short(ind) { blocked += 1; None } else { Some(Dir::Short) }
                    } else {
                        None
                    }
                }
            };

            if let Some(d) = dir {
                positions[ci] = Some(Position { dir: d, entry: entry_price, bars_held: 0 });
            }
        }
    }

    SimResult { daily_pnl, total_trades, total_wins, blocked, coin_trades, coin_wins, coin_pnl }
}

// ── Serialization ─────────────────────────────────────────────────────────────
#[derive(Serialize)]
struct CoinBreakdown {
    name:               String,
    trades:             usize,
    wins:               usize,
    wr_pct:             f64,
    pnl:                f64,
    delta_baseline_pnl: f64,
}

#[derive(Serialize)]
struct GridResult {
    config:            String,
    total_pnl:         f64,
    delta_baseline:    f64,
    trades:            usize,
    win_rate_pct:      f64,
    positive_days:     usize,
    positive_days_pct: f64,
    blocked:           usize,
    meets_gate:        bool,
    per_coin:          Vec<CoinBreakdown>,
}

#[derive(Serialize)]
struct Run33Output {
    notes:                      String,
    baseline_pnl:               f64,
    baseline_wr_pct:            f64,
    baseline_positive_days:     usize,
    baseline_positive_days_pct: f64,
    baseline_trades:            usize,
    decision_gate:              String,
    results:                    Vec<GridResult>,
}

// ── Entry point ───────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN33 — Uptrend Short Block Filter (v2: comprehensive indicator sweep)");
    eprintln!("Grid: {} combos (1 baseline + {} uptrend filters)", N_GRID, N_GRID - 1);
    eprintln!("Decision gate: ΔP&L > 0 AND positive_days% ≥ 55%");
    eprintln!("Loading 15m data for {} coins...", N_COINS);

    let coin_names: Vec<&str> = COIN_CFGS.iter().map(|c| c.name).collect();
    let mut all_bars_raw: Vec<Vec<Bar>> = Vec::new();
    let mut all_days_raw: Vec<Vec<u32>> = Vec::new();
    for name in &coin_names {
        let (bars, days) = load_15m(name);
        eprintln!("  {} — {} bars", name, bars.len());
        all_bars_raw.push(bars);
        all_days_raw.push(days);
    }

    let min_len = all_bars_raw.iter().map(|b| b.len()).min().unwrap_or(0);
    if min_len < 200 {
        eprintln!("ERROR: insufficient data ({} bars)", min_len);
        return;
    }
    let all_bars: Vec<Vec<Bar>> = all_bars_raw.into_iter()
        .map(|b| b[b.len()-min_len..].to_vec()).collect();
    let all_days: Vec<Vec<u32>> = all_days_raw.into_iter()
        .map(|d| d[d.len()-min_len..].to_vec()).collect();
    eprintln!("Aligned to {} bars (~{:.1} days)", min_len, min_len as f64 / 96.0);

    let mut seen = HashSet::new();
    let mut ordered_days: Vec<u32> = all_days[0].iter()
        .filter(|&&d| d > 0 && seen.insert(d))
        .cloned().collect();
    ordered_days.sort();
    eprintln!("Trading days: {}", ordered_days.len());

    if shutdown.load(Ordering::SeqCst) { return; }

    eprintln!("Computing indicators in parallel (incl. SMA50/100/200, EMA12/26, z50, bullish structure)...");
    let all_inds: Vec<Vec<Ind>> = all_bars.par_iter().enumerate()
        .map(|(ci, bars)| {
            let is_dash = coin_names[ci] == "DASH";
            compute_indicators(bars, is_dash)
        })
        .collect();

    // ── Build grid ────────────────────────────────────────────────────────────
    let grid: Vec<UpblockCfg> = (0..N_GRID as u8).map(|id| UpblockCfg { id }).collect();
    eprintln!("Running {} combos in parallel...", grid.len());

    // ── Parallel simulation ───────────────────────────────────────────────────
    let sims: Vec<SimResult> = grid.par_iter()
        .map(|&upblock| simulate(&all_inds, &all_bars, &all_days, &ordered_days, upblock))
        .collect();

    // ── Extract baseline stats ────────────────────────────────────────────────
    let base_pnls: Vec<f64> = ordered_days.iter()
        .map(|d| sims[0].daily_pnl.get(d).cloned().unwrap_or(0.0))
        .collect();
    let baseline_total   = base_pnls.iter().sum::<f64>();
    let baseline_pos     = base_pnls.iter().filter(|&&p| p >  1e-6).count();
    let baseline_pos_pct = baseline_pos as f64 / ordered_days.len() as f64 * 100.0;
    let baseline_wr      = if sims[0].total_trades > 0 {
        sims[0].total_wins as f64 / sims[0].total_trades as f64 * 100.0
    } else { 0.0 };
    let baseline_coin_pnl = &sims[0].coin_pnl;

    // ── Build combo stats ─────────────────────────────────────────────────────
    struct ComboStats {
        id:           u8,
        label:        &'static str,
        total_pnl:    f64,
        delta:        f64,
        trades:       usize,
        wr_pct:       f64,
        pos_days:     usize,
        pos_days_pct: f64,
        blocked:      usize,
        meets_gate:   bool,
        coin_trades:  Vec<usize>,
        coin_wins:    Vec<usize>,
        coin_pnl:     Vec<f64>,
    }

    let n_days = ordered_days.len() as f64;
    let mut stats: Vec<ComboStats> = grid.iter().zip(sims.iter()).map(|(cfg, sim)| {
        let pnls: Vec<f64> = ordered_days.iter()
            .map(|d| sim.daily_pnl.get(d).cloned().unwrap_or(0.0))
            .collect();
        let total    = pnls.iter().sum::<f64>();
        let pos_days = pnls.iter().filter(|&&p| p > 1e-6).count();
        let pos_pct  = pos_days as f64 / n_days * 100.0;
        let wr = if sim.total_trades > 0 {
            sim.total_wins as f64 / sim.total_trades as f64 * 100.0
        } else { 0.0 };
        let delta = total - baseline_total;
        let gate  = delta > 0.0 && pos_pct >= 55.0;
        ComboStats {
            id: cfg.id, label: cfg.label(),
            total_pnl: total, delta, trades: sim.total_trades,
            wr_pct: wr, pos_days, pos_days_pct: pos_pct, blocked: sim.blocked,
            meets_gate: gate,
            coin_trades: sim.coin_trades.clone(),
            coin_wins:   sim.coin_wins.clone(),
            coin_pnl:    sim.coin_pnl.clone(),
        }
    }).collect();

    // ── Sort: baseline pinned first, rest by TotalPnL desc ───────────────────
    let baseline_stat = stats.remove(0);
    stats.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    // ── Print table ───────────────────────────────────────────────────────────
    println!("\n{:-<99}", "");
    println!("{:<26} {:>10}  {:>8}  {:>6}  {:>5}  {:>5}  {:>7}  {:>7}",
        "Config", "TotalPnL", "ΔBase", "Trades", "WR%", "+Days", "+Days%", "Blocked");
    println!("{:-<99}", "");

    let print_row = |label: &str, s: &ComboStats, marker: &str| {
        println!("{}{:<25} {:>+10.2}  {:>+8.2}  {:>6}  {:>4.1}%  {:>5}  {:>6.1}%  {:>7}",
            marker, label,
            s.total_pnl, s.delta, s.trades, s.wr_pct,
            s.pos_days, s.pos_days_pct, s.blocked);
    };

    print_row(baseline_stat.label, &baseline_stat, "  ");
    println!("{:-<99}", "");
    let mut n_gate = 0usize;
    for s in &stats {
        let marker = if s.meets_gate { "* " } else { "  " };
        if s.meets_gate { n_gate += 1; }
        print_row(s.label, s, marker);
    }
    println!("{:-<99}", "");
    println!("* = meets decision gate (ΔP&L > 0 AND +days% ≥ 55%)   {} of {} combos pass",
        n_gate, stats.len());

    // ── Per-coin breakdown for top 5 ─────────────────────────────────────────
    let top_n = 5.min(stats.len());
    let top: Vec<&ComboStats> = stats.iter().take(top_n).collect();
    if !top.is_empty() {
        println!("\n── Per-coin breakdown (top {} combos by P&L) ─────────────────────────────────", top_n);
        for s in &top {
            println!("\n  [{}]  P&L={:+.2}  ΔBase={:+.2}  Blocked={} (id={})",
                s.label, s.total_pnl, s.delta, s.blocked, s.id);
            println!("  {:<6} {:>6}  {:>5}  {:>5}  {:>9}  {:>9}",
                "Coin", "Trades", "Wins", "WR%", "P&L", "ΔBase");
            let mut ci_order: Vec<usize> = (0..N_COINS).collect();
            ci_order.sort_by(|&a, &b| {
                let da = s.coin_pnl[a] - baseline_coin_pnl[a];
                let db = s.coin_pnl[b] - baseline_coin_pnl[b];
                db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
            });
            for ci in ci_order {
                let wr = if s.coin_trades[ci] > 0 {
                    s.coin_wins[ci] as f64 / s.coin_trades[ci] as f64 * 100.0
                } else { 0.0 };
                let delta_coin = s.coin_pnl[ci] - baseline_coin_pnl[ci];
                println!("  {:<6} {:>6}  {:>5}  {:>4.1}%  {:>+9.2}  {:>+9.2}",
                    COIN_CFGS[ci].name,
                    s.coin_trades[ci], s.coin_wins[ci], wr,
                    s.coin_pnl[ci], delta_coin);
            }
        }
    }

    // ── Summary diagnostics ──────────────────────────────────────────────────
    println!("\n── Blocking effectiveness ───────────────────────────────────────────────────────");
    println!("{:<26} {:>7} {:>8}  {:>10}  {:>10}", "Config", "Blocked", "ΔTrades", "ΔP&L", "$/block");
    println!("{:-<70}", "");
    for s in &stats {
        if s.blocked > 0 {
            let delta_trades = s.trades as i64 - baseline_stat.trades as i64;
            let pnl_per_block = if s.blocked > 0 { s.delta / s.blocked as f64 } else { 0.0 };
            println!("{:<26} {:>7} {:>+8}  {:>+10.2}  {:>+10.3}",
                s.label, s.blocked, delta_trades, s.delta, pnl_per_block);
        }
    }
    let zero_block: Vec<&ComboStats> = stats.iter().filter(|s| s.blocked == 0).collect();
    if !zero_block.is_empty() {
        println!("  ({} filters blocked 0 trades: {})",
            zero_block.len(),
            zero_block.iter().map(|s| s.label).collect::<Vec<_>>().join(", "));
    }

    // ── Save JSON ─────────────────────────────────────────────────────────────
    let mut all_results: Vec<GridResult> = Vec::with_capacity(N_GRID);

    // Baseline first
    let base_coin_bd: Vec<CoinBreakdown> = (0..N_COINS).map(|ci| {
        let wr = if baseline_stat.coin_trades[ci] > 0 {
            baseline_stat.coin_wins[ci] as f64 / baseline_stat.coin_trades[ci] as f64 * 100.0
        } else { 0.0 };
        CoinBreakdown {
            name: COIN_CFGS[ci].name.to_string(),
            trades: baseline_stat.coin_trades[ci], wins: baseline_stat.coin_wins[ci],
            wr_pct: wr, pnl: baseline_stat.coin_pnl[ci], delta_baseline_pnl: 0.0,
        }
    }).collect();
    all_results.push(GridResult {
        config: baseline_stat.label.to_string(),
        total_pnl: baseline_stat.total_pnl,
        delta_baseline: 0.0,
        trades: baseline_stat.trades,
        win_rate_pct: baseline_stat.wr_pct,
        positive_days: baseline_stat.pos_days,
        positive_days_pct: baseline_stat.pos_days_pct,
        blocked: 0,
        meets_gate: false,
        per_coin: base_coin_bd,
    });

    for s in &stats {
        let coin_bd: Vec<CoinBreakdown> = (0..N_COINS).map(|ci| {
            let wr = if s.coin_trades[ci] > 0 {
                s.coin_wins[ci] as f64 / s.coin_trades[ci] as f64 * 100.0
            } else { 0.0 };
            CoinBreakdown {
                name: COIN_CFGS[ci].name.to_string(),
                trades: s.coin_trades[ci], wins: s.coin_wins[ci],
                wr_pct: wr, pnl: s.coin_pnl[ci],
                delta_baseline_pnl: s.coin_pnl[ci] - baseline_coin_pnl[ci],
            }
        }).collect();
        all_results.push(GridResult {
            config: s.label.to_string(),
            total_pnl: s.total_pnl,
            delta_baseline: s.delta,
            trades: s.trades,
            win_rate_pct: s.wr_pct,
            positive_days: s.pos_days,
            positive_days_pct: s.pos_days_pct,
            blocked: s.blocked,
            meets_gate: s.meets_gate,
            per_coin: coin_bd,
        });
    }

    let output = Run33Output {
        notes: format!(
            "RUN33 v2: comprehensive uptrend short block filter. {} coins, {} days. \
             $100/coin/day reset. RISK={}% LEVERAGE={}x FEE={}%/side SLIP={}%/side. \
             29 uptrend indicators tested: MA position (sma50/100/200), MA cross/slope, \
             momentum (ret48/96), z50, RSI zone, bullish structure, and 9 combinations. \
             Complement strategies INCLUDED. Scalp/Momentum EXCLUDED.",
            N_COINS, ordered_days.len(), (RISK*100.0) as u32, LEVERAGE as u32,
            FEE*100.0, SLIP*100.0
        ),
        baseline_pnl: baseline_total,
        baseline_wr_pct: baseline_wr,
        baseline_positive_days: baseline_pos,
        baseline_positive_days_pct: baseline_pos_pct,
        baseline_trades: sims[0].total_trades,
        decision_gate: "delta_baseline > 0 AND positive_days_pct >= 55.0".to_string(),
        results: all_results,
    };

    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run33_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("Saved → {}", path);
}
