/// Indicators for RUN13 — STRAT1000 remaining strategies.

pub struct Indicators {
    // Base (reused from run11c)
    pub sma20: Vec<f64>,
    pub std20: Vec<f64>,
    pub z_score: Vec<f64>,
    pub vol_ratio: Vec<f64>,

    // Candle pattern stats
    pub avg_body20: Vec<f64>,
    pub avg_range20: Vec<f64>,

    // QQE (Quantitative Qualitative Estimation)
    pub qqe_rsi_smooth: Vec<f64>,  // EMA(RSI(14), 5)
    pub qqe_upper: Vec<f64>,       // smoothed RSI + atr_rsi * mult
    pub qqe_lower: Vec<f64>,       // smoothed RSI - atr_rsi * mult
    pub qqe_upper_tight: Vec<f64>, // mult=3.0 variant
    pub qqe_lower_tight: Vec<f64>,

    // Laguerre RSI (multiple gamma values)
    pub laguerre_rsi_05: Vec<f64>, // gamma=0.5
    pub laguerre_rsi_06: Vec<f64>, // gamma=0.6
    pub laguerre_rsi_07: Vec<f64>, // gamma=0.7
    pub laguerre_rsi_08: Vec<f64>, // gamma=0.8

    // Mass Index
    pub mass_index: Vec<f64>,

    // KST (Know Sure Thing)
    pub kst: Vec<f64>,
    pub kst_signal: Vec<f64>,

    // DEMA / TEMA
    pub dema10: Vec<f64>,
    pub dema20: Vec<f64>,
    pub tema10: Vec<f64>,
    pub tema20: Vec<f64>,

    // Parabolic SAR
    pub sar: Vec<f64>,              // af_max=0.2
    pub sar_conservative: Vec<f64>, // af_max=0.1

    // Ichimoku
    pub tenkan: Vec<f64>,    // 9-period midpoint
    pub kijun: Vec<f64>,     // 26-period midpoint
    pub senkou_a: Vec<f64>,  // (tenkan + kijun) / 2
    pub senkou_b: Vec<f64>,  // 52-period midpoint

    // Kalman Filter (multiple process noise values)
    pub kalman_est_001: Vec<f64>,    // Q=0.01
    pub kalman_var_001: Vec<f64>,
    pub kalman_est_0001: Vec<f64>,   // Q=0.001
    pub kalman_var_0001: Vec<f64>,
    pub kalman_est_00001: Vec<f64>,  // Q=0.0001
    pub kalman_var_00001: Vec<f64>,
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
        let mut s = 0.0;
        let mut s2 = 0.0;
        let mut cnt = 0usize;
        for &v in &data[i + 1 - window..=i] {
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

pub fn compute_ema(data: &[f64], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < period { return out; }
    let k = 2.0 / (period as f64 + 1.0);
    // Seed with SMA
    let mut sum = 0.0;
    let mut cnt = 0usize;
    for i in 0..period {
        if !data[i].is_nan() { sum += data[i]; cnt += 1; }
    }
    if cnt == period {
        out[period - 1] = sum / period as f64;
        for i in period..n {
            if !data[i].is_nan() && !out[i - 1].is_nan() {
                out[i] = data[i] * k + out[i - 1] * (1.0 - k);
            }
        }
    }
    out
}

pub fn compute_rsi(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    if n <= period { return out; }

    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let diff = close[i] - close[i - 1];
        if diff > 0.0 { gains[i] = diff; }
        else { losses[i] = -diff; }
    }

    // Initial average
    let mut avg_gain: f64 = gains[1..=period].iter().sum::<f64>() / period as f64;
    let mut avg_loss: f64 = losses[1..=period].iter().sum::<f64>() / period as f64;

    if avg_loss > 0.0 {
        let rs = avg_gain / avg_loss;
        out[period] = 100.0 - 100.0 / (1.0 + rs);
    } else {
        out[period] = 100.0;
    }

    for i in (period + 1)..n {
        avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;
        if avg_loss > 0.0 {
            let rs = avg_gain / avg_loss;
            out[i] = 100.0 - 100.0 / (1.0 + rs);
        } else {
            out[i] = 100.0;
        }
    }
    out
}

fn compute_roc(data: &[f64], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in period..n {
        if !data[i - period].is_nan() && data[i - period].abs() > 1e-15 {
            out[i] = (data[i] - data[i - period]) / data[i - period] * 100.0;
        }
    }
    out
}

// ---- QQE ----
fn compute_qqe(close: &[f64], rsi_period: usize, smoothing: usize, mult: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = close.len();
    let rsi = compute_rsi(close, rsi_period);
    let rsi_smooth = compute_ema(&rsi, smoothing);

    // ATR of RSI: Wilder smoothing of |delta(rsi_smooth)|
    let wilder_period = 2 * smoothing - 1;
    let mut abs_delta = vec![f64::NAN; n];
    for i in 1..n {
        if !rsi_smooth[i].is_nan() && !rsi_smooth[i - 1].is_nan() {
            abs_delta[i] = (rsi_smooth[i] - rsi_smooth[i - 1]).abs();
        }
    }
    let atr_rsi = compute_ema(&abs_delta, wilder_period);

    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];
    for i in 0..n {
        if !rsi_smooth[i].is_nan() && !atr_rsi[i].is_nan() {
            upper[i] = rsi_smooth[i] + atr_rsi[i] * mult;
            lower[i] = rsi_smooth[i] - atr_rsi[i] * mult;
        }
    }
    (rsi_smooth, upper, lower)
}

