/// Precomputed indicator vectors for RUN11b (15 new indicator strategies).
/// All vectors have the same length as the candle array.
/// NAN means "not yet valid" (insufficient lookback).

pub struct Indicators {
    // Base (needed by exit logic and z-filter)
    pub sma20: Vec<f64>,
    pub std20: Vec<f64>,
    pub z_score: Vec<f64>,
    pub vol_ratio: Vec<f64>,

    // Aroon (19)
    pub aroon_up: Vec<f64>,
    pub aroon_down: Vec<f64>,

    // Hull MA (20)
    pub hull_ma: Vec<f64>,
    pub hull_ma_prev: Vec<f64>, // for crossover detection

    // KAMA (21)
    pub kama: Vec<f64>,

    // TRIX (22)
    pub trix: Vec<f64>,
    pub trix_signal: Vec<f64>,

    // Vortex (23)
    pub vi_plus: Vec<f64>,
    pub vi_minus: Vec<f64>,

    // Elder Ray (24)
    pub bull_power: Vec<f64>,
    pub bear_power: Vec<f64>,
    pub ema13: Vec<f64>,

    // Awesome Oscillator (25)
    pub ao: Vec<f64>,

    // Heikin Ashi (26)
    pub ha_open: Vec<f64>,
    pub ha_close: Vec<f64>,

    // Bollinger + Keltner Squeeze (27)
    pub bb_upper: Vec<f64>,
    pub bb_lower: Vec<f64>,
    pub kc_upper: Vec<f64>,
    pub kc_lower: Vec<f64>,
    pub squeeze: Vec<bool>,  // BB inside KC = squeeze active

    // ATR (28, 30, 31)
    pub atr14: Vec<f64>,

    // Donchian (29)
    pub dc_upper: Vec<f64>,
    pub dc_lower: Vec<f64>,
    pub dc_mid: Vec<f64>,

    // Range stats (30, 32)
    pub avg_range7: Vec<f64>,
    pub avg_range20: Vec<f64>,

    // Linear Regression (33)
    pub linreg: Vec<f64>,
    pub linreg_upper: Vec<f64>,
    pub linreg_lower: Vec<f64>,
}

// ---- Rolling helpers ----

pub fn rolling_mean(data: &[f64], window: usize) -> Vec<f64> {
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
        if i + 1 >= window && count == window {
            out[i] = sum / window as f64;
        }
    }
    out
}

fn rolling_std(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < window { return out; }
    for i in (window - 1)..n {
        let slice = &data[i + 1 - window..=i];
        let mut s = 0.0;
        let mut s2 = 0.0;
        let mut cnt = 0usize;
        for &v in slice {
            if !v.is_nan() { s += v; s2 += v * v; cnt += 1; }
        }
        if cnt == window {
            let mean = s / cnt as f64;
            let var = s2 / cnt as f64 - mean * mean;
            out[i] = if var > 0.0 { var.sqrt() } else { 0.0 };
        }
    }
    out
}

fn rolling_min(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < window { return out; }
    for i in (window - 1)..n {
        let mut m = f64::INFINITY;
        for j in (i + 1 - window)..=i {
            if !data[j].is_nan() && data[j] < m { m = data[j]; }
        }
        if m.is_finite() { out[i] = m; }
    }
    out
}

fn rolling_max(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < window { return out; }
    for i in (window - 1)..n {
        let mut m = f64::NEG_INFINITY;
        for j in (i + 1 - window)..=i {
            if !data[j].is_nan() && data[j] > m { m = data[j]; }
        }
        if m.is_finite() { out[i] = m; }
    }
    out
}

fn compute_ema(data: &[f64], span: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n == 0 { return out; }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut started = false;
    for i in 0..n {
        if data[i].is_nan() { continue; }
        if !started {
            out[i] = data[i];
            started = true;
        } else {
            let prev = if i > 0 && !out[i - 1].is_nan() { out[i - 1] } else { data[i] };
            out[i] = alpha * data[i] + (1.0 - alpha) * prev;
        }
    }
    out
}

