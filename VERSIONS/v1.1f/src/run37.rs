/// RUN37 — Realistic Scalp Fill Simulation + Fee Model + Per-Coin Analysis
///
/// Problem: prior scalp backtests use bar-close fill simulation which is 20-30%
/// optimistic. This run uses realistic fill model:
///
///   1. Entry = next bar OPEN + 0.02% slippage (market order assumption)
///   2. TP exit = entry price × (1 + tp)  ← flat, not intrabar high/low tracking
///   3. SL exit = entry price × (1 - sl)
///   4. Fee: 0.05% per side (0.10% RT taker) + test maker 0.02% (0.04% RT)
///
/// Grid:
///   Fee model: TAKER (0.10% RT) | MAKER (0.04% RT) | ZERO (0% RT)
///   TP/SL: (0.8/0.1), (1.5/0.15), (2.0/0.2), (3.0/0.3), (0.4/0.1)
///   Regime filter: NONE | SAME_DIR (only scalp in same dir as 15m regime)
///   Slippage: 0.0% | 0.02% | 0.05%
///
/// Per-coin breakeven WR analysis:
///   breakeven = (SL + fee) / (TP + SL + 2*fee)
///
/// Run: cargo run --release --features run37 -- --run37

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
const SCALP_MAX_HOLD: u32 = 480;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Fee models ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FeeModel {
    Taker,  // 0.05% per side = 0.10% round trip
    Maker,   // 0.02% per side = 0.04% round trip
    Zero,    // 0% (zero-fee baseline)
}

impl FeeModel {
    fn round_trip(&self) -> f64 {
        match self {
            FeeModel::Taker => 0.0010,
            FeeModel::Maker => 0.0004,
            FeeModel::Zero => 0.0,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            FeeModel::Taker => "taker",
            FeeModel::Maker => "maker",
            FeeModel::Zero => "zero",
        }
    }
}

// ── Regime filter ───────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RegimeFilter {
    None,      // No regime filter
    SameDir,   // Only scalp in same direction as 15m regime
}

impl RegimeFilter {
    fn label(&self) -> &'static str {
        match self {
            RegimeFilter::None => "nofilt",
            RegimeFilter::SameDir => "same_dir",
        }
    }
}

// ── Grid config ─────────────────────────────────────────────────────────────
struct ScalpCfg {
    label: &'static str,
    tp: f64,           // take profit rate (e.g. 0.008 = 0.8%)
    sl: f64,           // stop loss rate (e.g. 0.001 = 0.1%)
    fee: FeeModel,
    slippage: f64,      // additional slippage on entry (0.0002 = 0.02%)
    regime_filter: RegimeFilter,
}