// ---- Laguerre RSI ----
fn compute_laguerre_rsi(close: &[f64], gamma: f64) -> Vec<f64> {
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    if n < 2 { return out; }

    let mut l0 = 0.0;
    let mut l1 = 0.0;
    let mut l2 = 0.0;
    let mut l3 = 0.0;

    for i in 0..n {
        let l0_prev = l0;
        let l1_prev = l1;
        let l2_prev = l2;

        l0 = (1.0 - gamma) * close[i] + gamma * l0_prev;
        l1 = -gamma * l0 + l0_prev + gamma * l1_prev;
        l2 = -gamma * l1 + l1_prev + gamma * l2_prev;
        l3 = -gamma * l2 + l2_prev + gamma * l3;

        let mut cu = 0.0;
        let mut cd = 0.0;

        let d0 = l0 - l1;
        if d0 > 0.0 { cu += d0; } else { cd -= d0; }

        let d1 = l1 - l2;
        if d1 > 0.0 { cu += d1; } else { cd -= d1; }

        let d2 = l2 - l3;
        if d2 > 0.0 { cu += d2; } else { cd -= d2; }

        if i >= 4 {
            let sum = cu + cd;
            out[i] = if sum > 0.0 { cu / sum * 100.0 } else { 50.0 };
        }
    }
    out
}

// ---- Mass Index ----
fn compute_mass_index(high: &[f64], low: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut range = vec![0.0; n];
    for i in 0..n {
        range[i] = high[i] - low[i];
    }
    let single_ema = compute_ema(&range, 9);
    let double_ema = compute_ema(&single_ema, 9);

    let mut ratio = vec![f64::NAN; n];
    for i in 0..n {
        if !single_ema[i].is_nan() && !double_ema[i].is_nan() && double_ema[i].abs() > 1e-15 {
            ratio[i] = single_ema[i] / double_ema[i];
        }
    }

    // Sum ratio over 25 bars
    let mut mi = vec![f64::NAN; n];
    if n >= 25 {
        for i in 24..n {
            let mut sum = 0.0;
            let mut cnt = 0;
            for j in (i + 1 - 25)..=i {
                if !ratio[j].is_nan() { sum += ratio[j]; cnt += 1; }
            }
            if cnt == 25 { mi[i] = sum; }
        }
    }
    mi
}