fn compute_wma(data: &[f64], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < period { return out; }
    let denom: f64 = (period * (period + 1) / 2) as f64;
    for i in (period - 1)..n {
        let mut sum = 0.0;
        let mut valid = true;
        for j in 0..period {
            let val = data[i - period + 1 + j];
            if val.is_nan() { valid = false; break; }
            sum += val * (j + 1) as f64;
        }
        if valid {
            out[i] = sum / denom;
        }
    }
    out
}

/// Compute ATR (Average True Range)
fn compute_atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut tr = vec![f64::NAN; n];
    if n == 0 { return tr; }
    tr[0] = high[0] - low[0];
    for i in 1..n {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    rolling_mean(&tr, period)
}

/// Linear regression over a rolling window. Returns (value, slope) at each point.
fn rolling_linreg(data: &[f64], window: usize) -> (Vec<f64>, Vec<f64>) {
    let n = data.len();
    let mut val = vec![f64::NAN; n];
    let mut slope = vec![f64::NAN; n];
    if n < window { return (val, slope); }

    let w = window as f64;
    // Precomputed: sum_x = 0+1+...+(w-1), sum_x2 = sum of squares
    let sum_x: f64 = (0..window).map(|x| x as f64).sum();
    let sum_x2: f64 = (0..window).map(|x| (x * x) as f64).sum();

    for i in (window - 1)..n {
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut valid = true;
        for j in 0..window {
            let y = data[i - window + 1 + j];
            if y.is_nan() { valid = false; break; }
            sum_y += y;
            sum_xy += j as f64 * y;
        }
        if !valid { continue; }
        let denom = w * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-15 { continue; }
        let b = (w * sum_xy - sum_x * sum_y) / denom;
        let a = (sum_y - b * sum_x) / w;
        // Value at end of window (x = window-1)
        val[i] = a + b * (w - 1.0);
        slope[i] = b;
    }
    (val, slope)
}

