use crate::indicators::Indicators;

#[derive(Debug, Clone)]
pub struct StratConfig {
    pub name: &'static str,
    pub p1: f64,
    pub p2: f64,
    pub p3: f64,
    pub z_filter: f64,
}

impl StratConfig {
    pub fn label(&self) -> String {
        if self.z_filter == 0.0 {
            format!("{}|{:.2}|{:.2}|{:.2}", self.name, self.p1, self.p2, self.p3)
        } else {
            format!("{}|{:.2}|{:.2}|z{:.1}", self.name, self.p1, self.p2, self.z_filter)
        }
    }
}

pub struct Candles<'a> {
    pub open: &'a [f64],
    pub high: &'a [f64],
    pub low: &'a [f64],
    pub close: &'a [f64],
    pub volume: &'a [f64],
}

pub fn check_entry(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if cfg.z_filter != 0.0 {
        if ind.z_score[i].is_nan() || ind.z_score[i] > cfg.z_filter {
            return false;
        }
    }
    match cfg.name {
        "pin_bar" => pin_bar(c, ind, i, cfg),
        "hammer" => hammer(c, ind, i, cfg),
        "engulfing" => engulfing(c, i),
        "qqe" => qqe(ind, i, cfg),
        "laguerre_rsi" => laguerre_rsi(ind, i, cfg),
        "mass_index" => mass_index_entry(ind, i, cfg),
        "kst_cross" => kst_cross(ind, i),
        "dema_tema" => dema_tema(c, ind, i, cfg),
        "parabolic_sar" => parabolic_sar(c, ind, i, cfg),
        "ichimoku" => ichimoku(c, ind, i),
        "kalman_filter" => kalman_filter(c, ind, i, cfg),
        "ctrl_mean_rev" => ctrl_mean_rev(ind, i, cfg),
        _ => false,
    }
}

pub fn all_configs() -> Vec<StratConfig> {
    let mut base = Vec::new();

    // 1. Pin Bar: p1=min_wick_ratio, p2=body_max_ratio
    for &wick in &[0.6, 0.7] {
        for &body_max in &[0.3, 0.2] {
            base.push(("pin_bar", wick, body_max, 0.0));
        }
    }

    // 2. Hammer: p1=min_wick_ratio
    for &wick in &[0.6, 0.7] {
        base.push(("hammer", wick, 0.0, 0.0));
    }

    // 3. Engulfing: no params
    base.push(("engulfing", 0.0, 0.0, 0.0));

    // 4. QQE: p1=mult selector (0=4.236, 1=3.0)
    for &mult in &[0.0, 1.0] {
        base.push(("qqe", mult, 0.0, 0.0));
    }

    // 5. Laguerre RSI: p1=gamma selector (0=0.5, 1=0.6, 2=0.7, 3=0.8)
    for g in 0..4 {
        base.push(("laguerre_rsi", g as f64, 0.0, 0.0));
    }

    // 6. Mass Index: p1=threshold
    for &t in &[26.5, 27.0] {
        base.push(("mass_index", t, 0.0, 0.0));
    }

    // 7. KST: standard params
    base.push(("kst_cross", 0.0, 0.0, 0.0));

    // 8. DEMA/TEMA: p1=period selector (0=10, 1=20), p2=type (0=DEMA, 1=TEMA)
    for &period in &[0.0, 1.0] {
        for &typ in &[0.0, 1.0] {
            base.push(("dema_tema", period, typ, 0.0));
        }
    }

    // 9. Parabolic SAR: p1=variant (0=standard af_max=0.2, 1=conservative af_max=0.1)
    for &v in &[0.0, 1.0] {
        base.push(("parabolic_sar", v, 0.0, 0.0));
    }

    // 10. Ichimoku: standard params
    base.push(("ichimoku", 0.0, 0.0, 0.0));

    // 11. Kalman Filter: p1=process noise selector (0=0.01, 1=0.001, 2=0.0001)
    for q in 0..3 {
        base.push(("kalman_filter", q as f64, 0.0, 0.0));
    }

    // Control
    base.push(("ctrl_mean_rev", 1.5, 1.2, 0.0));

    let z_filters = [0.0, -0.5, -1.0, -1.5];
    let mut cfgs = Vec::new();
    for &(name, p1, p2, p3) in &base {
        for &z in &z_filters {
            cfgs.push(StratConfig { name, p1, p2, p3, z_filter: z });
        }
    }
    cfgs
}

