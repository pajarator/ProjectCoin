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
        "orb" => orb(c, ind, i, cfg),
        "pivot_revert" => pivot_revert(c, ind, i, cfg),
        "fib_entry" => fib_entry(c, ind, i, cfg),
        "pct_rank" => pct_rank_entry(c, ind, i, cfg),
        "ou_mean_rev" => ou_mean_rev(c, ind, i, cfg),
        "hurst_filter" => hurst_filter(c, ind, i, cfg),
        "accel_reversal" => accel_reversal(c, ind, i, cfg),
        "dual_thrust" => dual_thrust(c, ind, i, cfg),
        "impulse_follow" => impulse_follow(c, ind, i, cfg),
        "gap_and_go" => gap_and_go(c, ind, i, cfg),
        "momentum_shift" => momentum_shift(c, ind, i, cfg),
        "vwap_atr_revert" => vwap_atr_revert(c, ind, i, cfg),
        "ctrl_mean_rev" => ctrl_mean_rev(ind, i, cfg),
        _ => false,
    }
}

pub fn all_configs() -> Vec<StratConfig> {
    let mut base = Vec::new();

    // 34. ORB: p1=breakout_mult (how far above OR high)
    for &m in &[0.0, 0.002] {
        base.push(("orb", m, 0.0, 0.0));
    }

    // 35. Pivot Point Reversion: p1=distance below S1 as fraction
    for &d in &[0.0, 0.002, 0.005] {
        base.push(("pivot_revert", d, 0.0, 0.0));
    }

    // 36. Fibonacci Retracement: p1=which level (0=0.618, 1=0.786)
    for &level in &[0.0, 1.0] {
        base.push(("fib_entry", level, 0.0, 0.0));
    }

    // 37. Percentile Rank: p1=threshold (enter when rank below this)
    for &t in &[5.0, 10.0, 15.0] {
        base.push(("pct_rank", t, 0.0, 0.0));
    }

    // 38. OU Mean Reversion: p1=min halflife, p2=deviation threshold (in std)
    for &hl in &[5.0, 10.0, 20.0] {
        for &dev in &[1.5, 2.0] {
            base.push(("ou_mean_rev", hl, dev, 0.0));
        }
    }

    // 39. Hurst Filter + mean rev: p1=max hurst (mean-reverting < 0.5)
    for &h in &[0.4, 0.45, 0.5] {
        base.push(("hurst_filter", h, 0.0, 0.0));
    }

    // 40. Acceleration Reversal: p1=lookback for negative accel
    base.push(("accel_reversal", 3.0, 0.0, 0.0));
    base.push(("accel_reversal", 5.0, 0.0, 0.0));

    // 41. Dual Thrust: p1=not used (params in indicator)
    base.push(("dual_thrust", 0.0, 0.0, 0.0));

    // 42. Impulse Follow: p1=body multiplier vs avg
    for &m in &[2.0, 3.0] {
        base.push(("impulse_follow", m, 0.0, 0.0));
    }

    // 43. Gap and Go: p1=min gap % (negative = gap down)
    for &g in &[-0.3, -0.5, -1.0] {
        base.push(("gap_and_go", g, 0.0, 0.0));
    }

    // 44. Momentum Shift: p1=roc5 cross direction (positive shift after negative)
    base.push(("momentum_shift", 0.0, 0.0, 0.0));

    // 45. VWAP + ATR Reversion: p1=ATR bands multiplier
    for &m in &[1.0, 1.5, 2.0] {
        base.push(("vwap_atr_revert", m, 0.0, 0.0));
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

/// 34. Opening Range Breakout: price breaks above opening range high
fn orb(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 100 { return false; }
    let mult = cfg.p1;

    if ind.or_high[i].is_nan() || ind.or_low[i].is_nan() { return false; }

    // For long: break above OR high (with optional margin)
    // But we want reversion-compatible: break BELOW OR low = oversold → buy
    let threshold = ind.or_low[i] * (1.0 - mult);
    c.close[i] <= threshold && c.close[i] > c.open[i] // touched below and bouncing
}

/// 35. Pivot Point Reversion: price near or below S1, bouncing toward pivot
fn pivot_revert(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 100 { return false; }
    let dist = cfg.p1;

    if ind.s1[i].is_nan() || ind.pivot[i].is_nan() { return false; }
    if ind.s1[i] <= 0.0 { return false; }

    // Price at or below S1 (with tolerance) and bullish candle
    let threshold = ind.s1[i] * (1.0 - dist);
    c.close[i] <= threshold && c.close[i] > c.open[i]
}

/// 36. Fibonacci Retracement: price at fib level and bouncing
fn fib_entry(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 55 { return false; }
    let level = cfg.p1 as i32; // 0=0.618, 1=0.786

    let fib = match level {
        0 => ind.fib_618[i],
        _ => ind.fib_786[i],
    };
    if fib.is_nan() || fib <= 0.0 { return false; }

    // Price near fib level (within 0.3%) and bullish
    let dist = (c.close[i] - fib).abs() / fib;
    dist < 0.003 && c.close[i] > c.open[i]
}

/// 37. Percentile Rank: price at historically low level, expect mean reversion
fn pct_rank_entry(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 105 { return false; }
    let threshold = cfg.p1;

    if ind.pct_rank[i].is_nan() || ind.pct_rank[i - 1].is_nan() { return false; }

    // Percentile rank below threshold and rising (bouncing from bottom)
    ind.pct_rank[i - 1] < threshold && ind.pct_rank[i] >= ind.pct_rank[i - 1]
}

/// 38. Ornstein-Uhlenbeck: confirmed mean-reverting regime + extended deviation
fn ou_mean_rev(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 105 { return false; }
    let min_halflife = cfg.p1;
    let dev_thresh = cfg.p2;

    if ind.ou_halflife[i].is_nan() || ind.ou_deviation[i].is_nan() { return false; }
    if ind.std20[i].is_nan() || ind.std20[i] <= 0.0 { return false; }

    // Halflife must be reasonable (not too fast, not too slow)
    let hl = ind.ou_halflife[i];
    if hl < min_halflife || hl > 100.0 { return false; }

    // Deviation below mean by dev_thresh standard deviations
    let dev_z = ind.ou_deviation[i] / ind.std20[i];
    dev_z < -dev_thresh
}

/// 39. Hurst Filter: only enter mean reversion when Hurst confirms mean-reverting regime
fn hurst_filter(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 105 { return false; }
    let max_hurst = cfg.p1;

    if ind.hurst[i].is_nan() || ind.z_score[i].is_nan() { return false; }

    // Hurst < 0.5 = mean-reverting regime, combined with z-score entry
    ind.hurst[i] < max_hurst && ind.z_score[i] < -1.5
}

/// 40. Acceleration Reversal: negative acceleration (deceleration of selling) → reversal
fn accel_reversal(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let lookback = cfg.p1 as usize;

    if ind.accel[i].is_nan() { return false; }

    // Price was falling (negative momentum) but deceleration kicks in (accel positive)
    // = selling pressure weakening
    let mut was_negative = false;
    for k in 1..=lookback {
        if i >= k && !ind.accel[i - k].is_nan() && ind.accel[i - k] < 0.0 {
            was_negative = true;
            break;
        }
    }

    was_negative && ind.accel[i] > 0.0 && c.close[i] > c.open[i]
}

/// 41. Dual Thrust: price breaks below lower threshold (reversion setup)
fn dual_thrust(c: &Candles, ind: &Indicators, i: usize, _cfg: &StratConfig) -> bool {
    if i < 500 { return false; } // need several sessions

    if ind.dt_lower[i].is_nan() { return false; }

    // Price below dual thrust lower band and bouncing
    c.low[i] <= ind.dt_lower[i] && c.close[i] > ind.dt_lower[i] && c.close[i] > c.open[i]
}

/// 42. Impulse Candle Follow-through: large bullish candle (conviction)
fn impulse_follow(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 22 { return false; }
    let body_mult = cfg.p1;

    if ind.avg_body20[i].is_nan() || ind.avg_body20[i] <= 0.0 { return false; }

    let body = c.close[i] - c.open[i]; // positive = bullish
    if body <= 0.0 { return false; }

    // Large bullish body relative to average
    body > ind.avg_body20[i] * body_mult
}

/// 43. Gap and Go: gap down reversal (gap down + volume + bullish close)
fn gap_and_go(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 22 { return false; }
    let min_gap = cfg.p1; // negative value for gap down

    if ind.gap_pct[i].is_nan() || ind.vol_ratio[i].is_nan() { return false; }

    // Gap down beyond threshold + above-average volume + bullish close
    ind.gap_pct[i] <= min_gap
        && ind.vol_ratio[i] > 1.2
        && c.close[i] > c.open[i]
}

/// 44. Momentum Shift: ROC5 crosses from negative to positive (momentum reversal)
fn momentum_shift(_c: &Candles, ind: &Indicators, i: usize, _cfg: &StratConfig) -> bool {
    if i < 22 { return false; }

    if ind.roc5[i].is_nan() || ind.roc5[i - 1].is_nan() { return false; }
    if ind.roc20[i].is_nan() { return false; }

    // Short-term momentum turns positive while long-term is still negative (early reversal)
    ind.roc5[i - 1] <= 0.0 && ind.roc5[i] > 0.0 && ind.roc20[i] < 0.0
}

/// 45. VWAP + ATR Reversion: price below VWAP by N×ATR, bouncing
fn vwap_atr_revert(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 100 { return false; }
    let atr_mult = cfg.p1;

    if ind.vwap96[i].is_nan() || ind.atr14[i].is_nan() { return false; }

    let lower_band = ind.vwap96[i] - atr_mult * ind.atr14[i];

    // Price at or below lower VWAP band and bullish
    c.close[i] <= lower_band && c.close[i] > c.open[i]
}

/// Control: standard mean reversion
fn ctrl_mean_rev(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    if ind.z_score[i].is_nan() || ind.vol_ratio[i].is_nan() { return false; }
    ind.z_score[i] < -cfg.p1 && ind.vol_ratio[i] > cfg.p2
}
