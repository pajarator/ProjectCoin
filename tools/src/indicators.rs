/// Vectorized technical indicators — pure functions, NaN for warmup.

// ── rolling primitives ────────────────────────────────────────────────────

pub fn rolling_sum(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < w { return out; }
    let mut s = 0.0;
    for i in 0..n {
        s += data[i];
        if i >= w { s -= data[i - w]; }
        if i + 1 >= w { out[i] = s; }
    }
    out
}

pub fn rolling_mean(data: &[f64], w: usize) -> Vec<f64> {
    rolling_sum(data, w).into_iter().map(|v| v / w as f64).collect()
}

pub fn rolling_std(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        let s = &data[i + 1 - w..=i];
        let mean = s.iter().sum::<f64>() / w as f64;
        let var = s.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / w as f64;
        out[i] = var.sqrt();
    }
    out
}

pub fn rolling_max(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

pub fn rolling_min(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

// ── moving averages ────────────────────────────────────────────────────────

pub fn sma(data: &[f64], period: usize) -> Vec<f64> {
    rolling_mean(data, period)
}

pub fn ema(data: &[f64], span: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut prev = f64::NAN;
    for i in 0..n {
        if data[i].is_nan() { continue; }
        prev = if prev.is_nan() { data[i] } else { alpha * data[i] + (1.0 - alpha) * prev };
        out[i] = prev;
    }
    out
}

// ── momentum ───────────────────────────────────────────────────────────────

/// RSI using simple rolling average (matches Python indicators.py)
pub fn rsi(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut gains = vec![0.0f64; n];
    let mut losses = vec![0.0f64; n];
    for i in 1..n {
        let d = close[i] - close[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
    }
    let ag = rolling_mean(&gains, period);
    let al = rolling_mean(&losses, period);
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if ag[i].is_nan() || al[i].is_nan() { continue; }
        out[i] = if al[i] == 0.0 {
            if ag[i] == 0.0 { 50.0 } else { 100.0 }
        } else {
            100.0 - 100.0 / (1.0 + ag[i] / al[i])
        };
    }
    out
}

// ── volatility ─────────────────────────────────────────────────────────────

pub fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut tr = vec![0.0f64; n];
    tr[0] = high[0] - low[0];
    for i in 1..n {
        tr[i] = (high[i] - low[i])
            .max((high[i] - close[i - 1]).abs())
            .max((low[i] - close[i - 1]).abs());
    }
    rolling_mean(&tr, period)
}

/// Bollinger Bands: returns (upper, middle, lower)
pub fn bollinger(close: &[f64], period: usize, nstd: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = close.len();
    let mid = sma(close, period);
    let std = rolling_std(close, period);
    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];
    for i in 0..n {
        if !mid[i].is_nan() && !std[i].is_nan() {
            upper[i] = mid[i] + nstd * std[i];
            lower[i] = mid[i] - nstd * std[i];
        }
    }
    (upper, mid, lower)
}

// ── volume ─────────────────────────────────────────────────────────────────

/// Rolling VWAP over `period` bars (typical price weighted)
pub fn vwap_rolling(
    high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: usize,
) -> Vec<f64> {
    let n = close.len();
    let tp: Vec<f64> = (0..n).map(|i| (high[i] + low[i] + close[i]) / 3.0).collect();
    let tpv: Vec<f64> = (0..n).map(|i| tp[i] * volume[i]).collect();
    let sum_tpv = rolling_sum(&tpv, period);
    let sum_v = rolling_sum(volume, period);
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if !sum_tpv[i].is_nan() && !sum_v[i].is_nan() && sum_v[i] > 0.0 {
            out[i] = sum_tpv[i] / sum_v[i];
        }
    }
    out
}

// ── derived ────────────────────────────────────────────────────────────────

/// Z-score of close vs SMA(period)
pub fn z_score(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mean = sma(close, period);
    let std = rolling_std(close, period);
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if !mean[i].is_nan() && !std[i].is_nan() && std[i] > 0.0 {
            out[i] = (close[i] - mean[i]) / std[i];
        }
    }
    out
}