// =============================================
// Strategy implementations
// =============================================

/// 1. Pin Bar: long lower wick (>60% of range), small body, signals reversal
fn pin_bar(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 22 { return false; }
    let min_wick_ratio = cfg.p1;
    let body_max_ratio = cfg.p2;

    let range = c.high[i] - c.low[i];
    if range <= 0.0 { return false; }

    let body = (c.close[i] - c.open[i]).abs();
    let lower_wick = c.open[i].min(c.close[i]) - c.low[i];

    // Long lower wick and small body
    let wick_ratio = lower_wick / range;
    let body_ratio = body / range;

    if wick_ratio < min_wick_ratio || body_ratio > body_max_ratio { return false; }

    // Must be at a relatively low level (below SMA20)
    if ind.sma20[i].is_nan() { return false; }
    c.close[i] < ind.sma20[i]
}

/// 2. Hammer: Pin Bar at swing low (close near high)
fn hammer(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 22 { return false; }
    let min_wick_ratio = cfg.p1;

    let range = c.high[i] - c.low[i];
    if range <= 0.0 { return false; }

    let lower_wick = c.open[i].min(c.close[i]) - c.low[i];
    let upper_wick = c.high[i] - c.open[i].max(c.close[i]);

    // Lower wick dominant, upper wick small, close near high
    let wick_ratio = lower_wick / range;
    let upper_ratio = upper_wick / range;
    if wick_ratio < min_wick_ratio || upper_ratio > 0.1 { return false; }

    // Close must be bullish (close > open) and below SMA20
    if c.close[i] <= c.open[i] { return false; }
    if ind.sma20[i].is_nan() { return false; }
    c.close[i] < ind.sma20[i]
}

/// 3. Engulfing: bullish engulfing (current green body fully engulfs prior red body)
fn engulfing(c: &Candles, i: usize) -> bool {
    if i < 2 { return false; }

    // Prior candle must be bearish (red)
    if c.close[i - 1] >= c.open[i - 1] { return false; }

    // Current candle must be bullish (green)
    if c.close[i] <= c.open[i] { return false; }

    // Current body engulfs prior body
    c.open[i] <= c.close[i - 1] && c.close[i] >= c.open[i - 1]
}

/// 4. QQE: smoothed RSI crosses above lower QQE band (oversold reversal)
fn qqe(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let use_tight = cfg.p1 as i32; // 0=standard (4.236), 1=tight (3.0)

    let (lower, lower_prev) = if use_tight == 1 {
        (ind.qqe_lower_tight[i], ind.qqe_lower_tight[i - 1])
    } else {
        (ind.qqe_lower[i], ind.qqe_lower[i - 1])
    };

    if ind.qqe_rsi_smooth[i].is_nan() || ind.qqe_rsi_smooth[i - 1].is_nan() { return false; }
    if lower.is_nan() || lower_prev.is_nan() { return false; }

    // RSI smoothed crosses above lower band from below
    ind.qqe_rsi_smooth[i - 1] <= lower_prev && ind.qqe_rsi_smooth[i] > lower
}

/// 5. Laguerre RSI: enters when laguerre RSI is oversold (<20) and rising
fn laguerre_rsi(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 10 { return false; }
    let gamma_idx = cfg.p1 as i32;

    let (curr, prev) = match gamma_idx {
        0 => (ind.laguerre_rsi_05[i], ind.laguerre_rsi_05[i - 1]),
        1 => (ind.laguerre_rsi_06[i], ind.laguerre_rsi_06[i - 1]),
        2 => (ind.laguerre_rsi_07[i], ind.laguerre_rsi_07[i - 1]),
        _ => (ind.laguerre_rsi_08[i], ind.laguerre_rsi_08[i - 1]),
    };

    if curr.is_nan() || prev.is_nan() { return false; }

    // Oversold and rising
    prev < 20.0 && curr > prev
}

/// 6. Mass Index: reversal bulge — MI rises above threshold then drops below
fn mass_index_entry(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 30 { return false; }
    let threshold = cfg.p1;

    if ind.mass_index[i].is_nan() || ind.mass_index[i - 1].is_nan() { return false; }

    // Look back for MI > 27 in recent bars, then current MI dropping below threshold
    let mut had_bulge = false;
    for k in 1..10 {
        if i >= k && !ind.mass_index[i - k].is_nan() && ind.mass_index[i - k] > 27.0 {
            had_bulge = true;
            break;
        }
    }

    had_bulge && ind.mass_index[i] < threshold && ind.mass_index[i - 1] >= threshold
}

