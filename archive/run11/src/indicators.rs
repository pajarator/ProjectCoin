/// Indicators for RUN11c structural/algorithmic strategies.

pub struct Indicators {
    // Base
    pub sma20: Vec<f64>,
    pub std20: Vec<f64>,
    pub z_score: Vec<f64>,
    pub vol_ratio: Vec<f64>,

    // Opening Range (34): first N candles of each "session" (96 candles = 24h)
    pub or_high: Vec<f64>,  // opening range high
    pub or_low: Vec<f64>,   // opening range low

    // Pivot Points (35): calculated from prior session
    pub pivot: Vec<f64>,
    pub r1: Vec<f64>,
    pub s1: Vec<f64>,

    // Fibonacci (36): retracement levels from recent swing
    pub fib_618: Vec<f64>,
    pub fib_786: Vec<f64>,

    // Percentile Rank (37): current close rank in last 100 bars
    pub pct_rank: Vec<f64>,

    // Half-Life (38): Ornstein-Uhlenbeck mean reversion speed
    pub ou_halflife: Vec<f64>,
    pub ou_deviation: Vec<f64>,

    // Hurst Exponent (39): mean-reverting (<0.5) vs trending (>0.5)
    pub hurst: Vec<f64>,

    // Acceleration (40): 2nd derivative of price (rate of change of momentum)
    pub accel: Vec<f64>,

    // Dual Thrust (41): range-based breakout thresholds
    pub dt_upper: Vec<f64>,
    pub dt_lower: Vec<f64>,

    // Impulse Candle (42)
    pub avg_body20: Vec<f64>,
    pub avg_range20: Vec<f64>,

    // Gap (43)
    pub gap_pct: Vec<f64>,

    // Momentum Shift (44): ROC changing sign
    pub roc5: Vec<f64>,
    pub roc20: Vec<f64>,

    // VWAP + ATR bands (45)
    pub vwap96: Vec<f64>,
    pub atr14: Vec<f64>,
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

    // 34. Opening Range: use 96-candle sessions (24h on 15m)
    let session = 96usize;
    let mut or_high = vec![f64::NAN; n];
    let mut or_low = vec![f64::NAN; n];
    let or_bars = 4; // first 4 bars = 1 hour opening range
    for i in 0..n {
        let session_start = (i / session) * session;
        if i >= session_start + or_bars {
            let mut hi = f64::NEG_INFINITY;
            let mut lo = f64::INFINITY;
            for j in session_start..session_start + or_bars {
                if high[j] > hi { hi = high[j]; }
                if low[j] < lo { lo = low[j]; }
            }
            or_high[i] = hi;
            or_low[i] = lo;
        }
    }

    // 35. Pivot Points: from previous session's HLC
    let mut pivot = vec![f64::NAN; n];
    let mut r1 = vec![f64::NAN; n];
    let mut s1 = vec![f64::NAN; n];
    for i in 0..n {
        let session_idx = i / session;
        if session_idx == 0 { continue; }
        let prev_start = (session_idx - 1) * session;
        let prev_end = session_idx * session;
        if prev_end > n { continue; }
        let mut ph = f64::NEG_INFINITY;
        let mut pl = f64::INFINITY;
        let pc = close[prev_end - 1];
        for j in prev_start..prev_end {
            if high[j] > ph { ph = high[j]; }
            if low[j] < pl { pl = low[j]; }
        }
        let pp = (ph + pl + pc) / 3.0;
        pivot[i] = pp;
        r1[i] = 2.0 * pp - pl;
        s1[i] = 2.0 * pp - ph;
    }

    // 36. Fibonacci: swing high/low in last 50 bars, retrace levels
    let swing_lb = 50usize;
    let mut fib_618 = vec![f64::NAN; n];
    let mut fib_786 = vec![f64::NAN; n];
    for i in swing_lb..n {
        let mut swing_hi = f64::NEG_INFINITY;
        let mut swing_lo = f64::INFINITY;
        for j in (i - swing_lb)..i {
            if high[j] > swing_hi { swing_hi = high[j]; }
            if low[j] < swing_lo { swing_lo = low[j]; }
        }
        let range = swing_hi - swing_lo;
        if range > 0.0 {
            fib_618[i] = swing_hi - 0.618 * range;
            fib_786[i] = swing_hi - 0.786 * range;
        }
    }

