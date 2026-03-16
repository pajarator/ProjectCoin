/// Precomputed indicator vectors for backtesting all 18 strategies.
/// All vectors are the same length as the candle array.
/// NAN means "not yet valid" (insufficient lookback).

pub struct Indicators {
    // Price
    pub sma20: Vec<f64>,
    pub std20: Vec<f64>,
    pub z_score: Vec<f64>,
    // RSI variants
    pub rsi14: Vec<f64>,
    pub rsi3: Vec<f64>,
    // Stochastic
    pub stoch_k: Vec<f64>,
    pub stoch_d: Vec<f64>,
    // Stochastic RSI: Stochastic of RSI (Chande & Kroll)
    pub stoch_rsi_k: Vec<f64>,  // %K line
    pub stoch_rsi_d: Vec<f64>,  // %D signal line (3-period SMA of %K)
    // Connors RSI (average of RSI(3), streak RSI, ROC percentile RSI)
    pub connors_rsi: Vec<f64>,
    // CCI at period 14 and 20
    pub cci14: Vec<f64>,
    pub cci20: Vec<f64>,
    // Volume
    pub obv: Vec<f64>,
    pub cmf20: Vec<f64>,
    pub mfi14: Vec<f64>,
    pub force2: Vec<f64>,
    pub force13: Vec<f64>,
    pub vol_ratio: Vec<f64>,
    // Candle body/range stats
    pub avg_body20: Vec<f64>,
    pub avg_range20: Vec<f64>,
    pub range: Vec<f64>,
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

fn compute_rsi(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut rsi = vec![f64::NAN; n];
    if n < period + 1 { return rsi; }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = close[i] - close[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
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
            let prev = if i > 0 && !out[i - 1].is_nan() { out[i - 1] } else { data[i] };
            out[i] = alpha * data[i] + (1.0 - alpha) * prev;
        }
    }
    out
}

/// Compute mean absolute deviation for CCI
fn rolling_mean_dev(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < window { return out; }
    let sma = rolling_mean(data, window);
    for i in (window - 1)..n {
        if sma[i].is_nan() { continue; }
        let mut sum = 0.0;
        for j in (i + 1 - window)..=i {
            sum += (data[j] - sma[i]).abs();
        }
        out[i] = sum / window as f64;
    }
    out
}

/// Compute all indicators from OHLCV arrays. All vectors have length n.
pub fn compute_all(
    open: &[f64], high: &[f64], low: &[f64], close: &[f64], volume: &[f64],
) -> Indicators {
    let n = close.len();

    // SMA, std, z-score
    let sma20 = rolling_mean(close, 20);
    let std20 = rolling_std(close, 20);
    let mut z_score = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            z_score[i] = (close[i] - sma20[i]) / std20[i];
        }
    }

    // RSI variants
    let rsi14 = compute_rsi(close, 14);
    let rsi3 = compute_rsi(close, 3);

    // Stochastic %K/%D (14-period)
    let low14 = rolling_min(low, 14);
    let high14 = rolling_max(high, 14);
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !low14[i].is_nan() && !high14[i].is_nan() {
            let range = high14[i] - low14[i];
            if range > 0.0 {
                stoch_k[i] = 100.0 * (close[i] - low14[i]) / range;
            }
        }
    }
    let stoch_d = rolling_mean(&stoch_k, 3);

    // Stochastic RSI: Stochastic formula applied to RSI (Chande & Kroll 1994)
    // StochRSI = (RSI - lowest_RSI_in_14) / (highest_RSI_in_14 - lowest_RSI_in_14) × 100
    let rsi_low14 = rolling_min(&rsi14, 14);
    let rsi_high14 = rolling_max(&rsi14, 14);
    let mut stoch_rsi_k = vec![f64::NAN; n];
    for i in 0..n {
        if !rsi14[i].is_nan() && !rsi_low14[i].is_nan() && !rsi_high14[i].is_nan() {
            let range = rsi_high14[i] - rsi_low14[i];
            if range > 0.0 {
                stoch_rsi_k[i] = 100.0 * (rsi14[i] - rsi_low14[i]) / range;
            }
        }
    }
    let stoch_rsi_d = rolling_mean(&stoch_rsi_k, 3); // %D signal line

    // Connors RSI: avg(RSI(3), streak_rsi, roc_percentile_rsi)
    let connors_rsi = compute_connors_rsi(close, &rsi3, n);

    // CCI = (close - SMA) / (0.015 * mean_deviation)
    let cci14 = compute_cci(close, 14);
    let cci20 = compute_cci(close, 20);

    // OBV (On-Balance Volume)
    let mut obv = vec![0.0; n];
    for i in 1..n {
        if close[i] > close[i - 1] {
            obv[i] = obv[i - 1] + volume[i];
        } else if close[i] < close[i - 1] {
            obv[i] = obv[i - 1] - volume[i];
        } else {
            obv[i] = obv[i - 1];
        }
    }

    // Chaikin Money Flow (20-period)
    // CLV = ((close - low) - (high - close)) / (high - low)
    // CMF = sum(CLV * volume, 20) / sum(volume, 20)
    let mut clv_vol = vec![0.0; n];
    for i in 0..n {
        let hl = high[i] - low[i];
        if hl > 0.0 {
            let clv = ((close[i] - low[i]) - (high[i] - close[i])) / hl;
            clv_vol[i] = clv * volume[i];
        }
    }
    let clv_vol_sum = rolling_sum(&clv_vol, 20);
    let vol_sum20 = rolling_sum(volume, 20);
    let mut cmf20 = vec![f64::NAN; n];
    for i in 0..n {
        if !clv_vol_sum[i].is_nan() && !vol_sum20[i].is_nan() && vol_sum20[i] > 0.0 {
            cmf20[i] = clv_vol_sum[i] / vol_sum20[i];
        }
    }

    // MFI (Money Flow Index) - volume-weighted RSI, 14-period
    let mfi14 = compute_mfi(high, low, close, volume, 14);

    // Force Index: (close - prev_close) * volume, then EMA smoothed
    let mut raw_force = vec![f64::NAN; n];
    for i in 1..n {
        raw_force[i] = (close[i] - close[i - 1]) * volume[i];
    }
    let force2 = compute_ema(&raw_force, 2);
    let force13 = compute_ema(&raw_force, 13);

    // Volume ratio (current vol / 20-period SMA vol)
    let vol_ma20 = rolling_mean(volume, 20);
    let mut vol_ratio = vec![f64::NAN; n];
    for i in 0..n {
        if !vol_ma20[i].is_nan() && vol_ma20[i] > 0.0 {
            vol_ratio[i] = volume[i] / vol_ma20[i];
        }
    }

    // Body and range stats for candle pattern strategies
    let mut body = vec![0.0; n];
    let mut range_vec = vec![0.0; n];
    for i in 0..n {
        body[i] = (close[i] - open[i]).abs();
        range_vec[i] = high[i] - low[i];
    }
    let avg_body20 = rolling_mean(&body, 20);
    let avg_range20 = rolling_mean(&range_vec, 20);

    Indicators {
        sma20,
        std20,
        z_score,
        rsi14,
        rsi3,
        stoch_k,
        stoch_d,
        stoch_rsi_k,
        stoch_rsi_d,
        connors_rsi,
        cci14,
        cci20,
        obv,
        cmf20,
        mfi14,
        force2,
        force13,
        vol_ratio,
        avg_body20,
        avg_range20,
        range: range_vec,
    }
}