/// Compute all indicators from OHLCV arrays.
pub fn compute_all(
    open: &[f64], high: &[f64], low: &[f64], close: &[f64], volume: &[f64],
) -> Indicators {
    let n = close.len();

    // Base: SMA20, std20, z-score, vol_ratio
    let sma20 = rolling_mean(close, 20);
    let std20 = rolling_std(close, 20);
    let mut z_score = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            z_score[i] = (close[i] - sma20[i]) / std20[i];
        }
    }
    let vol_ma20 = rolling_mean(volume, 20);
    let mut vol_ratio = vec![f64::NAN; n];
    for i in 0..n {
        if !vol_ma20[i].is_nan() && vol_ma20[i] > 0.0 {
            vol_ratio[i] = volume[i] / vol_ma20[i];
        }
    }

    // 19. Aroon Up/Down (25-period)
    let aroon_period = 25usize;
    let mut aroon_up = vec![f64::NAN; n];
    let mut aroon_down = vec![f64::NAN; n];
    for i in aroon_period..n {
        let mut hi_idx = 0usize;
        let mut lo_idx = 0usize;
        let mut hi_val = f64::NEG_INFINITY;
        let mut lo_val = f64::INFINITY;
        for j in 0..=aroon_period {
            let idx = i - aroon_period + j;
            if high[idx] > hi_val { hi_val = high[idx]; hi_idx = j; }
            if low[idx] < lo_val { lo_val = low[idx]; lo_idx = j; }
        }
        aroon_up[i] = 100.0 * hi_idx as f64 / aroon_period as f64;
        aroon_down[i] = 100.0 * lo_idx as f64 / aroon_period as f64;
    }

    // 20. Hull MA (period 16)
    // HMA = WMA(2*WMA(n/2) - WMA(n), sqrt(n))
    let hull_period = 16usize;
    let wma_half = compute_wma(close, hull_period / 2);
    let wma_full = compute_wma(close, hull_period);
    let mut hull_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !wma_half[i].is_nan() && !wma_full[i].is_nan() {
            hull_raw[i] = 2.0 * wma_half[i] - wma_full[i];
        }
    }
    let hull_sqrt = (hull_period as f64).sqrt() as usize;
    let hull_ma = compute_wma(&hull_raw, hull_sqrt);
    let mut hull_ma_prev = vec![f64::NAN; n];
    for i in 1..n {
        hull_ma_prev[i] = hull_ma[i - 1];
    }

    // 21. KAMA (Kaufman Adaptive MA, period 10, fast=2, slow=30)
    let kama = compute_kama(close, 10, 2, 30);

    // 22. TRIX (triple-smoothed EMA, period 15)
    let ema1 = compute_ema(close, 15);
    let ema2 = compute_ema(&ema1, 15);
    let ema3 = compute_ema(&ema2, 15);
    let mut trix = vec![f64::NAN; n];
    for i in 1..n {
        if !ema3[i].is_nan() && !ema3[i - 1].is_nan() && ema3[i - 1] != 0.0 {
            trix[i] = (ema3[i] - ema3[i - 1]) / ema3[i - 1] * 10000.0; // basis points
        }
    }
    let trix_signal = rolling_mean(&trix, 9);

    // 23. Vortex Indicator (14-period)
    let vortex_period = 14usize;
    let mut vi_plus = vec![f64::NAN; n];
    let mut vi_minus = vec![f64::NAN; n];
    if n > vortex_period {
        let mut vm_plus = vec![0.0; n];
        let mut vm_minus = vec![0.0; n];
        let mut tr = vec![0.0; n];
        for i in 1..n {
            vm_plus[i] = (high[i] - low[i - 1]).abs();
            vm_minus[i] = (low[i] - high[i - 1]).abs();
            let hl = high[i] - low[i];
            let hc = (high[i] - close[i - 1]).abs();
            let lc = (low[i] - close[i - 1]).abs();
            tr[i] = hl.max(hc).max(lc);
        }
        let vm_plus_sum = rolling_sum(&vm_plus, vortex_period);
        let vm_minus_sum = rolling_sum(&vm_minus, vortex_period);
        let tr_sum = rolling_sum(&tr, vortex_period);
        for i in 0..n {
            if !vm_plus_sum[i].is_nan() && !tr_sum[i].is_nan() && tr_sum[i] > 0.0 {
                vi_plus[i] = vm_plus_sum[i] / tr_sum[i];
                vi_minus[i] = vm_minus_sum[i] / tr_sum[i];
            }
        }
    }

    // 24. Elder Ray (13-period EMA, bull/bear power)
    let ema13 = compute_ema(close, 13);
    let mut bull_power = vec![f64::NAN; n];
    let mut bear_power = vec![f64::NAN; n];
    for i in 0..n {
        if !ema13[i].is_nan() {
            bull_power[i] = high[i] - ema13[i];
            bear_power[i] = low[i] - ema13[i];
        }
    }

    // 25. Awesome Oscillator: SMA5(median) - SMA34(median)
    let mut median = vec![f64::NAN; n];
    for i in 0..n {
        median[i] = (high[i] + low[i]) / 2.0;
    }
    let sma5_med = rolling_mean(&median, 5);
    let sma34_med = rolling_mean(&median, 34);
    let mut ao = vec![f64::NAN; n];
    for i in 0..n {
        if !sma5_med[i].is_nan() && !sma34_med[i].is_nan() {
            ao[i] = sma5_med[i] - sma34_med[i];
        }
    }

    // 26. Heikin Ashi
    let mut ha_open = vec![0.0; n];
    let mut ha_close = vec![0.0; n];
    ha_open[0] = open[0];
    ha_close[0] = (open[0] + high[0] + low[0] + close[0]) / 4.0;
    for i in 1..n {
        ha_open[i] = (ha_open[i - 1] + ha_close[i - 1]) / 2.0;
        ha_close[i] = (open[i] + high[i] + low[i] + close[i]) / 4.0;
    }

    // 27. Bollinger Bands + Keltner Channels (squeeze detection)
    let bb_upper_v = vec![f64::NAN; n];
    let bb_lower_v = vec![f64::NAN; n];
    let mut bb_upper = bb_upper_v;
    let mut bb_lower = bb_lower_v;
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() {
            bb_upper[i] = sma20[i] + 2.0 * std20[i];
            bb_lower[i] = sma20[i] - 2.0 * std20[i];
        }
    }
    let atr14 = compute_atr(high, low, close, 14);
    let ema20 = compute_ema(close, 20);
    let mut kc_upper = vec![f64::NAN; n];
    let mut kc_lower = vec![f64::NAN; n];
    let mut squeeze = vec![false; n];
    for i in 0..n {
        if !ema20[i].is_nan() && !atr14[i].is_nan() {
            kc_upper[i] = ema20[i] + 1.5 * atr14[i];
            kc_lower[i] = ema20[i] - 1.5 * atr14[i];
            // Squeeze: BB inside KC
            if !bb_upper[i].is_nan() && !bb_lower[i].is_nan() {
                squeeze[i] = bb_lower[i] > kc_lower[i] && bb_upper[i] < kc_upper[i];
            }
        }
    }

    // 29. Donchian Channel (20-period)
    let dc_upper = rolling_max(high, 20);
    let dc_lower = rolling_min(low, 20);
    let mut dc_mid = vec![f64::NAN; n];
    for i in 0..n {
        if !dc_upper[i].is_nan() && !dc_lower[i].is_nan() {
            dc_mid[i] = (dc_upper[i] + dc_lower[i]) / 2.0;
        }
    }

    // Range stats
    let mut range_vec = vec![0.0; n];
    for i in 0..n {
        range_vec[i] = high[i] - low[i];
    }
    let avg_range7 = rolling_mean(&range_vec, 7);
    let avg_range20 = rolling_mean(&range_vec, 20);

    // 33. Linear Regression Channel (20-period)
    let (linreg, _linreg_slope) = rolling_linreg(close, 20);
    let linreg_std = rolling_std(close, 20); // reuse std20
    let mut linreg_upper = vec![f64::NAN; n];
    let mut linreg_lower = vec![f64::NAN; n];
    for i in 0..n {
        if !linreg[i].is_nan() && !linreg_std[i].is_nan() {
            linreg_upper[i] = linreg[i] + 2.0 * linreg_std[i];
            linreg_lower[i] = linreg[i] - 2.0 * linreg_std[i];
        }
    }

    Indicators {
        sma20, std20, z_score, vol_ratio,
        aroon_up, aroon_down,
        hull_ma, hull_ma_prev,
        kama,
        trix, trix_signal,
        vi_plus, vi_minus,
        bull_power, bear_power, ema13,
        ao,
        ha_open, ha_close,
        bb_upper, bb_lower, kc_upper, kc_lower, squeeze,
        atr14,
        dc_upper, dc_lower, dc_mid,
        avg_range7, avg_range20,
        linreg, linreg_upper, linreg_lower,
    }
}

