/// RUN36.1 — Scalp Choppiness Detector Grid Search (1m, 1-year, 18 coins)
///
/// Tests 4 choppiness detection methods × parameter grid against baseline scalping.
/// Hypothesis: pausing scalp entries during choppy regimes improves WR and P&L.
///
/// Run: cargo run --release --features run36 -- --run36
///
/// Grid search over:
///   chop_method: CI | Z_STD | BAYESIAN_WR | CONSEC_LOSS
///   chop_threshold: 0.6 | 0.7 | 0.8  (for BAYESIAN_WR)
///   chop_window: 20 | 40 | 60 bars    (for CI and Z_STD)
///   pause_bars: 5 | 10 | 20            (global pause after chop detected)
///   consec_threshold: 3 | 4 | 5        (for CONSEC_LOSS)
///
/// Baseline: no chop filter (all entries allowed)

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const SCALP_RISK: f64 = 0.05;
const LEVERAGE: f64 = 5.0;
const INITIAL_BAL: f64 = 100.0;

const SCALP_VOL_MULT: f64 = 3.5;
const SCALP_RSI_EXTREME: f64 = 20.0;
const SCALP_STOCH_EXTREME: f64 = 5.0;
const SCALP_BB_SQUEEZE: f64 = 0.4;
const F6_DIR_ROC_3: f64 = -0.195;
const F6_AVG_BODY_3: f64 = 0.072;
const SCALP_SL: f64 = 0.001;
const SCALP_TP: f64 = 0.008;
const SCALP_MAX_HOLD: u32 = 480;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Choppiness methods ────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChopMethod {
    Ci,           // Choppiness Index: ADX+ATR based
    AdxAbs,       // Absolute ADX threshold (low ADX = choppy)
    BayesianWr,   // Bayesian WR regime detection
    ConsecLoss,   // Consecutive loss trigger
}

// ── Grid config ───────────────────────────────────────────────────────────────
struct ChopCfg {
    label: &'static str,
    method: ChopMethod,
    // For CI
    window: usize,
    // For AdxAbs: absolute ADX threshold
    adx_thresh: f64,
    // For Bayesian WR
    bayes_thresh: f64,
    // For ConsecLoss
    consec_thresh: usize,
    // Global pause after chop detected
    pause_bars: usize,
}

fn build_grid() -> Vec<ChopCfg> {
    let mut grid = Vec::new();

    // Baseline: no filter
    grid.push(ChopCfg {
        label: "baseline",
        method: ChopMethod::Ci,
        window: 0,
        adx_thresh: 0.0,
        bayes_thresh: 0.0,
        consec_thresh: 0,
        pause_bars: 0,
    });

    // CI grid
    for window in [20usize, 40, 60] {
        for pause in [5usize, 10, 20] {
            grid.push(ChopCfg {
                label: match (window, pause) {
                    (20, 5) => "ci20_p5",
                    (20, 10) => "ci20_p10",
                    (20, 20) => "ci20_p20",
                    (40, 5) => "ci40_p5",
                    (40, 10) => "ci40_p10",
                    (40, 20) => "ci40_p20",
                    (60, 5) => "ci60_p5",
                    (60, 10) => "ci60_p10",
                    (60, 20) => "ci60_p20",
                    _ => unreachable!(),
                },
                method: ChopMethod::Ci,
                window,
                adx_thresh: 0.0,
                bayes_thresh: 0.0,
                consec_thresh: 0,
                pause_bars: pause,
            });
        }
    }

    // Absolute ADX threshold grid (low ADX = choppy, high ADX = trending)
    // ADX < threshold → choppy (pause scalp entries)
    for thresh in [15.0f64, 20.0, 25.0, 30.0] {
        for pause in [5usize, 10, 20] {
            let label = format!("adx{:.0}_p{}", thresh, pause);
            grid.push(ChopCfg {
                label: Box::leak(label.into_boxed_str()),
                method: ChopMethod::AdxAbs,
                window: 0,
                adx_thresh: thresh,
                bayes_thresh: 0.0,
                consec_thresh: 0,
                pause_bars: pause,
            });
        }
    }

    // Bayesian WR grid
    for thresh in [0.6f64, 0.7, 0.8] {
        for pause in [5usize, 10, 20] {
            grid.push(ChopCfg {
                label: match (thresh, pause) {
                    (0.6, 5) => "bay06_p5",
                    (0.6, 10) => "bay06_p10",
                    (0.6, 20) => "bay06_p20",
                    (0.7, 5) => "bay07_p5",
                    (0.7, 10) => "bay07_p10",
                    (0.7, 20) => "bay07_p20",
                    (0.8, 5) => "bay08_p5",
                    (0.8, 10) => "bay08_p10",
                    (0.8, 20) => "bay08_p20",
                    _ => unreachable!(),
                },
                method: ChopMethod::BayesianWr,
                window: 30,       // rolling window for WR calc
                adx_thresh: 0.0,
                bayes_thresh: thresh,
                consec_thresh: 0,
                pause_bars: pause,
            });
        }
    }

    // Consecutive loss grid
    for consec in [3usize, 4, 5] {
        for pause in [5usize, 10, 20] {
            grid.push(ChopCfg {
                label: match (consec, pause) {
                    (3, 5) => "cs3_p5",
                    (3, 10) => "cs3_p10",
                    (3, 20) => "cs3_p20",
                    (4, 5) => "cs4_p5",
                    (4, 10) => "cs4_p10",
                    (4, 20) => "cs4_p20",
                    (5, 5) => "cs5_p5",
                    (5, 10) => "cs5_p10",
                    (5, 20) => "cs5_p20",
                    _ => unreachable!(),
                },
                method: ChopMethod::ConsecLoss,
                window: 0,
                adx_thresh: 0.0,
                bayes_thresh: 0.0,
                consec_thresh: consec,
                pause_bars: pause,
            });
        }
    }

    grid
}

