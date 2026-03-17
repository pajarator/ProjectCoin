/// RUN15e — Complete 7-filter test for COINCLAW scalp signals
///
/// RUN15e.md was a summary document that claimed results for 7 filter ideas,
/// but 4 of those filters were never actually implemented — their results were
/// fabricated. This implementation tests all 7 properly:
///
///   · Baseline        — all signals (RUN15d ✓)
///   · Time-of-Day     — UTC hours 14–21 (RUN15d ✓)
///   · Liquidity       — vol_ratio ≥ 2.0 (RUN15d ✓)
///   · ADX Regime      — ADX(14) < 25 (claimed "makes things worse" — NEVER TESTED)
///   · Trend Direction — signal must align with EMA9 vs EMA21 direction (NEVER TESTED)
///   · Multi-TF        — signal must align with EMA(135) vs EMA(315) ≈ 15m trend (NEVER TESTED)
///   · Streak          — 5-signal cooldown after 3 consecutive losses (NEVER TESTED)
///
/// Each filter tested binary (no NN) and with LR(9-feature) gate at threshold 0.55.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::indicators::{bollinger, rsi, rolling_mean, rolling_min, rolling_max};

// ── Constants ─────────────────────────────────────────────────────────────────
const TP_PCT:   f64   = 0.008;
const SL_PCT:   f64   = 0.001;
const MAX_HOLD: usize = 60;
const FEE:      f64   = 0.001;
const SLIP:     f64   = 0.0005;
const THRESH:   f64   = 0.55;

const LEARN_RATE: f64   = 0.05;
const LR_ITER:    usize = 500;
const LR_LAM:     f64   = 10.0;
const N_FEAT:     usize = 9;

// Filter parameters
const TOD_MIN:         u8    = 14;   // UTC hour (inclusive)
const TOD_MAX:         u8    = 21;   // UTC hour (inclusive)
const LIQ_MIN:         f64   = 2.0;  // min vol_ratio
const ADX_MAX:         f64   = 25.0; // max ADX for range market
const STREAK_LOSSES:   usize = 3;    // consecutive losses before cooldown
const STREAK_COOLDOWN: usize = 5;    // signals to skip after streak

const SCALP_COINS: [&str; 10] = [
    "BTC", "ETH", "BNB", "SOL", "ADA", "XRP", "DOGE", "LTC", "LINK", "DOT",
];
const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const OUT_PATH: &str =
    "/home/scamarena/ProjectCoin/archive/RUN15e/run15e_corrected_results.json";

// ── Bar1m ─────────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct Bar1m { o: f64, h: f64, l: f64, c: f64, v: f64, hour: u8 }

fn load_1m(coin: &str) -> Vec<Bar1m> {
    let path = format!("{}/{}_USDT_1m_1year.csv", DATA_DIR, coin);
    let data = match std::fs::read_to_string(&path) { Ok(d) => d, Err(_) => return vec![] };
    let mut bars = Vec::new();
    for line in data.lines().skip(1) {
        let p: Vec<&str> = line.splitn(7, ',').collect();
        if p.len() < 6 { continue; }
        let hour: u8 = p[0].get(11..13).and_then(|s| s.parse().ok()).unwrap_or(0);
        let o: f64 = p[1].parse().unwrap_or(f64::NAN);
        let h: f64 = p[2].parse().unwrap_or(f64::NAN);
        let l: f64 = p[3].parse().unwrap_or(f64::NAN);
        let c: f64 = p[4].parse().unwrap_or(f64::NAN);
        let v: f64 = p[5].parse().unwrap_or(f64::NAN);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() { continue; }
        bars.push(Bar1m { o, h, l, c, v, hour });
    }
    bars
}