fn rolling_sum(data: &[f64], window: usize) -> Vec<f64> {
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
        if i + 1 >= window && count == window {
            out[i] = sum;
        }
    }
    out
}

/// KAMA: Kaufman Adaptive Moving Average
fn compute_kama(close: &[f64], er_period: usize, fast_sc: usize, slow_sc: usize) -> Vec<f64> {
    let n = close.len();
    let mut kama = vec![f64::NAN; n];
    if n < er_period + 1 { return kama; }

    let fast_alpha = 2.0 / (fast_sc as f64 + 1.0);
    let slow_alpha = 2.0 / (slow_sc as f64 + 1.0);

    // First KAMA value = close at er_period
    kama[er_period] = close[er_period];

    for i in (er_period + 1)..n {
        // Efficiency Ratio = direction / volatility
        let direction = (close[i] - close[i - er_period]).abs();
        let mut volatility = 0.0;
        for j in (i - er_period + 1)..=i {
            volatility += (close[j] - close[j - 1]).abs();
        }
        let er = if volatility > 0.0 { direction / volatility } else { 0.0 };
        // Smoothing constant: sc = (ER * (fast - slow) + slow)^2
        let sc = (er * (fast_alpha - slow_alpha) + slow_alpha).powi(2);
        let prev = if !kama[i - 1].is_nan() { kama[i - 1] } else { close[i] };
        kama[i] = prev + sc * (close[i] - prev);
    }
    kama
}