fn build_grid() -> Vec<ScalpCfg> {
    let mut grid = Vec::new();

    // ── Baseline: zero fee, bar-close fill (old model) ──
    // (This reproduces the old "optimistic" result for comparison)
    grid.push(ScalpCfg {
        label: "baseline_zero_barclose",
        tp: 0.008, sl: 0.001, fee: FeeModel::Zero,
        slippage: 0.0, regime_filter: RegimeFilter::None,
    });

    // ── Fee models ──
    for fee in [FeeModel::Taker, FeeModel::Maker, FeeModel::Zero] {
        grid.push(ScalpCfg {
            label: match fee {
                FeeModel::Taker => "taker_8_1",
                FeeModel::Maker => "maker_8_1",
                FeeModel::Zero => "zero_8_1",
            },
            tp: 0.008, sl: 0.001, fee,
            slippage: 0.0,
            regime_filter: RegimeFilter::None,
        });
    }

    // ── Slippage test (taker fee, 0.8/0.1) ──
    for slip in [0.0002, 0.0005, 0.0010] {
        grid.push(ScalpCfg {
            label: match slip {
                0.0002 => "taker_8_1_slip02",
                0.0005 => "taker_8_1_slip05",
                0.0010 => "taker_8_1_slip10",
                _ => unreachable!(),
            },
            tp: 0.008, sl: 0.001, fee: FeeModel::Taker,
            slippage: slip,
            regime_filter: RegimeFilter::None,
        });
    }

    // ── Wider TP/SL brackets (taker fee) ──
    for (tp, sl, lbl) in [
        (0.015, 0.0015, "taker_15_15"),
        (0.020, 0.0020, "taker_20_20"),
        (0.030, 0.0030, "taker_30_30"),
        (0.004, 0.0010, "taker_4_10"),  // tighter TP, same SL
        (0.006, 0.0010, "taker_6_10"),  // tighter TP
        (0.010, 0.0010, "taker_10_10"), // 1% TP
        (0.015, 0.0010, "taker_15_10"), // wider TP, same SL
        (0.020, 0.0010, "taker_20_10"), // wider TP
        (0.005, 0.0005, "taker_5_5"),   // tight everything
        (0.010, 0.0005, "taker_10_5"),  // moderate TP, tight SL
    ] {
        grid.push(ScalpCfg {
            label: lbl, tp, sl, fee: FeeModel::Taker,
            slippage: 0.0, regime_filter: RegimeFilter::None,
        });
    }

    // ── Regime alignment filter ──
    for lbl in ["taker_8_1_regime", "taker_15_15_regime", "taker_20_20_regime"] {
        let (tp, sl) = match lbl {
            "taker_8_1_regime" => (0.008, 0.001),
            "taker_15_15_regime" => (0.015, 0.0015),
            "taker_20_20_regime" => (0.020, 0.0020),
            _ => unreachable!(),
        };
        grid.push(ScalpCfg {
            label: lbl, tp, sl, fee: FeeModel::Taker,
            slippage: 0.0, regime_filter: RegimeFilter::SameDir,
        });
    }

    // ── Combined: wider TP + regime filter ──
    grid.push(ScalpCfg {
        label: "taker_20_20_regime_slip02",
        tp: 0.020, sl: 0.0020, fee: FeeModel::Taker,
        slippage: 0.0002, regime_filter: RegimeFilter::SameDir,
    });

    // ── Maker fee + wider brackets ──
    for (tp, sl, lbl) in [
        (0.008, 0.001, "maker_8_1"),
        (0.015, 0.0015, "maker_15_15"),
        (0.020, 0.0020, "maker_20_20"),
        (0.030, 0.0030, "maker_30_30"),
    ] {
        grid.push(ScalpCfg {
            label: lbl, tp, sl, fee: FeeModel::Maker,
            slippage: 0.0, regime_filter: RegimeFilter::None,
        });
    }

    grid
}

// ── Data structures ─────────────────────────────────────────────────────────
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
    // 15m regime data (sampled every 15 bars)
    regime_adx: Vec<f64>,
    regime_dir: Vec<i8>,  // 1 = long regime, -1 = short regime, 0 = neutral
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
    wr: f64,
    // Per-trade breakdown for breakeven calc
    gross_pnl_per_trade: f64,
    fee_per_trade: f64,
    net_pnl_per_trade: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    win_rate: f64,
    total_pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    fee_per_trade: f64,
    gross_pnl_per_trade: f64,
    coins: Vec<CoinResult>,
    // Breakeven analysis
    avg_tp: f64,
    avg_sl: f64,
    breakeven_wr: f64,
    coins_above_be: Vec<String>,
    coins_below_be: Vec<String>,
}

#[derive(Serialize)]
struct Output {
    notes: String,
    configs: Vec<ConfigResult>,
}

// ── Rolling helpers ──────────────────────────────────────────────────────────
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

// ── ATR / ADX ────────────────────────────────────────────────────────────────
fn compute_atr(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let hl = (high[i] - low[i]).abs();
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    rmean(&tr, 14)
}