// ── Data structures ───────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Dir { Long, Short }

struct CoinData {
    name: &'static str,
    close: Vec<f64>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    vol: Vec<f64>,
    rsi: Vec<f64>,
    vol_ma: Vec<f64>,
    stoch_k: Vec<f64>,
    stoch_d: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    bb_width: Vec<f64>,
    bb_width_avg: Vec<f64>,
    roc_3: Vec<f64>,
    avg_body_3: Vec<f64>,
    // For choppiness indicators
    atr: Vec<f64>,
    adx: Vec<f64>,
    sma20: Vec<f64>,
    std20: Vec<f64>,
    z: Vec<f64>,
}

struct ScalpPos {
    dir: Dir,
    entry: f64,
    notional: f64,
    bars_held: u32,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    pause_pct: f64,
    wr: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    method: String,
    total_trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    win_rate: f64,
    total_pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    pause_pct: f64,
    coins: Vec<CoinResult>,
    // vs-baseline delta
    delta_pnl: f64,
    delta_wr: f64,
}

// ── Rolling helpers ───────────────────────────────────────────────────────────
/// Rolling mean — NaN-ignorant version (original behavior)
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

/// Rolling mean — NaN-aware version (skips NaN values in window)
fn rmean_nan(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        if !data[i].is_nan() {
            sum += data[i];
            count += 1;
        }
        if i >= w && !data[i - w].is_nan() {
            sum -= data[i - w];
            count -= 1;
        }
        if i + 1 >= w && count > 0 {
            out[i] = sum / count as f64;
        }
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

fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len();
    let mut out = vec![f64::NAN; n];
    if n < period + 1 { return out; }
    let mut gains = vec![0.0; n];
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

// ── Choppiness Index (CI) ─────────────────────────────────────────────────────
// CI = 100 * log10(sum(ATR, N)) / (log10(N) * (HH - LL) / LL)
// CI > threshold → choppy
fn compute_ci(high: &[f64], low: &[f64], _close: &[f64], atr: &[f64], window: usize) -> Vec<f64> {
    let n = high.len();
    let mut ci = vec![f64::NAN; n];
    if n < window { return ci; }

    for i in (window - 1)..n {
        let mut atr_sum = 0.0;
        for j in (i + 1 - window)..=i {
            atr_sum += atr[j];
        }
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::INFINITY;
        for j in (i + 1 - window)..=i {
            if high[j] > hh { hh = high[j]; }
            if low[j] < ll { ll = low[j]; }
        }
        if ll > 0.0 && atr_sum > 0.0 {
            let range_ratio = (hh - ll) / ll;
            if range_ratio > 1e-10 {
                ci[i] = 100.0 * atr_sum.log10() / ((window as f64).log10() * range_ratio);
            }
        }
    }
    ci
}

// ── ATR computation ────────────────────────────────────────────────────────────
fn compute_atr(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let hl = (high[i] - low[i]).abs();
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    rmean_nan(&tr, 14)
}

// ── ADX computation ────────────────────────────────────────────────────────────
fn compute_adx(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    for i in 1..n {
        let hh = high[i] - high[i - 1];
        let ll = low[i - 1] - low[i];
        if hh > ll && hh > 0.0 { plus_dm[i] = hh; }
        if ll > hh && ll > 0.0 { minus_dm[i] = ll; }
    }
    let atr = compute_atr(high, low, close);
    let pdm_avg = rmean_nan(&plus_dm, 14);
    let mdm_avg = rmean_nan(&minus_dm, 14);
    let mut dx = vec![f64::NAN; n];
    for i in 14..n {
        if atr[i] > 0.0 && !pdm_avg[i].is_nan() && !mdm_avg[i].is_nan() {
            let pdi = 100.0 * pdm_avg[i] / atr[i];
            let mdi = 100.0 * mdm_avg[i] / atr[i];
            let dsum = pdi + mdi;
            if dsum > 0.0 {
                dx[i] = 100.0 * (pdi - mdi).abs() / dsum;
            }
        }
    }
    rmean_nan(&dx, 14)
}

// ── Rolling z-score std dev ───────────────────────────────────────────────────
fn rolling_zstd(z: &[f64], window: usize) -> Vec<f64> {
    rstd(z, window)
}

// ── CSV loader ───────────────────────────────────────────────────────────────
fn load_1m(coin: &str) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_1year.csv", coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return None; }
    };
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    let mut vols = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next();
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if o.is_nan()||h.is_nan()||l.is_nan()||c.is_nan()||v.is_nan() { continue; }
        opens.push(o); highs.push(h); lows.push(l); closes.push(c); vols.push(v);
    }
    if closes.len() < 100 { return None; }
    Some((opens, highs, lows, closes, vols))
}