// ---- KST ----
fn compute_kst(close: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let roc1 = compute_roc(close, 10);
    let roc2 = compute_roc(close, 15);
    let roc3 = compute_roc(close, 20);
    let roc4 = compute_roc(close, 30);

    let sma_roc1 = rolling_mean(&roc1, 10);
    let sma_roc2 = rolling_mean(&roc2, 10);
    let sma_roc3 = rolling_mean(&roc3, 10);
    let sma_roc4 = rolling_mean(&roc4, 15);

    let n = close.len();
    let mut kst = vec![f64::NAN; n];
    for i in 0..n {
        if !sma_roc1[i].is_nan() && !sma_roc2[i].is_nan()
            && !sma_roc3[i].is_nan() && !sma_roc4[i].is_nan()
        {
            kst[i] = sma_roc1[i] * 1.0 + sma_roc2[i] * 2.0
                + sma_roc3[i] * 3.0 + sma_roc4[i] * 4.0;
        }
    }
    let kst_signal = rolling_mean(&kst, 9);
    (kst, kst_signal)
}

// ---- DEMA / TEMA ----
fn compute_dema(close: &[f64], period: usize) -> Vec<f64> {
    let ema1 = compute_ema(close, period);
    let ema2 = compute_ema(&ema1, period);
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if !ema1[i].is_nan() && !ema2[i].is_nan() {
            out[i] = 2.0 * ema1[i] - ema2[i];
        }
    }
    out
}

fn compute_tema(close: &[f64], period: usize) -> Vec<f64> {
    let ema1 = compute_ema(close, period);
    let ema2 = compute_ema(&ema1, period);
    let ema3 = compute_ema(&ema2, period);
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if !ema1[i].is_nan() && !ema2[i].is_nan() && !ema3[i].is_nan() {
            out[i] = 3.0 * ema1[i] - 3.0 * ema2[i] + ema3[i];
        }
    }
    out
}

// ---- Parabolic SAR ----
fn compute_parabolic_sar(high: &[f64], low: &[f64], close: &[f64], af_start: f64, af_max: f64) -> Vec<f64> {
    let n = high.len();
    let mut sar = vec![f64::NAN; n];
    if n < 2 { return sar; }

    let af_step = af_start;
    let mut is_long = close[1] > close[0];
    let mut af = af_start;
    let mut ep;
    let mut current_sar;

    if is_long {
        current_sar = low[0];
        ep = high[1];
    } else {
        current_sar = high[0];
        ep = low[1];
    }
    sar[1] = current_sar;

    for i in 2..n {
        let prev_sar = current_sar;

        if is_long {
            current_sar = prev_sar + af * (ep - prev_sar);
            // SAR cannot be above the prior two lows
            current_sar = current_sar.min(low[i - 1]).min(low[i - 2]);

            if low[i] < current_sar {
                // Flip to short
                is_long = false;
                current_sar = ep;
                ep = low[i];
                af = af_start;
            } else {
                if high[i] > ep {
                    ep = high[i];
                    af = (af + af_step).min(af_max);
                }
            }
        } else {
            current_sar = prev_sar + af * (ep - prev_sar);
            // SAR cannot be below the prior two highs
            current_sar = current_sar.max(high[i - 1]).max(high[i - 2]);

            if high[i] > current_sar {
                // Flip to long
                is_long = true;
                current_sar = ep;
                ep = high[i];
                af = af_start;
            } else {
                if low[i] < ep {
                    ep = low[i];
                    af = (af + af_step).min(af_max);
                }
            }
        }
        sar[i] = current_sar;
    }
    sar
}

// ---- Ichimoku ----
fn compute_midpoint(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let hi = rolling_max(high, period);
    let lo = rolling_min(low, period);
    let n = high.len();
    let mut out = vec![f64::NAN; n];
    for i in 0..n {
        if !hi[i].is_nan() && !lo[i].is_nan() {
            out[i] = (hi[i] + lo[i]) / 2.0;
        }
    }
    out
}