/// 7. KST: KST crosses above signal line from below
fn kst_cross(ind: &Indicators, i: usize) -> bool {
    if i < 50 { return false; }

    if ind.kst[i].is_nan() || ind.kst[i - 1].is_nan() { return false; }
    if ind.kst_signal[i].is_nan() || ind.kst_signal[i - 1].is_nan() { return false; }

    // Cross above signal from below
    ind.kst[i - 1] <= ind.kst_signal[i - 1] && ind.kst[i] > ind.kst_signal[i]
}

/// 8. DEMA/TEMA cross: price crosses above DEMA or TEMA from below
fn dema_tema(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let period_idx = cfg.p1 as i32; // 0=10, 1=20
    let type_idx = cfg.p2 as i32;   // 0=DEMA, 1=TEMA

    let (curr_ma, prev_ma) = match (period_idx, type_idx) {
        (0, 0) => (ind.dema10[i], ind.dema10[i - 1]),
        (0, 1) => (ind.tema10[i], ind.tema10[i - 1]),
        (1, 0) => (ind.dema20[i], ind.dema20[i - 1]),
        _      => (ind.tema20[i], ind.tema20[i - 1]),
    };

    if curr_ma.is_nan() || prev_ma.is_nan() { return false; }

    // Price crosses above MA from below
    c.close[i - 1] <= prev_ma && c.close[i] > curr_ma
}

/// 9. Parabolic SAR: SAR flips from above to below price (bullish reversal)
fn parabolic_sar(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 5 { return false; }
    let variant = cfg.p1 as i32; // 0=standard, 1=conservative

    let (curr_sar, prev_sar) = if variant == 1 {
        (ind.sar_conservative[i], ind.sar_conservative[i - 1])
    } else {
        (ind.sar[i], ind.sar[i - 1])
    };

    if curr_sar.is_nan() || prev_sar.is_nan() { return false; }

    // SAR was above price (bearish) and flips to below (bullish)
    prev_sar > c.high[i - 1] && curr_sar < c.low[i]
}

/// 10. Ichimoku: Tenkan crosses above Kijun + price above cloud
fn ichimoku(c: &Candles, ind: &Indicators, i: usize) -> bool {
    if i < 55 { return false; }

    if ind.tenkan[i].is_nan() || ind.tenkan[i - 1].is_nan() { return false; }
    if ind.kijun[i].is_nan() || ind.kijun[i - 1].is_nan() { return false; }
    if ind.senkou_a[i].is_nan() || ind.senkou_b[i].is_nan() { return false; }

    // Tenkan crosses above Kijun
    let cross = ind.tenkan[i - 1] <= ind.kijun[i - 1] && ind.tenkan[i] > ind.kijun[i];
    if !cross { return false; }

    // Price above cloud (above both Senkou A and Senkou B)
    let cloud_top = ind.senkou_a[i].max(ind.senkou_b[i]);
    c.close[i] > cloud_top
}

/// 11. Kalman Filter: price below Kalman estimate by N * sqrt(variance)
fn kalman_filter(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 50 { return false; }
    let q_idx = cfg.p1 as i32; // 0=0.01, 1=0.001, 2=0.0001

    let (est, var) = match q_idx {
        0 => (ind.kalman_est_001[i], ind.kalman_var_001[i]),
        1 => (ind.kalman_est_0001[i], ind.kalman_var_0001[i]),
        _ => (ind.kalman_est_00001[i], ind.kalman_var_00001[i]),
    };

    if est.is_nan() || var.is_nan() || var <= 0.0 { return false; }

    // Price below Kalman estimate by 2 std deviations (mean reversion entry)
    let threshold = est - 2.0 * var.sqrt();

    // Also check that price is actually depressed relative to SMA
    if ind.sma20[i].is_nan() { return false; }

    c.close[i] < threshold && c.close[i] < ind.sma20[i]
}

/// Control: standard mean reversion (z-score + volume filter)
fn ctrl_mean_rev(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    if ind.z_score[i].is_nan() || ind.vol_ratio[i].is_nan() { return false; }
    ind.z_score[i] < -cfg.p1 && ind.vol_ratio[i] > cfg.p2
}