// ── Compute all 1m indicators ─────────────────────────────────────────────────
fn compute_1m_data(
    name: &'static str,
    o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64>,
) -> CoinData {
    let n = c.len();

    let rsi = rsi_calc(&c, 14);
    let vol_ma = rmean(&v, 20);

    let ll = rmin(&l, 14);
    let hh = rmax(&h, 14);
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !ll[i].is_nan() && !hh[i].is_nan() {
            let range = hh[i] - ll[i];
            if range > 0.0 { stoch_k[i] = 100.0 * (c[i] - ll[i]) / range; }
        }
    }
    let stoch_d = rmean(&stoch_k, 3);

    let bb_sma = rmean(&c, 20);
    let bb_std = rstd(&c, 20);
    let mut bb_upper = vec![f64::NAN; n];
    let mut bb_lower = vec![f64::NAN; n];
    let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !bb_sma[i].is_nan() && !bb_std[i].is_nan() {
            bb_upper[i] = bb_sma[i] + 2.0 * bb_std[i];
            bb_lower[i] = bb_sma[i] - 2.0 * bb_std[i];
            bb_width_raw[i] = bb_upper[i] - bb_lower[i];
        }
    }
    let bb_width_avg = rmean(&bb_width_raw, 20);

    let mut roc_3 = vec![f64::NAN; n];
    for i in 3..n {
        if c[i - 3] > 0.0 { roc_3[i] = (c[i] - c[i - 3]) / c[i - 3] * 100.0; }
    }
    let mut avg_body_3 = vec![f64::NAN; n];
    for i in 2..n {
        let b0 = (c[i] - o[i]).abs() / c[i] * 100.0;
        let b1 = (c[i - 1] - o[i - 1]).abs() / c[i - 1] * 100.0;
        let b2 = (c[i - 2] - o[i - 2]).abs() / c[i - 2] * 100.0;
        avg_body_3[i] = (b0 + b1 + b2) / 3.0;
    }

    let atr = compute_atr(&h, &l, &c);
    let adx = compute_adx(&h, &l, &c);
    let sma20 = rmean(&c, 20);
    let std20 = rstd(&c, 20);
    let mut z = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && std20[i] > 0.0 {
            z[i] = (c[i] - sma20[i]) / std20[i];
        }
    }

    CoinData {
        name, close: c, open: o, high: h, low: l, vol: v,
        rsi, vol_ma, stoch_k, stoch_d,
        bb_upper, bb_lower, bb_width: bb_width_raw, bb_width_avg,
        roc_3, avg_body_3, atr, adx, sma20, std20, z,
    }
}