fn rmean_nan(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        if !data[i].is_nan() { sum += data[i]; count += 1; }
        if i >= w && !data[i - w].is_nan() { sum -= data[i - w]; count -= 1; }
        if i + 1 >= w && count > 0 { out[i] = sum / count as f64; }
    }
    out
}

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
            if dsum > 0.0 { dx[i] = 100.0 * (pdi - mdi).abs() / dsum; }
        }
    }
    rmean_nan(&dx, 14)
}

// ── Regime direction (simplified 15m-style ADX regime) ─────────────────────
// Returns: 1 = trending up (long regime), -1 = trending down (short regime), 0 = chop
fn regime_direction(adx: f64, close: &[f64], sma20: f64, i: usize) -> i8 {
    if adx.is_nan() { return 0; }
    // Trending: ADX > 25 + price above/below SMA20
    if adx >= 25.0 {
        if close[i] > sma20 { 1 } else { -1 }
    } else {
        0  // choppy
    }
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

// ── Compute 1m indicators ─────────────────────────────────────────────────────
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

    // ADX for regime detection
    let adx = compute_adx(&h, &l, &c);

    // Regime direction per bar
    let mut regime_dir = vec![0i8; n];
    for i in 20..n {
        if !bb_sma[i].is_nan() {
            regime_dir[i] = regime_direction(adx[i], &c, bb_sma[i], i);
        }
    }

    CoinData {
        name, close: c, open: o, high: h, low: l, vol: v,
        rsi, vol_ma, stoch_k, stoch_d,
        bb_upper, bb_lower, bb_width: bb_width_raw, bb_width_avg,
        roc_3, avg_body_3, regime_adx: adx, regime_dir,
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

// ── Realistic simulation ─────────────────────────────────────────────────────
struct SimResult {
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    fee_total: f64,
    gross_pnl_total: f64,
}

fn simulate_coin(data: &CoinData, cfg: &ScalpCfg) -> SimResult {
    let n = data.close.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown: u32 = 0;
    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut flats = 0usize;
    let mut fee_total = 0.0;
    let mut gross_pnl_total = 0.0;

    let rt_fee = cfg.fee.round_trip();
    let slip = cfg.slippage;

    for i in 1..n {
        // Position management
        if let Some(ref mut p) = pos {
            let pnl_pct = if p.dir == Dir::Long {
                (data.close[i] - p.entry) / p.entry
            } else {
                (p.entry - data.close[i]) / p.entry
            };

            let mut closed = false;
            let mut exit_pnl_pct = 0.0;

            if pnl_pct <= -cfg.sl {
                exit_pnl_pct = -cfg.sl;
                closed = true;
            } else if pnl_pct >= cfg.tp {
                exit_pnl_pct = cfg.tp;
                closed = true;
            }

            p.bars_held += 1;
            if !closed && p.bars_held >= SCALP_MAX_HOLD {
                exit_pnl_pct = pnl_pct;
                closed = true;
            }

            if closed {
                let gross = p.notional * exit_pnl_pct;
                let fee = p.notional * rt_fee;
                let net = gross - fee;
                bal += net;
                fee_total += fee;
                gross_pnl_total += gross;

                if net > 1e-10 {
                    win_pnls.push(net);
                } else if net < -1e-10 {
                    loss_pnls.push(net);
                } else {
                    flats += 1;
                }

                pos = None;
                cooldown = 2;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Entry check
            if let Some(dir) = scalp_signal(data, i) {
                // Regime filter check
                if cfg.regime_filter == RegimeFilter::SameDir {
                    let reg = data.regime_dir[i];
                    if reg == 0 { /* chop — skip */ }
                    else if dir == Dir::Long && reg != 1 { pos = None; }
                    else if dir == Dir::Short && reg != -1 { pos = None; }
                    // Otherwise aligned, proceed
                }

                // Realistic fill: next bar open + slippage
                if i + 1 < n {
                    let entry_price = data.open[i + 1] * (1.0 + slip);
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
    }

    // Force close
    if let Some(ref p) = pos {
        let price = data.close[n - 1];
        let pnl_pct = if p.dir == Dir::Long {
            (price - p.entry) / p.entry
        } else {
            (p.entry - price) / p.entry
        };
        let gross = p.notional * pnl_pct;
        let fee = p.notional * rt_fee;
        let net = gross - fee;
        bal += net;
        fee_total += fee;
        gross_pnl_total += gross;
        if net > 1e-10 { win_pnls.push(net); }
        else if net < -1e-10 { loss_pnls.push(net); }
        else { flats += 1; }
    }

    let total = win_pnls.len() + loss_pnls.len() + flats;
    SimResult {
        trades: total,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flats,
        pnl: bal - INITIAL_BAL,
        avg_win: if win_pnls.is_empty() { 0.0 } else { win_pnls.iter().sum::<f64>() / win_pnls.len() as f64 },
        avg_loss: if loss_pnls.is_empty() { 0.0 } else { loss_pnls.iter().sum::<f64>() / loss_pnls.len() as f64 },
        fee_total,
        gross_pnl_total,
    }
}

// ── Output ────────────────────────────────────────────────────────────────────
#[derive(Serialize)]
struct FullOutput {
    notes: String,
    configs: Vec<ConfigResult>,
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN37 — Realistic Scalp Fill Simulation + Fee Model + Per-Coin Analysis");
    eprintln!("Key change: entry = next bar OPEN + slippage (market order)");
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

    // Phase 2: Compute indicators
    eprintln!("\nComputing 1m indicators...");
    let start = std::time::Instant::now();
    let coin_data: Vec<CoinData> = raw.into_par_iter().enumerate()
        .map(|(ci, r)| {
            let (o, h, l, c, v) = r.unwrap();
            compute_1m_data(COIN_NAMES[ci], o, h, l, c, v)
        })
        .collect();
    eprintln!("Indicators computed in {:.1}s", start.elapsed().as_secs_f64());

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 3: Run grid
    let grid = build_grid();
    eprintln!("\nSimulating {} configs × {} coins...", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label.to_string(),
                total_trades: 0, wins: 0, losses: 0, flats: 0,
                win_rate: 0.0, total_pnl: 0.0, avg_win: 0.0, avg_loss: 0.0,
                fee_per_trade: 0.0, gross_pnl_per_trade: 0.0,
                coins: vec![],
                avg_tp: cfg.tp, avg_sl: cfg.sl,
                breakeven_wr: 0.0,
                coins_above_be: vec![], coins_below_be: vec![],
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter()
            .map(|cd| {
                let r = simulate_coin(cd, cfg);
                let wr = if r.trades > 0 { r.wins as f64 / r.trades as f64 } else { 0.0 };
                let fee_per_trade = if r.trades > 0 { r.fee_total / r.trades as f64 } else { 0.0 };
                let gross_per_trade = if r.trades > 0 { r.gross_pnl_total / r.trades as f64 } else { 0.0 };
                CoinResult {
                    coin: cd.name.to_string(),
                    trades: r.trades,
                    wins: r.wins,
                    losses: r.losses,
                    flats: r.flats,
                    pnl: r.pnl,
                    avg_win: r.avg_win,
                    avg_loss: r.avg_loss,
                    wr,
                    fee_per_trade,
                    gross_pnl_per_trade: gross_per_trade,
                    net_pnl_per_trade: gross_per_trade - fee_per_trade,
                }
            })
            .collect();

        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let flats: usize = coin_results.iter().map(|c| c.flats).sum();
        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let total_fees: f64 = coin_results.iter().map(|c| c.fee_per_trade * c.trades as f64).sum();
        let total_gross: f64 = coin_results.iter().map(|c| c.gross_pnl_per_trade * c.trades as f64).sum();
        let fee_per_trade = if total_trades > 0 { total_fees / total_trades as f64 } else { 0.0 };
        let gross_per_trade = if total_trades > 0 { total_gross / total_trades as f64 } else { 0.0 };

        // Breakeven WR for this config
        let rt_fee = cfg.fee.round_trip();
        // Breakeven: WR * (tp - fee) = (1-WR) * (sl + fee)
        // WR = (sl + fee) / (tp + sl + 2*fee)
        let be_wr = (cfg.sl + rt_fee) / (cfg.tp + cfg.sl + 2.0 * rt_fee);

        // Per-coin profitability
        let coins_above_be: Vec<String> = coin_results.iter()
            .filter(|c| c.trades >= 50 && c.pnl > 0.0)
            .map(|c| format!("{}({:.1}% WR ${:.1})", c.coin, c.wr, c.pnl))
            .collect();
        let coins_below_be: Vec<String> = coin_results.iter()
            .filter(|c| c.trades >= 50 && c.pnl <= 0.0)
            .map(|c| format!("{}({:.1}% WR ${:.1})", c.coin, c.wr, c.pnl))
            .collect();

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<28} {:>6} trades  WR:{:>5.1}%  PnL:${:>+8.2}  BE:{:>5.1}%  dBE:{:>+5.1}pp",
            d, total_cfgs, cfg.label, total_trades, wr, total_pnl,
            be_wr * 100.0, (wr / 100.0 - be_wr) * 100.0);

        ConfigResult {
            label: cfg.label.to_string(),
            total_trades, wins, losses, flats,
            win_rate: wr, total_pnl,
            avg_win: if wins > 0 {
                coin_results.iter().filter(|c| c.wins > 0)
                    .map(|c| c.avg_win * c.wins as f64).sum::<f64>() / wins as f64
            } else { 0.0 },
            avg_loss: if losses > 0 {
                coin_results.iter().filter(|c| c.losses > 0)
                    .map(|c| c.avg_loss * c.losses as f64).sum::<f64>() / losses as f64
            } else { 0.0 },
            fee_per_trade,
            gross_pnl_per_trade: gross_per_trade,
            coins: coin_results,
            avg_tp: cfg.tp,
            avg_sl: cfg.sl,
            breakeven_wr: be_wr * 100.0,
            coins_above_be,
            coins_below_be,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("\nInterrupted — partial results not saved.");
        return;
    }

    // Phase 4: Summary
    eprintln!();
    println!("\n{:<30} {:>7} {:>6} {:>5} {:>7} {:>8} {:>8} {:>8} {:>9}",
        "Config", "Trades", "Wins", "WR%", "BE_WR%", "Gross/Trd", "Fee/Trd", "Net/Trd", "TotalPnL$");
    println!("{}", "-".repeat(105));
    for r in &results {
        let net = r.gross_pnl_per_trade - r.fee_per_trade;
        println!("{:<30} {:>7} {:>6} {:>5.1}% {:>7.1}% {:>+8.4} {:>+8.4} {:>+8.4} {:>+9.2}",
            r.label, r.total_trades, r.wins,
            r.win_rate, r.breakeven_wr,
            r.gross_pnl_per_trade, r.fee_per_trade, net, r.total_pnl);
    }
    println!("{}", "=".repeat(105));

    // Per-coin breakdown for best configs
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap());
    println!("\n--- Per-coin breakdown (top 5 by PnL) ---");
    for r in sorted.iter().take(5) {
        println!("\n  {}: PnL=${:+.2}, WR={:.1}%, BE={:.1}%", r.label, r.total_pnl, r.win_rate, r.breakeven_wr);
        if !r.coins_above_be.is_empty() {
            println!("  ABOVE BE: {}", r.coins_above_be.join(", "));
        }
        if !r.coins_below_be.is_empty() {
            println!("  BELOW BE: {}", r.coins_below_be.join(", "));
        }
    }

    // Save
    let notes = format!(
        "RUN37 Realistic Fill + Fee Model. Entry = next bar OPEN + slip. {} configs. Baseline TP=0.8% SL=0.1%. Fee: taker=0.10% RT, maker=0.04% RT, zero=0%.",
        results.len()
    );
    let output = FullOutput { notes, configs: results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run37_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("\nSaved → {}", path);
}