fn compute_cci(close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let sma = rolling_mean(close, period);
    let md = rolling_mean_dev(close, period);
    let mut cci = vec![f64::NAN; n];
    for i in 0..n {
        if !sma[i].is_nan() && !md[i].is_nan() && md[i] > 0.0 {
            cci[i] = (close[i] - sma[i]) / (0.015 * md[i]);
        }
    }
    cci
}

fn compute_mfi(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut mfi = vec![f64::NAN; n];
    if n < period + 1 { return mfi; }

    // Typical price
    let mut tp = vec![0.0; n];
    for i in 0..n {
        tp[i] = (high[i] + low[i] + close[i]) / 3.0;
    }

    // Raw money flow: positive if tp > prev_tp, negative otherwise
    let mut pos_mf = vec![0.0; n];
    let mut neg_mf = vec![0.0; n];
    for i in 1..n {
        let mf = tp[i] * volume[i];
        if tp[i] > tp[i - 1] {
            pos_mf[i] = mf;
        } else {
            neg_mf[i] = mf;
        }
    }

    let pos_sum = rolling_sum(&pos_mf, period);
    let neg_sum = rolling_sum(&neg_mf, period);
    for i in 0..n {
        if !pos_sum[i].is_nan() && !neg_sum[i].is_nan() && neg_sum[i] > 0.0 {
            let ratio = pos_sum[i] / neg_sum[i];
            mfi[i] = 100.0 - (100.0 / (1.0 + ratio));
        } else if !pos_sum[i].is_nan() && !neg_sum[i].is_nan() {
            mfi[i] = 100.0;
        }
    }
    mfi
}

fn compute_connors_rsi(close: &[f64], rsi3: &[f64], n: usize) -> Vec<f64> {
    let mut crsi = vec![f64::NAN; n];
    if n < 100 { return crsi; }

    // Component 2: Streak RSI
    // Streak = consecutive up/down days. +1 per up day, -1 per down day.
    let mut streak = vec![0.0; n];
    for i in 1..n {
        if close[i] > close[i - 1] {
            streak[i] = if streak[i - 1] > 0.0 { streak[i - 1] + 1.0 } else { 1.0 };
        } else if close[i] < close[i - 1] {
            streak[i] = if streak[i - 1] < 0.0 { streak[i - 1] - 1.0 } else { -1.0 };
        }
        // equal: streak = 0
    }
    let streak_rsi = compute_rsi(&streak, 2);

    // Component 3: Percentile rank of 1-period ROC over last 100 periods
    let mut roc1 = vec![f64::NAN; n];
    for i in 1..n {
        if close[i - 1] > 0.0 {
            roc1[i] = (close[i] - close[i - 1]) / close[i - 1] * 100.0;
        }
    }
    let mut pct_rank = vec![f64::NAN; n];
    for i in 100..n {
        if roc1[i].is_nan() { continue; }
        let mut count_below = 0u32;
        let mut total = 0u32;
        for j in (i - 99)..=i {
            if !roc1[j].is_nan() {
                total += 1;
                if roc1[j] < roc1[i] {
                    count_below += 1;
                }
            }
        }
        if total > 0 {
            pct_rank[i] = count_below as f64 / total as f64 * 100.0;
        }
    }

    // Connors RSI = (RSI(3) + streak_rsi(2) + pct_rank) / 3
    for i in 0..n {
        if !rsi3[i].is_nan() && !streak_rsi[i].is_nan() && !pct_rank[i].is_nan() {
            crsi[i] = (rsi3[i] + streak_rsi[i] + pct_rank[i]) / 3.0;
        }
    }
    crsi
}
