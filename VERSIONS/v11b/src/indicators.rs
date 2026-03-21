#[derive(Debug, Clone, Default)]
pub struct Candle {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Ind15m {
    pub p: f64,
    pub sma20: f64,
    pub sma9: f64,
    pub std20: f64,
    pub z: f64,
    pub bb_lo: f64,
    pub bb_hi: f64,
    pub bb_width: f64,
    pub bb_width_avg: f64,
    pub vol: f64,
    pub vol_ma: f64,
    pub adr_lo: f64,
    pub adr_hi: f64,
    pub rsi: f64,
    pub rsi7: f64,
    pub vwap: f64,
    pub adx: f64,
    pub macd: f64,
    pub macd_signal: f64,
    pub macd_hist: f64,
    pub ou_halflife: f64,
    pub ou_deviation: f64,
    pub valid: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Ind1m {
    pub rsi: f64,
    pub vol: f64,
    pub vol_ma: f64,
    pub stoch_k: f64,
    pub stoch_d: f64,
    pub stoch_k_prev: f64,
    pub stoch_d_prev: f64,
    pub bb_upper: f64,
    pub bb_lower: f64,
    pub bb_width: f64,
    pub bb_width_avg: f64,
    pub roc_3: f64,
    pub avg_body_3: f64,
    pub valid: bool,
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
    if data.len() < window { return out; }
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
    if data.len() < window { return out; }
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
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = close[i] - close[i - 1];
        if d > 0.0 { gains[i] = d; }
        else { losses[i] = -d; }
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
            let prev = if out[i - 1].is_nan() { data[i] } else { out[i - 1] };
            out[i] = alpha * data[i] + (1.0 - alpha) * prev;
        }
    }
    out
}