// ---- Kalman Filter ----
fn compute_kalman(close: &[f64], process_noise: f64, measurement_noise: f64) -> (Vec<f64>, Vec<f64>) {
    let n = close.len();
    let mut est = vec![f64::NAN; n];
    let mut var = vec![f64::NAN; n];
    if n == 0 { return (est, var); }

    let mut x_est = close[0];
    let mut p_est = 1.0;

    est[0] = x_est;
    var[0] = p_est;

    for i in 1..n {
        // Predict
        let x_pred = x_est;
        let p_pred = p_est + process_noise;

        // Update
        let k = p_pred / (p_pred + measurement_noise);
        x_est = x_pred + k * (close[i] - x_pred);
        p_est = (1.0 - k) * p_pred;

        est[i] = x_est;
        var[i] = p_est;
    }
    (est, var)
}

// ---- Main compute function ----

pub fn compute_all(
    open: &[f64], high: &[f64], low: &[f64], close: &[f64], volume: &[f64],
) -> Indicators {
    let n = close.len();

    // Base
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

    // Candle stats
    let mut body = vec![0.0; n];
    let mut range_vec = vec![0.0; n];
    for i in 0..n {
        body[i] = (close[i] - open[i]).abs();
        range_vec[i] = high[i] - low[i];
    }
    let avg_body20 = rolling_mean(&body, 20);
    let avg_range20 = rolling_mean(&range_vec, 20);

    // QQE
    let (qqe_rsi_smooth, qqe_upper, qqe_lower) = compute_qqe(close, 14, 5, 4.236);
    let (_, qqe_upper_tight, qqe_lower_tight) = compute_qqe(close, 14, 5, 3.0);

    // Laguerre RSI
    let laguerre_rsi_05 = compute_laguerre_rsi(close, 0.5);
    let laguerre_rsi_06 = compute_laguerre_rsi(close, 0.6);
    let laguerre_rsi_07 = compute_laguerre_rsi(close, 0.7);
    let laguerre_rsi_08 = compute_laguerre_rsi(close, 0.8);

    // Mass Index
    let mass_index = compute_mass_index(high, low);

    // KST
    let (kst, kst_signal) = compute_kst(close);

    // DEMA / TEMA
    let dema10 = compute_dema(close, 10);
    let dema20 = compute_dema(close, 20);
    let tema10 = compute_tema(close, 10);
    let tema20 = compute_tema(close, 20);

    // Parabolic SAR
    let sar = compute_parabolic_sar(high, low, close, 0.02, 0.2);
    let sar_conservative = compute_parabolic_sar(high, low, close, 0.02, 0.1);

    // Ichimoku
    let tenkan = compute_midpoint(high, low, 9);
    let kijun = compute_midpoint(high, low, 26);
    let senkou_b = compute_midpoint(high, low, 52);
    let mut senkou_a = vec![f64::NAN; n];
    for i in 0..n {
        if !tenkan[i].is_nan() && !kijun[i].is_nan() {
            senkou_a[i] = (tenkan[i] + kijun[i]) / 2.0;
        }
    }

    // Kalman Filter (3 process noise levels)
    let (kalman_est_001, kalman_var_001) = compute_kalman(close, 0.01, 1.0);
    let (kalman_est_0001, kalman_var_0001) = compute_kalman(close, 0.001, 1.0);
    let (kalman_est_00001, kalman_var_00001) = compute_kalman(close, 0.0001, 1.0);

    Indicators {
        sma20, std20, z_score, vol_ratio,
        avg_body20, avg_range20,
        qqe_rsi_smooth, qqe_upper, qqe_lower, qqe_upper_tight, qqe_lower_tight,
        laguerre_rsi_05, laguerre_rsi_06, laguerre_rsi_07, laguerre_rsi_08,
        mass_index,
        kst, kst_signal,
        dema10, dema20, tema10, tema20,
        sar, sar_conservative,
        tenkan, kijun, senkou_a, senkou_b,
        kalman_est_001, kalman_var_001,
        kalman_est_0001, kalman_var_0001,
        kalman_est_00001, kalman_var_00001,
    }
}