// ── Entry signal ─────────────────────────────────────────────────────────────
fn f6_pass(data: &CoinData, i: usize, dir: Dir) -> bool {
    if data.roc_3[i].is_nan() || data.avg_body_3[i].is_nan() { return false; }
    let sign = if dir == Dir::Long { 1.0 } else { -1.0 };
    data.roc_3[i] * sign < F6_DIR_ROC_3 && data.avg_body_3[i] > F6_AVG_BODY_3
}

fn scalp_signal(data: &CoinData, i: usize) -> Option<Dir> {
    if i < 40 { return None; }
    if data.vol_ma[i].is_nan() || data.vol_ma[i] <= 0.0 { return None; }
    if data.rsi[i].is_nan() { return None; }

    let vol_r = data.vol[i] / data.vol_ma[i];
    let rsi_lo = SCALP_RSI_EXTREME;
    let rsi_hi = 100.0 - SCALP_RSI_EXTREME;

    // 1. vol_spike_rev
    if vol_r > SCALP_VOL_MULT {
        if data.rsi[i] < rsi_lo && f6_pass(data, i, Dir::Long) {
            return Some(Dir::Long);
        }
        if data.rsi[i] > rsi_hi && f6_pass(data, i, Dir::Short) {
            return Some(Dir::Short);
        }
    }

    // 2. stoch_cross
    if i >= 1 {
        let sk = data.stoch_k[i]; let sd = data.stoch_d[i];
        let skp = data.stoch_k[i - 1]; let sdp = data.stoch_d[i - 1];
        if !sk.is_nan() && !sd.is_nan() && !skp.is_nan() && !sdp.is_nan() {
            let lo = SCALP_STOCH_EXTREME;
            let hi = 100.0 - SCALP_STOCH_EXTREME;
            if skp <= sdp && sk > sd && sk < lo && sd < lo && f6_pass(data, i, Dir::Long) {
                return Some(Dir::Long);
            }
            if skp >= sdp && sk < sd && sk > hi && sd > hi && f6_pass(data, i, Dir::Short) {
                return Some(Dir::Short);
            }
        }
    }

    // 3. bb_squeeze_break
    if !data.bb_width_avg[i].is_nan() && data.bb_width_avg[i] > 0.0
        && !data.bb_upper[i].is_nan()
    {
        let squeeze = data.bb_width[i] < data.bb_width_avg[i] * SCALP_BB_SQUEEZE;
        if squeeze && vol_r > 2.0 {
            if data.close[i] > data.bb_upper[i] { return Some(Dir::Long); }
            if data.close[i] < data.bb_lower[i] { return Some(Dir::Short); }
        }
    }

    None
}

// ── Bayesian WR state ─────────────────────────────────────────────────────────
struct BayesState {
    chop_prob: f64,   // P(choppy | history)
    wins: usize,
    losses: usize,
    wins_in_row: usize,
    losses_in_row: usize,
}

impl BayesState {
    fn new() -> Self {
        // Prior: Beta(2,5) → P(choppy) ≈ 0.286
        BayesState {
            chop_prob: 0.286,
            wins: 0,
            losses: 0,
            wins_in_row: 0,
            losses_in_row: 0,
        }
    }

    // Likelihoods
    const P_WIN_CHOPPY: f64 = 0.25;
    const P_WIN_TRENDING: f64 = 0.55;
    const P_LOSS_CHOPPY: f64 = 0.75;
    const P_LOSS_TRENDING: f64 = 0.45;