/// Compute 15m indicators from candle array. Returns the latest Ind15m.
pub fn compute_15m_indicators(candles: &[Candle]) -> Option<Ind15m> {
    let n = candles.len();
    if n < 30 { return None; }

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
    let rsi14 = compute_rsi(&c, 14);
    let rsi7 = compute_rsi(&c, 7);

    // MACD
    let ema12 = compute_ema(&c, 12);
    let ema26 = compute_ema(&c, 26);
    let macd_line: Vec<f64> = (0..n).map(|i| {
        if ema12[i].is_nan() || ema26[i].is_nan() { f64::NAN } else { ema12[i] - ema26[i] }
    }).collect();
    let macd_signal = compute_ema(&macd_line, 9);

    // VWAP
    let tp: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let tp_v: Vec<f64> = (0..n).map(|i| tp[i] * v[i]).collect();
    let tp_v_sum = rolling_sum(&tp_v, 20);
    let v_sum = rolling_sum(&v, 20);

    // BB width
    let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() {
            bb_width_raw[i] = 4.0 * std20[i];
        }
    }
    let bb_width_avg = rolling_mean(&bb_width_raw, 20);

    // ADX
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    let mut true_range = vec![0.0; n];
    for i in 1..n {
        let hh = h[i] - h[i - 1];
        let ll = l[i - 1] - l[i];
        if hh > ll && hh > 0.0 { plus_dm[i] = hh; }
        if ll > hh && ll > 0.0 { minus_dm[i] = ll; }
        let hl = h[i] - l[i];
        let hc = (h[i] - c[i - 1]).abs();
        let lc = (l[i] - c[i - 1]).abs();
        true_range[i] = hl.max(hc).max(lc);
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

    // OU Mean Reversion (RUN11c: DASH only, but computed for all coins)
    let ou_window = crate::config::OU_WINDOW;
    let (ou_halflife, ou_deviation) = if n >= ou_window + 1 {
        let window_c = &c[n - ou_window..];
        let mu = window_c.iter().sum::<f64>() / ou_window as f64;
        // AR(1) regression: x_t = alpha + beta * x_{t-1} + eps
        let mut sx = 0.0_f64; let mut sy = 0.0_f64;
        let mut sxx = 0.0_f64; let mut sxy = 0.0_f64;
        let m = (ou_window - 1) as f64;
        for k in 0..ou_window - 1 {
            sx += window_c[k]; sy += window_c[k + 1];
            sxx += window_c[k] * window_c[k]; sxy += window_c[k] * window_c[k + 1];
        }
        let beta = (m * sxy - sx * sy) / (m * sxx - sx * sx);
        let halflife = if beta > 0.0 && beta < 1.0 { -std::f64::consts::LN_2 / beta.ln() } else { f64::NAN };
        let dev = c[n - 1] - mu;
        (halflife, dev)
    } else {
        (f64::NAN, f64::NAN)
    };

    let i = n - 1;
    if sma20[i].is_nan() || std20[i].is_nan() || std20[i] == 0.0 {
        return None;
    }

    let z = (c[i] - sma20[i]) / std20[i];
    let vwap_val = if !tp_v_sum[i].is_nan() && !v_sum[i].is_nan() && v_sum[i] > 0.0 {
        tp_v_sum[i] / v_sum[i]
    } else { f64::NAN };

    let macd_h = if !macd_line[i].is_nan() && !macd_signal[i].is_nan() {
        macd_line[i] - macd_signal[i]
    } else { 0.0 };

    Some(Ind15m {
        p: c[i],
        sma20: sma20[i],
        sma9: sma9[i],
        std20: std20[i],
        z,
        bb_lo: sma20[i] - 2.0 * std20[i],
        bb_hi: sma20[i] + 2.0 * std20[i],
        bb_width: bb_width_raw[i],
        bb_width_avg: bb_width_avg[i],
        vol: v[i],
        vol_ma: vol_ma[i],
        adr_lo: adr_lo[i],
        adr_hi: adr_hi[i],
        rsi: rsi14[i],
        rsi7: rsi7[i],
        vwap: vwap_val,
        adx: adx[i],
        macd: if macd_line[i].is_nan() { 0.0 } else { macd_line[i] },
        macd_signal: if macd_signal[i].is_nan() { 0.0 } else { macd_signal[i] },
        macd_hist: macd_h,
        ou_halflife,
        ou_deviation,
        valid: !rsi14[i].is_nan() && !adx[i].is_nan(),
    })
}

/// Compute 1m indicators from candle array. Returns the latest Ind1m.
pub fn compute_1m_indicators(candles: &[Candle]) -> Option<Ind1m> {
    let n = candles.len();
    if n < 25 { return None; }

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
    let bb_width_avg = rolling_mean(&bb_width_raw, 20);

    let i = n - 1;
    // F6 filter inputs: 3-bar ROC and avg body size
    let o: Vec<f64> = candles.iter().map(|x| x.o).collect();
    let roc_3 = if i >= 3 && c[i - 3] > 0.0 { (c[i] - c[i - 3]) / c[i - 3] * 100.0 } else { 0.0 };
    let avg_body_3 = if i >= 3 {
        (0..3).map(|k| {
            let idx = i - k;
            if c[idx] > 0.0 { (c[idx] - o[idx]).abs() / c[idx] * 100.0 } else { 0.0 }
        }).sum::<f64>() / 3.0
    } else { 0.0 };

    Some(Ind1m {
        rsi: rsi[i],
        vol: v[i],
        vol_ma: vol_ma[i],
        stoch_k: stoch_k[i],
        stoch_d: stoch_d[i],
        stoch_k_prev: if i > 0 { stoch_k[i - 1] } else { f64::NAN },
        stoch_d_prev: if i > 0 { stoch_d[i - 1] } else { f64::NAN },
        bb_upper: bb_upper[i],
        bb_lower: bb_lower[i],
        bb_width: bb_width_raw[i],
        bb_width_avg: bb_width_avg[i],
        roc_3,
        avg_body_3,
        valid: !rsi[i].is_nan() && !vol_ma[i].is_nan(),
    })
}