    // 37. Percentile Rank: where is current close vs last 100 bars
    let pct_window = 100usize;
    let mut pct_rank = vec![f64::NAN; n];
    for i in pct_window..n {
        let mut below = 0u32;
        for j in (i - pct_window)..i {
            if close[j] < close[i] { below += 1; }
        }
        pct_rank[i] = below as f64 / pct_window as f64 * 100.0;
    }

    // 38. Ornstein-Uhlenbeck half-life (rolling 100-bar regression of spread)
    let ou_window = 100usize;
    let mut ou_halflife = vec![f64::NAN; n];
    let mut ou_deviation = vec![f64::NAN; n];
    for i in (ou_window + 1)..n {
        // Regress delta_spread on spread: delta = a + b*spread
        // spread = close - SMA20
        if sma20[i].is_nan() { continue; }
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        let mut cnt = 0u32;
        for j in (i - ou_window + 1)..=i {
            if sma20[j].is_nan() || sma20[j - 1].is_nan() { continue; }
            let spread = close[j - 1] - sma20[j - 1];
            let delta = (close[j] - sma20[j]) - spread;
            sum_x += spread;
            sum_y += delta;
            sum_xy += spread * delta;
            sum_x2 += spread * spread;
            cnt += 1;
        }
        if cnt < 50 { continue; }
        let n_f = cnt as f64;
        let denom = n_f * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-15 { continue; }
        let b = (n_f * sum_xy - sum_x * sum_y) / denom;
        if b >= 0.0 { continue; } // not mean-reverting
        let hl = -(2.0_f64.ln()) / b;
        ou_halflife[i] = hl;
        ou_deviation[i] = close[i] - sma20[i];
    }

    // 39. Hurst Exponent (simplified R/S analysis, rolling 100 bars)
    let hurst_window = 100usize;
    let mut hurst = vec![f64::NAN; n];
    for i in hurst_window..n {
        let slice = &close[i - hurst_window..i];
        let h = compute_hurst_rs(slice);
        if !h.is_nan() { hurst[i] = h; }
    }

    // 40. Acceleration: 2nd derivative of price via ROC of ROC
    let mut roc1 = vec![f64::NAN; n];
    for i in 1..n {
        if close[i - 1] > 0.0 {
            roc1[i] = (close[i] - close[i - 1]) / close[i - 1];
        }
    }
    let roc1_sma = rolling_mean(&roc1, 5); // smooth momentum
    let mut accel = vec![f64::NAN; n];
    for i in 1..n {
        if !roc1_sma[i].is_nan() && !roc1_sma[i - 1].is_nan() {
            accel[i] = roc1_sma[i] - roc1_sma[i - 1]; // change in momentum
        }
    }

    // 41. Dual Thrust: range-based breakout
    let dt_period = 4usize; // N prior sessions
    let dt_k1 = 0.5; // buy multiplier
    let dt_k2 = 0.5; // sell multiplier
    let mut dt_upper = vec![f64::NAN; n];
    let mut dt_lower = vec![f64::NAN; n];
    for i in 0..n {
        let session_idx = i / session;
        if session_idx < dt_period + 1 { continue; }
        // Get range from prior N sessions
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::INFINITY;
        let mut hc = f64::NEG_INFINITY;
        let mut lc = f64::INFINITY;
        for s in (session_idx - dt_period)..session_idx {
            let s_start = s * session;
            let s_end = ((s + 1) * session).min(n);
            for j in s_start..s_end {
                if high[j] > hh { hh = high[j]; }
                if low[j] < ll { ll = low[j]; }
                if close[j] > hc { hc = close[j]; }
                if close[j] < lc { lc = close[j]; }
            }
        }
        let range = (hh - lc).max(hc - ll);
        let session_open = open[(i / session) * session];
        dt_upper[i] = session_open + dt_k1 * range;
        dt_lower[i] = session_open - dt_k2 * range;
    }

    // 42. Impulse Candle stats
    let mut body = vec![0.0; n];
    let mut range_vec = vec![0.0; n];
    for i in 0..n {
        body[i] = (close[i] - open[i]).abs();
        range_vec[i] = high[i] - low[i];
    }
    let avg_body20 = rolling_mean(&body, 20);
    let avg_range20 = rolling_mean(&range_vec, 20);