    fn update(&mut self, won: bool) {
        if won {
            self.wins += 1;
            self.wins_in_row += 1;
            self.losses_in_row = 0;
        } else {
            self.losses += 1;
            self.losses_in_row += 1;
            self.wins_in_row = 0;
        }

        let p_win_given_chop = Self::P_WIN_CHOPPY;
        let p_win_given_trend = Self::P_WIN_TRENDING;
        let p_loss_given_chop = Self::P_LOSS_CHOPPY;
        let p_loss_given_trend = Self::P_LOSS_TRENDING;

        // Posterior: P(choppy|win) = P(win|choppy)*P(choppy) / evidence
        // Evidence = P(win|choppy)*P(choppy) + P(win|trending)*P(trending)
        let p_chop = self.chop_prob;
        let p_trend = 1.0 - p_chop;

        if won {
            let num = p_win_given_chop * p_chop;
            let denom = num + p_win_given_trend * p_trend;
            if denom > 1e-10 {
                self.chop_prob = num / denom;
            }
        } else {
            let num = p_loss_given_chop * p_chop;
            let denom = num + p_loss_given_trend * p_trend;
            if denom > 1e-10 {
                self.chop_prob = num / denom;
            }
        }
    }

    fn is_choppy(&self, thresh: f64) -> bool {
        self.chop_prob > thresh
    }
}

// ── Simulation ───────────────────────────────────────────────────────────────
struct SimResult {
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    pause_bars: usize,
    total_bars: usize,
}