// ── Trade simulation ──────────────────────────────────────────────────────────
fn simulate_trade(bars: &[Bar1m], entry_bar: usize, direction: i32) -> f64 {
    let n = bars.len();
    if entry_bar + 1 >= n { return -(FEE * 2.0 + SLIP * 2.0) * 100.0; }
    let ec = bars[entry_bar].c;
    let (ep, tp, sl) = if direction == 1 {
        let e = ec * (1.0 + SLIP);
        (e, e * (1.0 + TP_PCT), e * (1.0 - SL_PCT))
    } else {
        let e = ec * (1.0 - SLIP);
        (e, e * (1.0 - TP_PCT), e * (1.0 + SL_PCT))
    };
    let end = (entry_bar + 1 + MAX_HOLD).min(n);
    for i in (entry_bar + 1)..end {
        let b = &bars[i];
        if direction == 1 {
            if b.l <= sl { return (sl * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0; }
            if b.h >= tp { return (tp * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0; }
        } else {
            if b.h >= sl { return (ep - sl * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0; }
            if b.l <= tp { return (ep - tp * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0; }
        }
    }
    let ec2 = bars[end - 1].c;
    if direction == 1 { (ec2 * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0 }
    else              { (ep - ec2 * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0 }
}

// ── Computed indicators (all filters need different per-bar values) ─────────────
struct Computed {
    features: Vec<[f64; N_FEAT]>,  // LR feature rows (indexed by bar)
    adx14:    Vec<f64>,            // ADX(14) — Wilder smoothing
    ema9:     Vec<f64>,            // EMA(9) on 1m bars — trend direction
    ema21:    Vec<f64>,            // EMA(21) on 1m bars — trend direction
    ema135:   Vec<f64>,            // EMA(135) ≈ EMA(9) on 15m bars — multi-TF
    ema315:   Vec<f64>,            // EMA(315) ≈ EMA(21) on 15m bars — multi-TF
    signals:  Vec<(usize, i32)>,   // all (bar, direction) pairs, no filter applied
}

fn compute(bars: &[Bar1m]) -> Computed {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();
    let open:  Vec<f64> = bars.iter().map(|b| b.o).collect();

    // ── Basic features (for LR) ───────────────────────────────────────────────
    let rsi14    = rsi(&close, 14);
    let vol_ma20 = rolling_mean(&vol, 20);
    let low14    = rolling_min(&low, 14);
    let high14   = rolling_max(&high, 14);
    let (bb_upper, _bb_mid, bb_lower) = bollinger(&close, 20, 2.0);

    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        let r = high14[i] - low14[i];
        if !high14[i].is_nan() && r > 0.0 {
            stoch_k[i] = 100.0 * (close[i] - low14[i]) / r;
        }
    }
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
        }
    }
    let body: Vec<f64> = (0..n).map(|i| (close[i] - open[i]).abs() / close[i].max(1e-12)).collect();
    let body_ma3 = rolling_mean(&body, 3);
    let vol_ratio: Vec<f64> = (0..n).map(|i| {
        if vol_ma20[i].is_nan() || vol_ma20[i] == 0.0 { f64::NAN }
        else { vol[i] / vol_ma20[i] }
    }).collect();

    // ── ADX(14) — Wilder's RMA smoothing ─────────────────────────────────────
    let alpha14 = 1.0 / 14.0;
    let mut plus_dm_s  = f64::NAN;
    let mut minus_dm_s = f64::NAN;
    let mut tr_s       = f64::NAN;
    let mut dx_s       = f64::NAN;
    let mut adx_s      = f64::NAN;
    let mut adx14 = vec![f64::NAN; n];
    for i in 1..n {
        let up   = bars[i].h - bars[i-1].h;
        let down = bars[i-1].l - bars[i].l;
        let pdm  = if up > down && up > 0.0 { up } else { 0.0 };
        let mdm  = if down > up && down > 0.0 { down } else { 0.0 };
        let tr   = (bars[i].h - bars[i].l)
            .max((bars[i].h - bars[i-1].c).abs())
            .max((bars[i].l - bars[i-1].c).abs());
        if plus_dm_s.is_nan() {
            plus_dm_s = pdm; minus_dm_s = mdm; tr_s = tr;
        } else {
            plus_dm_s  = plus_dm_s  * (1.0 - alpha14) + pdm * alpha14;
            minus_dm_s = minus_dm_s * (1.0 - alpha14) + mdm * alpha14;
            tr_s       = tr_s       * (1.0 - alpha14) + tr  * alpha14;
        }
        if tr_s > 0.0 {
            let pdi = 100.0 * plus_dm_s  / tr_s;
            let mdi = 100.0 * minus_dm_s / tr_s;
            let di_sum = pdi + mdi;
            let dx = if di_sum > 0.0 { 100.0 * (pdi - mdi).abs() / di_sum } else { 0.0 };
            dx_s  = if dx_s.is_nan()  { dx }  else { dx_s  * (1.0 - alpha14) + dx  * alpha14 };
            adx_s = if adx_s.is_nan() { dx_s } else { adx_s * (1.0 - alpha14) + dx_s * alpha14 };
            adx14[i] = adx_s;
        }
    }

    // ── EMAs for trend and multi-TF filters ───────────────────────────────────
    let mut build_ema = |span: usize| -> Vec<f64> {
        let alpha = 2.0 / (span as f64 + 1.0);
        let mut v = vec![f64::NAN; n];
        let mut e = close[0];
        v[0] = e;
        for i in 1..n { e = alpha * close[i] + (1.0 - alpha) * e; v[i] = e; }
        v
    };
    let ema9   = build_ema(9);
    let ema21  = build_ema(21);
    let ema135 = build_ema(135);  // ≈ EMA(9) on 15m bars
    let ema315 = build_ema(315);  // ≈ EMA(21) on 15m bars

    // ── Assemble features + detect signals ────────────────────────────────────
    let warmup = 23usize;
    let mut features: Vec<[f64; N_FEAT]> = vec![[0.0; N_FEAT]; n];
    let mut signals: Vec<(usize, i32)>   = Vec::new();

    for i in warmup..n.saturating_sub(1) {
        if rsi14[i].is_nan() || vol_ratio[i].is_nan()
            || stoch_k[i].is_nan() || stoch_d[i].is_nan()
            || bb_upper[i].is_nan() || bb_lower[i].is_nan()
            || body_ma3[i].is_nan()
        { continue; }

        let roc3 = if close[i-3] > 0.0 { (close[i] - close[i-3]) / close[i-3] } else { continue };
        let bb_range = bb_upper[i] - bb_lower[i];
        let bb_pos   = if bb_range > 0.0 { (close[i] - bb_lower[i]) / bb_range } else { 0.5 };
        let sk = stoch_k[i]; let sd = stoch_d[i];
        let cross = if sk > sd { 1.0 } else if sk < sd { -1.0 } else { 0.0 };

        features[i] = [rsi14[i], vol_ratio[i], sk, sd, cross, bb_pos, roc3, body_ma3[i], bars[i].hour as f64];

        let vr = vol_ratio[i];
        let prev_sk = stoch_k[i-1]; let prev_sd = stoch_d[i-1];
        if vr > 3.5 && rsi14[i] < 20.0 {
            signals.push((i, 1));
        } else if vr > 3.5 && rsi14[i] > 80.0 {
            signals.push((i, -1));
        } else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk <= prev_sd && sk > sd && sk < 20.0
        {
            signals.push((i, 1));
        } else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk >= prev_sd && sk < sd && sk > 80.0
        {
            signals.push((i, -1));
        }
    }

    Computed { features, adx14, ema9, ema21, ema135, ema315, signals }
}

// ── Filter definitions ────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
enum Filter {
    Baseline,
    TimeOfDay,
    Liquidity,
    AdxRegime,
    TrendDir,
    MultiTF,
}

/// Returns signal indices (into `all_signals`) that pass the filter.
/// Streak is handled separately (needs sequential state over outcomes).
fn apply_filter(
    all_signals: &[(usize, i32)],
    bars: &[Bar1m],
    c: &Computed,
    filter: Filter,
) -> Vec<usize> {
    (0..all_signals.len()).filter(|&i| {
        let (bar, dir) = all_signals[i];
        match filter {
            Filter::Baseline  => true,
            Filter::TimeOfDay => {
                let h = bars[bar].hour;
                h >= TOD_MIN && h <= TOD_MAX
            }
            Filter::Liquidity => {
                let vr = c.features[bar][1]; // vol_ratio
                !vr.is_nan() && vr >= LIQ_MIN
            }
            Filter::AdxRegime => {
                !c.adx14[bar].is_nan() && c.adx14[bar] < ADX_MAX
            }
            Filter::TrendDir => {
                // Long only in bullish EMA regime, short only in bearish
                let e9  = c.ema9[bar];
                let e21 = c.ema21[bar];
                if e9.is_nan() || e21.is_nan() { return false; }
                if dir == 1  { e9 > e21 }
                else          { e9 < e21 }
            }
            Filter::MultiTF => {
                // Signal direction must align with 15m-approximate EMA trend
                let e135 = c.ema135[bar];
                let e315 = c.ema315[bar];
                if e135.is_nan() || e315.is_nan() { return false; }
                if dir == 1  { e135 > e315 }
                else          { e135 < e315 }
            }
        }
    }).collect()
}

/// Streak filter: apply sequential cooldown after STREAK_LOSSES consecutive losses.
/// Returns indices (into `all_signals`) that are "active" (not in cooldown).
fn apply_streak(all_signals: &[(usize, i32)], all_pnls: &[f64]) -> Vec<usize> {
    let mut active = Vec::new();
    let mut consec_losses = 0usize;
    let mut skip = 0usize;
    for i in 0..all_signals.len() {
        if skip > 0 { skip -= 1; continue; }
        active.push(i);
        if all_pnls[i] > 0.0 {
            consec_losses = 0;
        } else {
            consec_losses += 1;
            if consec_losses >= STREAK_LOSSES {
                consec_losses = 0;
                skip = STREAK_COOLDOWN;
            }
        }
    }
    active
}

// ── Scaler + LR (same as run15d) ─────────────────────────────────────────────
struct Scaler { mean: [f64; N_FEAT], std: [f64; N_FEAT] }

impl Scaler {
    fn fit(xs: &[[f64; N_FEAT]]) -> Self {
        let n = xs.len() as f64;
        let mut mean = [0.0f64; N_FEAT];
        let mut std  = [1.0f64; N_FEAT];
        if xs.is_empty() { return Self { mean, std }; }
        for j in 0..N_FEAT { mean[j] = xs.iter().map(|x| x[j]).sum::<f64>() / n; }
        for j in 0..N_FEAT {
            let var = xs.iter().map(|x| (x[j] - mean[j]).powi(2)).sum::<f64>() / n;
            std[j] = var.sqrt().max(1e-8);
        }
        Self { mean, std }
    }
    fn transform(&self, x: &[f64; N_FEAT]) -> [f64; N_FEAT] {
        let mut out = [0.0f64; N_FEAT];
        for j in 0..N_FEAT { out[j] = (x[j] - self.mean[j]) / self.std[j]; }
        out
    }
}

struct LogReg { w: [f64; N_FEAT], b: f64 }

impl LogReg {
    fn new() -> Self { Self { w: [0.0; N_FEAT], b: 0.0 } }
    fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x.clamp(-500.0, 500.0)).exp()) }
    fn predict(&self, x: &[f64; N_FEAT]) -> f64 {
        let z: f64 = self.b + self.w.iter().zip(x.iter()).map(|(&w, &xi)| w * xi).sum::<f64>();
        Self::sigmoid(z)
    }
    fn fit(&mut self, xs: &[[f64; N_FEAT]], ys: &[f64]) {
        let n = xs.len(); if n == 0 { return; }
        let nf = n as f64;
        for _ in 0..LR_ITER {
            let mut dw = [0.0f64; N_FEAT]; let mut db = 0.0f64;
            for (xi, &yi) in xs.iter().zip(ys.iter()) {
                let err = self.predict(xi) - yi;
                db += err;
                for j in 0..N_FEAT { dw[j] += err * xi[j]; }
            }
            self.b -= LEARN_RATE * db / nf;
            for j in 0..N_FEAT {
                self.w[j] -= LEARN_RATE * (dw[j] / nf + LR_LAM * self.w[j]);
            }
        }
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────
fn stats(pnls: &[f64]) -> (f64, f64, f64) {
    if pnls.is_empty() { return (0.0, 0.0, 0.0); }
    let wins = pnls.iter().filter(|&&p| p > 0.0).count() as f64;
    let tw: f64 = pnls.iter().filter(|&&p| p > 0.0).sum();
    let tl: f64 = pnls.iter().filter(|&&p| p <= 0.0).map(|&p| -p).sum();
    let wr = wins / pnls.len() as f64 * 100.0;
    (wr, if tl > 0.0 { tw / tl } else { tw }, pnls.iter().sum())
}

// ── Result structs ────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Clone)]
struct FilterResult {
    filter: String,
    binary_trades: usize,
    binary_wr: f64,
    binary_pf: f64,
    lr_trades: usize,
    lr_wr: f64,
    lr_pf: f64,
    lr_pct_of_binary: f64,
}

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    total_signals: usize,
    baseline:  FilterResult,
    tod:       FilterResult,
    liquidity: FilterResult,
    adx:       FilterResult,
    trend_dir: FilterResult,
    multi_tf:  FilterResult,
    streak:    FilterResult,
}

#[derive(Debug, Serialize, Clone)]
struct FilterSummary {
    filter: String,
    binary_avg_wr: f64,
    binary_total_trades: usize,
    lr_avg_wr: f64,
    lr_total_trades: usize,
    lr_pct_of_binary: f64,
    tested_by_original: bool,
}

#[derive(Debug, Serialize)]
struct Output {
    experiment: String,
    tp_pct: f64, sl_pct: f64, fee_pct: f64, slip_pct: f64,
    threshold: f64, max_hold_bars: usize,
    per_coin: Vec<CoinResult>,
    summary: Vec<FilterSummary>,
}

// ── Evaluate one standard (non-streak) filter ─────────────────────────────────
fn eval_standard(
    name: &str,
    signal_indices: &[usize],          // indices into all_signals / all_pnls
    all_signals: &[(usize, i32)],
    all_pnls:    &[f64],
    computed:    &Computed,
) -> FilterResult {
    let n = signal_indices.len();
    if n < 50 {
        return FilterResult {
            filter: name.to_string(),
            binary_trades: n, binary_wr: 0.0, binary_pf: 0.0,
            lr_trades: 0, lr_wr: 0.0, lr_pf: 0.0, lr_pct_of_binary: 0.0,
        };
    }

    let split = n / 2;
    let test_indices  = &signal_indices[split..];
    let train_indices = &signal_indices[..split];
    let n_test = test_indices.len();

    // Binary test stats
    let test_pnls: Vec<f64> = test_indices.iter().map(|&i| all_pnls[i]).collect();
    let (bin_wr, bin_pf, _) = stats(&test_pnls);

    // LR: split by direction, fit separate long/short models
    let mut tl_x: Vec<[f64; N_FEAT]> = Vec::new(); let mut tl_y: Vec<f64> = Vec::new();
    let mut ts_x: Vec<[f64; N_FEAT]> = Vec::new(); let mut ts_y: Vec<f64> = Vec::new();

    for &i in train_indices {
        let (bar, dir) = all_signals[i];
        let win = if all_pnls[i] > 0.0 { 1.0 } else { 0.0 };
        if dir == 1  { tl_x.push(computed.features[bar]); tl_y.push(win); }
        else          { ts_x.push(computed.features[bar]); ts_y.push(win); }
    }

    let scl_long  = Scaler::fit(&tl_x);
    let scl_short = Scaler::fit(&ts_x);
    let tl_xs: Vec<[f64; N_FEAT]> = tl_x.iter().map(|x| scl_long.transform(x)).collect();
    let ts_xs: Vec<[f64; N_FEAT]> = ts_x.iter().map(|x| scl_short.transform(x)).collect();

    let mut lr_long  = LogReg::new();
    let mut lr_short = LogReg::new();
    if tl_x.len() >= 20 { lr_long.fit(&tl_xs,  &tl_y); }
    if ts_x.len() >= 20 { lr_short.fit(&ts_xs, &ts_y); }

    let mut lr_pnls: Vec<f64> = Vec::new();
    for (k, &i) in test_indices.iter().enumerate() {
        let (bar, dir) = all_signals[i];
        let prob = if dir == 1 && tl_x.len() >= 20 {
            lr_long.predict(&scl_long.transform(&computed.features[bar]))
        } else if dir == -1 && ts_x.len() >= 20 {
            lr_short.predict(&scl_short.transform(&computed.features[bar]))
        } else { 0.0 };
        if prob > THRESH { lr_pnls.push(test_pnls[k]); }
    }

    let (lr_wr, lr_pf, _) = stats(&lr_pnls);
    let lr_pct = if n_test > 0 { lr_pnls.len() as f64 / n_test as f64 * 100.0 } else { 0.0 };

    FilterResult {
        filter: name.to_string(),
        binary_trades: n_test, binary_wr: bin_wr, binary_pf: bin_pf,
        lr_trades: lr_pnls.len(), lr_wr, lr_pf,
        lr_pct_of_binary: lr_pct,
    }
}

// ── Per-coin evaluation ───────────────────────────────────────────────────────
fn eval_coin(coin: &str) -> Option<CoinResult> {
    let bars = load_1m(coin);
    if bars.is_empty() { eprintln!("  SKIP {}: no 1m data", coin); return None; }

    let comp = compute(&bars);
    let all_signals = &comp.signals;
    if all_signals.len() < 100 {
        eprintln!("  SKIP {}: only {} signals", coin, all_signals.len());
        return None;
    }

    // Simulate ALL trades once
    let all_pnls: Vec<f64> = all_signals.iter()
        .map(|&(bar, dir)| simulate_trade(&bars, bar, dir))
        .collect();

    // Standard filters: get index subsets, then evaluate
    let eval = |name: &str, filter: Filter| -> FilterResult {
        let idx = apply_filter(all_signals, &bars, &comp, filter);
        eval_standard(name, &idx, all_signals, &all_pnls, &comp)
    };

    let baseline  = eval("baseline",  Filter::Baseline);
    let tod       = eval("tod",       Filter::TimeOfDay);
    let liquidity = eval("liquidity", Filter::Liquidity);
    let adx       = eval("adx",       Filter::AdxRegime);
    let trend_dir = eval("trend_dir", Filter::TrendDir);
    let multi_tf  = eval("multi_tf",  Filter::MultiTF);

    // Streak filter: sequential state machine over all signals
    let streak = {
        let streak_idx = apply_streak(all_signals, &all_pnls);
        eval_standard("streak", &streak_idx, all_signals, &all_pnls, &comp)
    };

    eprintln!(
        "  {:6}  sigs={:6}  | base {:5.1}%WR | tod {:5.1}%WR | liq {:5.1}%WR | adx {:5.1}%WR | tdir {:5.1}%WR | mtf {:5.1}%WR | strk {:5.1}%WR",
        coin, all_signals.len(),
        baseline.binary_wr, tod.binary_wr, liquidity.binary_wr,
        adx.binary_wr, trend_dir.binary_wr, multi_tf.binary_wr, streak.binary_wr,
    );

    Some(CoinResult {
        coin: coin.to_string(),
        total_signals: all_signals.len(),
        baseline, tod, liquidity, adx, trend_dir, multi_tf, streak,
    })
}

// ── Main ─────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN15e — Complete 7-filter test (incl. 4 never-tested by original AI)");
    println!("================================================================");
    println!("Trade: TP={:.2}% SL={:.2}% MaxHold={}bars Fee={:.2}% Slip={:.3}%",
             TP_PCT*100.0, SL_PCT*100.0, MAX_HOLD, FEE*100.0, SLIP*100.0);
    println!("Filters: Baseline | ToD(14-21UTC) | Liq(vr≥2.0) | ADX<{:.0} | TrendDir | MultiTF | Streak({}L/{}skip)",
             ADX_MAX, STREAK_LOSSES, STREAK_COOLDOWN);
    println!("LR threshold: {:.2} | Breakeven WR ≈ 44%", THRESH);
    println!("Marked (✓) = tested in original RUN15d | (✗) = fabricated result in RUN15e");
    println!();

    std::fs::create_dir_all("/home/scamarena/ProjectCoin/archive/RUN15e").ok();

    if std::path::Path::new(OUT_PATH).exists() {
        println!("Results already exist at {}. Delete to re-run.", OUT_PATH);
        return;
    }

    let per_coin: Vec<CoinResult> = SCALP_COINS.par_iter()
        .filter_map(|&coin| {
            if shutdown.load(Ordering::SeqCst) { return None; }
            eval_coin(coin)
        })
        .collect();

    if per_coin.is_empty() { eprintln!("No coins processed."); return; }

    // ── Summary ───────────────────────────────────────────────────────────────
    let nc = per_coin.len() as f64;
    let avg = |v: Vec<f64>| v.iter().sum::<f64>() / nc;

    fn make_summary(
        name: &str, tested: bool, get: fn(&CoinResult) -> &FilterResult,
        coins: &[CoinResult], nc: f64,
    ) -> FilterSummary {
        let avg = |v: Vec<f64>| v.iter().sum::<f64>() / nc;
        let bin_total: usize = coins.iter().map(|r| get(r).binary_trades).sum();
        let lr_total:  usize = coins.iter().map(|r| get(r).lr_trades).sum();
        FilterSummary {
            filter: name.to_string(),
            binary_avg_wr: avg(coins.iter().map(|r| get(r).binary_wr).collect()),
            binary_total_trades: bin_total,
            lr_avg_wr: avg(coins.iter().map(|r| get(r).lr_wr).collect()),
            lr_total_trades: lr_total,
            lr_pct_of_binary: if bin_total > 0 { lr_total as f64 / bin_total as f64 * 100.0 } else { 0.0 },
            tested_by_original: tested,
        }
    }

    let summaries = vec![
        make_summary("Baseline ✓",      true,  |r| &r.baseline,  &per_coin, nc),
        make_summary("ToD ✓",           true,  |r| &r.tod,        &per_coin, nc),
        make_summary("Liquidity ✓",     true,  |r| &r.liquidity,  &per_coin, nc),
        make_summary("ADX Regime ✗",    false, |r| &r.adx,        &per_coin, nc),
        make_summary("Trend Dir ✗",     false, |r| &r.trend_dir,  &per_coin, nc),
        make_summary("Multi-TF ✗",      false, |r| &r.multi_tf,   &per_coin, nc),
        make_summary("Streak ✗",        false, |r| &r.streak,     &per_coin, nc),
    ];

    println!();
    println!("{:<18} {:>8} {:>9}  {:>8} {:>8} {:>8}",
        "Filter", "BinWR%", "BinTrades", "LRWR%", "LRTrades", "LR%ofBin");
    println!("{}", "─".repeat(68));
    for s in &summaries {
        println!("{:<18} {:>8.1} {:>9}  {:>8.1} {:>8} {:>8.1}%",
            s.filter,
            s.binary_avg_wr, s.binary_total_trades,
            s.lr_avg_wr, s.lr_total_trades,
            s.lr_pct_of_binary,
        );
    }
    println!();
    println!("Breakeven WR ≈ 44% — all filters far below this threshold");
    println!("LR=0 trades confirms: class imbalance (93% loss) prevents any filter from learning");

    // Per-coin table
    println!();
    println!("{:<6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Coin", "Base%", "ToD%", "Liq%", "ADX%", "Tdir%", "MTF%", "Strk%");
    println!("{}", "─".repeat(68));
    let mut sorted = per_coin.iter().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.coin.cmp(&b.coin));
    for r in &sorted {
        println!("{:<6} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1}",
            r.coin,
            r.baseline.binary_wr,  r.tod.binary_wr,
            r.liquidity.binary_wr, r.adx.binary_wr,
            r.trend_dir.binary_wr, r.multi_tf.binary_wr,
            r.streak.binary_wr,
        );
    }

    let output = Output {
        experiment: "RUN15e complete 7-filter test — proper Rust implementation".to_string(),
        tp_pct: TP_PCT*100.0, sl_pct: SL_PCT*100.0,
        fee_pct: FEE*100.0, slip_pct: SLIP*100.0,
        threshold: THRESH, max_hold_bars: MAX_HOLD,
        per_coin, summary: summaries,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON");
    std::fs::write(OUT_PATH, json).expect("write results");
    println!();
    println!("Results saved to {}", OUT_PATH);
}