    // 43. Gap %: gap between current open and previous close
    let mut gap_pct = vec![f64::NAN; n];
    for i in 1..n {
        if close[i - 1] > 0.0 {
            gap_pct[i] = (open[i] - close[i - 1]) / close[i - 1] * 100.0;
        }
    }

    // 44. ROC (Rate of Change)
    let mut roc5 = vec![f64::NAN; n];
    let mut roc20 = vec![f64::NAN; n];
    for i in 5..n {
        if close[i - 5] > 0.0 {
            roc5[i] = (close[i] - close[i - 5]) / close[i - 5] * 100.0;
        }
    }
    for i in 20..n {
        if close[i - 20] > 0.0 {
            roc20[i] = (close[i] - close[i - 20]) / close[i - 20] * 100.0;
        }
    }

    // 45. Rolling VWAP (96-period = 24h)
    let mut tp_vol = vec![0.0; n];
    for i in 0..n {
        tp_vol[i] = (high[i] + low[i] + close[i]) / 3.0 * volume[i];
    }
    let tp_vol_sum = rolling_sum(&tp_vol, 96);
    let vol_sum = rolling_sum(volume, 96);
    let mut vwap96 = vec![f64::NAN; n];
    for i in 0..n {
        if !tp_vol_sum[i].is_nan() && !vol_sum[i].is_nan() && vol_sum[i] > 0.0 {
            vwap96[i] = tp_vol_sum[i] / vol_sum[i];
        }
    }

    // ATR
    let mut tr = vec![0.0; n];
    if n > 0 { tr[0] = high[0] - low[0]; }
    for i in 1..n {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    let atr14 = rolling_mean(&tr, 14);

    Indicators {
        sma20, std20, z_score, vol_ratio,
        or_high, or_low,
        pivot, r1, s1,
        fib_618, fib_786,
        pct_rank,
        ou_halflife, ou_deviation,
        hurst,
        accel,
        dt_upper, dt_lower,
        avg_body20, avg_range20,
        gap_pct,
        roc5, roc20,
        vwap96, atr14,
    }
}

/// Simplified Hurst exponent via R/S analysis on a slice.
/// Returns H ≈ log(R/S) / log(n)
fn compute_hurst_rs(data: &[f64]) -> f64 {
    let n = data.len();
    if n < 20 { return f64::NAN; }

    // Use sub-periods of sizes [10, 20, 50] and regress log(R/S) vs log(n)
    let sizes = [10usize, 20, 50];
    let mut log_n = Vec::new();
    let mut log_rs = Vec::new();

    for &size in &sizes {
        if size > n { continue; }
        let num_blocks = n / size;
        if num_blocks == 0 { continue; }
        let mut rs_sum = 0.0;
        let mut rs_count = 0u32;

        for b in 0..num_blocks {
            let start = b * size;
            let end = start + size;
            let block = &data[start..end];

            let mean: f64 = block.iter().sum::<f64>() / size as f64;
            let mut cumdev = vec![0.0; size];
            cumdev[0] = block[0] - mean;
            for j in 1..size {
                cumdev[j] = cumdev[j - 1] + (block[j] - mean);
            }
            let r = cumdev.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - cumdev.iter().cloned().fold(f64::INFINITY, f64::min);
            let s = (block.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / size as f64).sqrt();
            if s > 0.0 {
                rs_sum += r / s;
                rs_count += 1;
            }
        }

        if rs_count > 0 {
            let avg_rs = rs_sum / rs_count as f64;
            if avg_rs > 0.0 {
                log_n.push((size as f64).ln());
                log_rs.push(avg_rs.ln());
            }
        }
    }

    if log_n.len() < 2 { return f64::NAN; }

    // Simple linear regression: log_rs = H * log_n + c
    let n_pts = log_n.len() as f64;
    let sx: f64 = log_n.iter().sum();
    let sy: f64 = log_rs.iter().sum();
    let sxy: f64 = log_n.iter().zip(log_rs.iter()).map(|(x, y)| x * y).sum();
    let sx2: f64 = log_n.iter().map(|x| x * x).sum();

    let denom = n_pts * sx2 - sx * sx;
    if denom.abs() < 1e-15 { return f64::NAN; }

    let h = (n_pts * sxy - sx * sy) / denom;
    if h.is_finite() && h > 0.0 && h < 1.5 { h } else { f64::NAN }
}