fn simulate_coin(data: &CoinData, cfg: &ChopCfg, baseline: bool) -> SimResult {
    let n = data.close.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown: u32 = 0;
    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut flats = 0usize;

    // Choppiness state
    let mut ci_vals: Vec<f64> = vec![];
    if cfg.method == ChopMethod::Ci && cfg.window > 0 {
        ci_vals = compute_ci(&data.high, &data.low, &data.close, &data.atr, cfg.window);
    }

    let mut bayes = BayesState::new();
    let mut pause_remaining = 0usize;
    let mut consec_losses = 0usize;
    let mut total_pause_bars = 0usize;

    // For CI: threshold = 70-80 (0.75 default)
    let ci_thresh = 75.0;

    for i in 1..n {
        // Update choppiness state (only for non-baseline)
        let is_choppy = if baseline {
            false
        } else {
            match cfg.method {
                ChopMethod::Ci => {
                    if ci_vals.get(i).map(|v| v.is_nan()).unwrap_or(true) {
                        false
                    } else {
                        ci_vals[i] > ci_thresh
                    }
                }
                ChopMethod::AdxAbs => {
                    // Absolute ADX threshold: ADX < thresh → choppy
                    if data.adx[i].is_nan() {
                        false
                    } else {
                        data.adx[i] < cfg.adx_thresh
                    }
                }
                ChopMethod::BayesianWr => {
                    bayes.is_choppy(cfg.bayes_thresh)
                }
                ChopMethod::ConsecLoss => {
                    consec_losses >= cfg.consec_thresh
                }
            }
        };

        // Manage pause
        if pause_remaining > 0 {
            total_pause_bars += 1;
            pause_remaining -= 1;
        } else if is_choppy && pos.is_none() {
            // Entering chop, pause scalp entries
            pause_remaining = cfg.pause_bars;
        }

        // Position management
        if let Some(ref mut p) = pos {
            let price = data.close[i];
            let pnl_pct = if p.dir == Dir::Long {
                (price - p.entry) / p.entry
            } else {
                (p.entry - price) / p.entry
            };

            let mut closed = false;
            let mut exit_pnl_pct = 0.0;

            if pnl_pct <= -SCALP_SL {
                exit_pnl_pct = -SCALP_SL;
                closed = true;
            } else if pnl_pct >= SCALP_TP {
                exit_pnl_pct = SCALP_TP;
                closed = true;
            }

            p.bars_held += 1;
            if !closed && p.bars_held >= SCALP_MAX_HOLD {
                exit_pnl_pct = pnl_pct;
                closed = true;
            }

            if closed {
                let pnl_dollars = p.notional * exit_pnl_pct;
                bal += pnl_dollars;
                let won = pnl_dollars > 1e-10;
                let lost = pnl_dollars < -1e-10;

                if won {
                    win_pnls.push(pnl_dollars);
                    consec_losses = 0;
                } else if lost {
                    loss_pnls.push(pnl_dollars);
                    consec_losses += 1;
                } else {
                    flats += 1;
                }

                if !baseline {
                    bayes.update(won);
                }

                pos = None;
                cooldown = 2;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else if pause_remaining == 0 || baseline {
            // Entry check
            if let Some(dir) = scalp_signal(data, i) {
                let entry_price = data.close[i];
                if entry_price > 0.0 {
                    pos = Some(ScalpPos {
                        dir,
                        entry: entry_price,
                        notional: bal * SCALP_RISK * LEVERAGE,
                        bars_held: 0,
                    });
                }
            }
        }
    }

    // Force close
    if let Some(ref p) = pos {
        let price = data.close[n - 1];
        let pnl_pct = if p.dir == Dir::Long {
            (price - p.entry) / p.entry
        } else {
            (p.entry - price) / p.entry
        };
        let pnl_dollars = p.notional * pnl_pct;
        bal += pnl_dollars;
        if pnl_dollars > 1e-10 { win_pnls.push(pnl_dollars); }
        else if pnl_dollars < -1e-10 { loss_pnls.push(pnl_dollars); }
        else { flats += 1; }
    }

    let total = win_pnls.len() + loss_pnls.len() + flats;
    SimResult {
        trades: total,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flats,
        pnl: bal - INITIAL_BAL,
        avg_win: if win_pnls.is_empty() { 0.0 }
            else { win_pnls.iter().sum::<f64>() / win_pnls.len() as f64 },
        avg_loss: if loss_pnls.is_empty() { 0.0 }
            else { loss_pnls.iter().sum::<f64>() / loss_pnls.len() as f64 },
        pause_bars: total_pause_bars,
        total_bars: n,
    }
}

// ── Output structures ─────────────────────────────────────────────────────────
#[derive(Serialize)]
struct Output {
    notes: String,
    configs: Vec<ConfigResult>,
}

#[derive(Serialize)]
struct ShadowCoinResult {
    coin: String,
    baseline_trades: usize,
    filter_trades: usize,
    baseline_pnl: f64,
    filter_pnl: f64,
    improvement_pnl: f64,
    baseline_wr: f64,
    filter_wr: f64,
    improvement_wr: f64,
    pause_pct: f64,
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN36.1 — Scalp Choppiness Detector Grid Search (1m, 1-year, 18 coins)");
    eprintln!("Methods: CI (Choppiness Index), ZStd (z-score stability), Bayesian WR, ConsecLoss");
    eprintln!();

    // Phase 1: Load data
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw: Vec<Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref d) = loaded {
            eprintln!("  {} — {} bars", name, d.3.len());
        }
        raw.push(loaded);
    }

    let mut all_ok = true;
    for (i, r) in raw.iter().enumerate() {
        if r.is_none() {
            eprintln!("ERROR: failed to load {}", COIN_NAMES[i]);
            all_ok = false;
        }
    }
    if !all_ok { return; }

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 2: Compute indicators in parallel
    eprintln!("\nComputing 1m indicators...");
    let start = std::time::Instant::now();
    let coin_data: Vec<CoinData> = raw.into_par_iter().enumerate()
        .map(|(ci, r)| {
            let (o, h, l, c, v) = r.unwrap();
            compute_1m_data(COIN_NAMES[ci], o, h, l, c, v)
        })
        .collect();
    eprintln!("Indicators computed in {:.1}s", start.elapsed().as_secs_f64());

    // Debug: check ADX distribution
    {
        eprintln!("[DEBUG] checking ADX for {} coins...", coin_data.len());
        let adx = &coin_data[0].adx;
        let mut vals: Vec<f64> = adx.iter().filter(|v| !v.is_nan()).cloned().collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = vals.len();
        eprintln!("[DEBUG] ADX valid count: {}, total: {}", n, adx.len());
        if n > 0 {
            eprintln!("[DEBUG] ADX: min={:.1}, p5={:.1}, p25={:.1}, p50={:.1}, p75={:.1}, max={:.1}",
                vals[0], vals[n/20], vals[n/4], vals[n/2], vals[3*n/4], vals[n-1]);
            let below_15 = vals.iter().filter(|v| **v < 15.0).count();
            let below_20 = vals.iter().filter(|v| **v < 20.0).count();
            let below_25 = vals.iter().filter(|v| **v < 25.0).count();
            let below_30 = vals.iter().filter(|v| **v < 30.0).count();
            eprintln!("[DEBUG] ADX < 15: {:.1}%, < 20: {:.1}%, < 25: {:.1}%, < 30: {:.1}%",
                100.0 * below_15 as f64 / n as f64,
                100.0 * below_20 as f64 / n as f64,
                100.0 * below_25 as f64 / n as f64,
                100.0 * below_30 as f64 / n as f64);
        } else {
            eprintln!("[DEBUG] ADX: all NaN!");
        }
    }

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 3: Build grid + simulate
    let grid = build_grid();
    let baseline_cfg = ChopCfg {
        label: "baseline",
        method: ChopMethod::Ci,
        window: 0,
        adx_thresh: 0.0,
        bayes_thresh: 0.0,
        consec_thresh: 0,
        pause_bars: 0,
    };

    // Pre-compute baseline for all coins
    eprintln!("\nComputing baseline for all coins...");
    let baseline_coin_results: Vec<SimResult> = coin_data.iter()
        .map(|cd| simulate_coin(cd, &baseline_cfg, true))
        .collect();

    let baseline_total_pnl: f64 = baseline_coin_results.iter().map(|r| r.pnl).sum();
    let baseline_total_wr = if baseline_coin_results.iter().map(|r| r.trades).sum::<usize>() > 0 {
        baseline_coin_results.iter().map(|r| r.wins).sum::<usize>() as f64
            / baseline_coin_results.iter().map(|r| r.trades).sum::<usize>() as f64 * 100.0
    } else { 0.0 };

    eprintln!("Baseline total PnL: ${:+.2}, WR: {:.1}%", baseline_total_pnl, baseline_total_wr);
    eprintln!();

    eprintln!("Simulating {} configs × {} coins...", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label.to_string(),
                method: format!("{:?}", cfg.method),
                total_trades: 0, wins: 0, losses: 0, flats: 0,
                win_rate: 0.0, total_pnl: 0.0, avg_win: 0.0, avg_loss: 0.0,
                pause_pct: 0.0, coins: vec![], delta_pnl: 0.0, delta_wr: 0.0,
            };
        }

        let is_baseline = cfg.label == "baseline";
        let coin_results: Vec<CoinResult> = if is_baseline {
            baseline_coin_results.iter().enumerate().map(|(ci, r)| {
                CoinResult {
                    coin: coin_data[ci].name.to_string(),
                    trades: r.trades,
                    wins: r.wins,
                    losses: r.losses,
                    flats: r.flats,
                    pnl: r.pnl,
                    avg_win: r.avg_win,
                    avg_loss: r.avg_loss,
                    pause_pct: 0.0,
                    wr: if r.trades > 0 { r.wins as f64 / r.trades as f64 * 100.0 } else { 0.0 },
                }
            }).collect()
        } else {
            coin_data.iter()
                .map(|cd| {
                    let r = simulate_coin(cd, cfg, false);
                    CoinResult {
                        coin: cd.name.to_string(),
                        trades: r.trades,
                        wins: r.wins,
                        losses: r.losses,
                        flats: r.flats,
                        pnl: r.pnl,
                        avg_win: r.avg_win,
                        avg_loss: r.avg_loss,
                        pause_pct: if r.total_bars > 0 {
                            r.pause_bars as f64 / r.total_bars as f64 * 100.0
                        } else { 0.0 },
                        wr: if r.trades > 0 { r.wins as f64 / r.trades as f64 * 100.0 } else { 0.0 },
                    }
                })
                .collect()
        };

        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let flats: usize = coin_results.iter().map(|c| c.flats).sum();
        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let pause_pct = coin_results.iter().map(|c| c.pause_pct).sum::<f64>() / N_COINS as f64;

        // Delta vs baseline
        let delta_pnl = total_pnl - baseline_total_pnl;
        let delta_wr = wr - baseline_total_wr;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<12} {:>6} trades  WR:{:>5.1}%  PnL:${:>+8.2}  ΔWR:{:>+5.1}pp  ΔPnL:${:>+8.2}  pause:{:>4.1}%",
            d, total_cfgs, cfg.label, total_trades, wr, total_pnl, delta_wr, delta_pnl, pause_pct);

        ConfigResult {
            label: cfg.label.to_string(),
            method: format!("{:?}", cfg.method),
            total_trades, wins, losses, flats,
            win_rate: wr, total_pnl,
            avg_win: if wins > 0 {
                coin_results.iter().filter(|c| c.wins > 0).map(|c| c.avg_win * c.wins as f64).sum::<f64>() / wins as f64
            } else { 0.0 },
            avg_loss: if losses > 0 {
                coin_results.iter().filter(|c| c.losses > 0).map(|c| c.avg_loss * c.losses as f64).sum::<f64>() / losses as f64
            } else { 0.0 },
            pause_pct,
            coins: coin_results,
            delta_pnl,
            delta_wr,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("\nInterrupted — partial results not saved.");
        return;
    }

    // Phase 4: Print summary
    eprintln!();
    println!("\n{:<14} {:>7} {:>6} {:>6} {:>5} {:>6} {:>8} {:>8} {:>10} {:>8} {:>8}",
        "Config", "Trades", "Wins", "Loss", "Flat", "WR%", "AvgWin", "AvgLoss", "Total$", "ΔWR", "ΔPnL$");
    println!("{}", "-".repeat(110));
    for r in &results {
        println!("{:<14} {:>7} {:>6} {:>6} {:>5} {:>6.1}% ${:>+7.4} ${:>+7.4} ${:>+9.2} {:>+7.1}pp ${:>+8.2}",
            r.label, r.total_trades, r.wins, r.losses, r.flats,
            r.win_rate, r.avg_win, r.avg_loss, r.total_pnl, r.delta_wr, r.delta_pnl);
    }
    println!("{}", "=".repeat(110));

    // Find best by PnL
    let best_pnl = results.iter().max_by(|a, b| a.total_pnl.partial_cmp(&b.total_pnl).unwrap());
    let best_wr = results.iter().max_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap());

    if let Some(b) = best_pnl {
        println!("\nBest PnL: {} — ${:+.2}, {:.1}% WR, ΔWR:{:+.1}pp, ΔPnL: ${:+.2}",
            b.label, b.total_pnl, b.win_rate, b.delta_wr, b.delta_pnl);
    }
    if let Some(b) = best_wr {
        println!("Best WR:  {} — {:.1}% WR, ${:+.2} PnL",
            b.label, b.win_rate, b.total_pnl);
    }

    // Per-coin breakdown for top 5 + baseline
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap());

    println!("\n--- Per-coin breakdown (top 5 + baseline) ---");
    let baseline_r = results.iter().find(|r| r.label == "baseline");
    let show: Vec<&ConfigResult> = sorted.iter().take(5).cloned()
        .chain(baseline_r.iter().cloned())
        .collect();

    for r in &show {
        println!("\n  {}: PnL=${:+.2}, WR={:.1}%, pause={:.1}%", r.label, r.total_pnl, r.win_rate, r.pause_pct);
        println!("  {:<6} {:>5} {:>4} {:>4} {:>4} {:>8} {:>7}",
            "Coin", "Trd", "Win", "Los", "Flt", "PnL$", "WR%");
        for c in &r.coins {
            println!("  {:<6} {:>5} {:>4} {:>4} {:>4} ${:>+7.3} {:>6.1}%",
                c.coin, c.trades, c.wins, c.losses, c.flats, c.pnl, c.wr);
        }
    }

    // Phase 5: Shadow analysis — WHY does the filter help/hurt?
    println!("\n--- Shadow analysis: per-coin contribution to ΔPnL ---");
    let best = sorted.first();
    if let Some(b) = best {
        if b.label != "baseline" {
            println!("\n  {} vs baseline:", b.label);
            println!("  {:<6} {:>8} {:>8} {:>8} {:>8}",
                "Coin", "BasePnL", "FiltPnL", "ΔPnL", "FiltTrades");
            for (ci, cr) in b.coins.iter().enumerate() {
                let br = &baseline_r.unwrap().coins[ci];
                let delta = cr.pnl - br.pnl;
                println!("  {:<6} ${:>+8.3} ${:>+8.3} ${:>+8.3} {:>5}",
                    cr.coin, br.pnl, cr.pnl, delta, cr.trades);
            }
        }
    }

    // Save JSON
    let notes = format!(
        "RUN36.1 Scalp Choppiness Detector Grid Search. {} coins, 1-year 1m data. \
         Zero-fee. SCALP_RISK={}% LEVERAGE={}x. \
         {} configs tested. Baseline PnL=${:.2} WR={:.1}%.",
        N_COINS, (SCALP_RISK * 100.0) as u32, LEVERAGE as u32,
        results.len(), baseline_total_pnl, baseline_total_wr
    );
    let output = Output { notes, configs: results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run36_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("\nSaved → {}", path);
}
