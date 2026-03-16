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
        "aroon_cross" => aroon_cross(c, ind, i, cfg),
        "hull_cross" => hull_cross(c, ind, i, cfg),
        "kama_revert" => kama_revert(c, ind, i, cfg),
        "trix_cross" => trix_cross(c, ind, i, cfg),
        "vortex_cross" => vortex_cross(c, ind, i, cfg),
        "elder_ray" => elder_ray(c, ind, i, cfg),
        "awesome_osc" => awesome_osc(c, ind, i, cfg),
        "heikin_ashi" => heikin_ashi(c, ind, i, cfg),
        "bb_kc_squeeze" => bb_kc_squeeze(c, ind, i, cfg),
        "atr_trail_entry" => atr_trail_entry(c, ind, i, cfg),
        "donchian_revert" => donchian_revert(c, ind, i, cfg),
        "vol_contract_expand" => vol_contract_expand(c, ind, i, cfg),
        "chandelier_entry" => chandelier_entry(c, ind, i, cfg),
        "range_contract_break" => range_contract_break(c, ind, i, cfg),
        "linreg_revert" => linreg_revert(c, ind, i, cfg),
        // Control
        "ctrl_mean_rev" => ctrl_mean_rev(ind, i, cfg),
        _ => false,
    }
}

pub fn all_configs() -> Vec<StratConfig> {
    let mut base = Vec::new();

    // 19. Aroon Crossover: p1=aroon_up threshold for bullish
    for &t in &[70.0, 80.0, 90.0] {
        base.push(("aroon_cross", t, 30.0, 0.0)); // up > t, down < p2
    }

    // 20. Hull MA Cross: p1=lookback for trend (price crosses above Hull)
    base.push(("hull_cross", 0.0, 0.0, 0.0));

    // 21. KAMA Revert: p1=distance below KAMA as % to enter
    for &d in &[0.005, 0.01, 0.02] {
        base.push(("kama_revert", d, 0.0, 0.0));
    }

    // 22. TRIX Crossover: TRIX crosses above signal line while negative
    for &t in &[-5.0, -10.0, 0.0] {
        base.push(("trix_cross", t, 0.0, 0.0)); // p1 = max trix level for entry
    }

    // 23. Vortex Cross: VI+ crosses above VI-
    for &margin in &[0.0, 0.05] {
        base.push(("vortex_cross", margin, 0.0, 0.0));
    }

    // 24. Elder Ray: bear power negative but rising, price > EMA13
    for &bp_thresh in &[-0.5, -1.0] {
        base.push(("elder_ray", bp_thresh, 0.0, 0.0));
    }

    // 25. Awesome Oscillator: AO crosses zero from below, or saucer (two consecutive green after red)
    for &mode in &[0.0, 1.0] { // 0=zero cross, 1=saucer
        base.push(("awesome_osc", mode, 0.0, 0.0));
    }

    // 26. Heikin Ashi Reversal: HA turns bullish after bearish sequence
    for &bearish_count in &[2.0, 3.0] {
        base.push(("heikin_ashi", bearish_count, 0.0, 0.0));
    }

    // 27. BB+Keltner Squeeze: squeeze releases with upward momentum
    base.push(("bb_kc_squeeze", 0.0, 0.0, 0.0));

    // 28. ATR Trailing Stop Entry: price rebounds from ATR-based level
    for &mult in &[2.0, 3.0] {
        base.push(("atr_trail_entry", mult, 0.0, 0.0));
    }

    // 29. Donchian Channel Reversion: price near lower channel, reversal
    for &pct in &[0.1, 0.2] { // distance from bottom as fraction of channel width
        base.push(("donchian_revert", pct, 0.0, 0.0));
    }

    // 30. Volatility Contraction → Expansion: ATR contracts then expands
    for &contract_ratio in &[0.5, 0.6] {
        for &expand_mult in &[1.5, 2.0] {
            base.push(("vol_contract_expand", contract_ratio, expand_mult, 0.0));
        }
    }

    // 31. Chandelier Exit as entry: price above chandelier support level
    for &mult in &[2.0, 3.0] {
        base.push(("chandelier_entry", mult, 0.0, 0.0));
    }

    // 32. Range Contraction Breakout: NR7 (narrowest in 7) then break above
    base.push(("range_contract_break", 7.0, 0.0, 0.0));

    // 33. Linear Regression Channel Reversion: price below lower band, bouncing
    for &band_mult in &[1.5, 2.0] {
        base.push(("linreg_revert", band_mult, 0.0, 0.0));
    }

    // Control
    base.push(("ctrl_mean_rev", 1.5, 1.2, 0.0));

    // Expand with z_filter variants
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

/// 19. Aroon Crossover: Aroon Up crosses above threshold, Aroon Down below threshold
/// Signals end of downtrend / start of uptrend
fn aroon_cross(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 26 { return false; }
    let up_thresh = cfg.p1;
    let down_thresh = cfg.p2;

    if ind.aroon_up[i].is_nan() || ind.aroon_down[i].is_nan() { return false; }
    if ind.aroon_up[i - 1].is_nan() || ind.aroon_down[i - 1].is_nan() { return false; }

    // Crossover: up was below down, now above
    ind.aroon_up[i - 1] <= ind.aroon_down[i - 1]
        && ind.aroon_up[i] > ind.aroon_down[i]
        && ind.aroon_up[i] >= up_thresh
        && ind.aroon_down[i] <= down_thresh
}

/// 20. Hull MA Cross: Hull MA turns from falling to rising, price above Hull
fn hull_cross(c: &Candles, ind: &Indicators, i: usize, _cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    if ind.hull_ma[i].is_nan() || ind.hull_ma_prev[i].is_nan() { return false; }

    let curr = ind.hull_ma[i];
    let prev = ind.hull_ma_prev[i]; // hull_ma[i-1]

    // Need hull_ma[i-2] via hull_ma_prev[i-1]
    if i < 2 { return false; }
    let prev_prev = ind.hull_ma_prev[i - 1];
    if prev_prev.is_nan() { return false; }

    // Inflection: was flat/falling, now rising, price above Hull
    prev <= prev_prev && curr > prev && c.close[i] > curr
}

/// 21. KAMA Reversion: price is below KAMA by threshold, and bouncing
fn kama_revert(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 15 { return false; }
    let distance = cfg.p1;

    if ind.kama[i].is_nan() || ind.kama[i - 1].is_nan() { return false; }
    if ind.kama[i] <= 0.0 { return false; }

    let dist_pct = (ind.kama[i] - c.close[i]) / ind.kama[i];

    // Price is below KAMA by at least `distance`, and current candle is bullish
    dist_pct >= distance && c.close[i] > c.open[i]
}

/// 22. TRIX Crossover: TRIX crosses above signal while in negative territory
fn trix_cross(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 50 { return false; }
    let max_level = cfg.p1;

    if ind.trix[i].is_nan() || ind.trix_signal[i].is_nan() { return false; }
    if ind.trix[i - 1].is_nan() || ind.trix_signal[i - 1].is_nan() { return false; }

    // TRIX crosses above signal, while TRIX is below max_level (negative = oversold)
    ind.trix[i - 1] <= ind.trix_signal[i - 1]
        && ind.trix[i] > ind.trix_signal[i]
        && ind.trix[i] < max_level
}

/// 23. Vortex Crossover: VI+ crosses above VI-
fn vortex_cross(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 16 { return false; }
    let margin = cfg.p1;

    if ind.vi_plus[i].is_nan() || ind.vi_minus[i].is_nan() { return false; }
    if ind.vi_plus[i - 1].is_nan() || ind.vi_minus[i - 1].is_nan() { return false; }

    // Crossover with margin
    ind.vi_plus[i - 1] <= ind.vi_minus[i - 1] + margin
        && ind.vi_plus[i] > ind.vi_minus[i] + margin
}

/// 24. Elder Ray: bear power negative but improving, price above EMA13
fn elder_ray(c: &Candles, ind: &Indicators, i: usize, _cfg: &StratConfig) -> bool {
    if i < 15 { return false; }

    if ind.bear_power[i].is_nan() || ind.bear_power[i - 1].is_nan() { return false; }
    if ind.ema13[i].is_nan() { return false; }

    // Price above EMA13 (uptrend), bear power negative but rising (bears weakening)
    c.close[i] > ind.ema13[i]
        && ind.bear_power[i] < 0.0
        && ind.bear_power[i] > ind.bear_power[i - 1]
}

/// 25. Awesome Oscillator
/// mode 0: zero-line cross (AO crosses from negative to positive)
/// mode 1: saucer (AO positive, dips then rises)
fn awesome_osc(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 36 { return false; }
    let mode = cfg.p1 as i32;

    if ind.ao[i].is_nan() || ind.ao[i - 1].is_nan() { return false; }

    match mode {
        0 => {
            // Zero cross: AO was negative, now positive
            ind.ao[i - 1] <= 0.0 && ind.ao[i] > 0.0
        }
        1 => {
            // Saucer: AO > 0, had a dip (decrease then increase)
            if i < 3 { return false; }
            if ind.ao[i - 2].is_nan() { return false; }
            ind.ao[i] > 0.0
                && ind.ao[i - 2] > ind.ao[i - 1]  // was decreasing
                && ind.ao[i] > ind.ao[i - 1]        // now increasing
        }
        _ => false,
    }
}

/// 26. Heikin Ashi Reversal: N consecutive bearish HA candles followed by bullish
fn heikin_ashi(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    let n_bearish = cfg.p1 as usize;
    if i < n_bearish + 1 { return false; }

    // Current HA candle is bullish
    if ind.ha_close[i] <= ind.ha_open[i] { return false; }

    // Previous N candles were bearish
    for k in 1..=n_bearish {
        if ind.ha_close[i - k] >= ind.ha_open[i - k] {
            return false;
        }
    }
    true
}

/// 27. BB+Keltner Squeeze: squeeze was active, now releasing with upward momentum
fn bb_kc_squeeze(c: &Candles, ind: &Indicators, i: usize, _cfg: &StratConfig) -> bool {
    if i < 22 { return false; }

    // Squeeze was active in previous bar, now released
    if !ind.squeeze[i - 1] || ind.squeeze[i] { return false; }

    // Upward momentum: close > SMA20 and rising
    if ind.sma20[i].is_nan() { return false; }
    c.close[i] > ind.sma20[i] && c.close[i] > c.close[i - 1]
}

/// 28. ATR Trailing Stop Entry: price bounces off ATR-based support
fn atr_trail_entry(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    let atr_mult = cfg.p1;

    if ind.atr14[i].is_nan() { return false; }

    // Find highest high in last 20 bars
    let mut highest = f64::NEG_INFINITY;
    for k in 0..20 {
        if i >= k && c.high[i - k] > highest {
            highest = c.high[i - k];
        }
    }

    // Chandelier/ATR stop level
    let stop_level = highest - atr_mult * ind.atr14[i];

    // Price touched or went below stop level recently and is now bouncing above
    if i < 2 { return false; }
    let was_near = c.low[i - 1] <= stop_level * 1.002;
    let now_above = c.close[i] > stop_level && c.close[i] > c.open[i];

    was_near && now_above
}

/// 29. Donchian Channel Reversion: price near lower channel boundary, bouncing
fn donchian_revert(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 21 { return false; }
    let pct = cfg.p1;

    if ind.dc_upper[i].is_nan() || ind.dc_lower[i].is_nan() { return false; }

    let width = ind.dc_upper[i] - ind.dc_lower[i];
    if width <= 0.0 { return false; }

    // Price is in bottom `pct` of channel
    let position = (c.close[i] - ind.dc_lower[i]) / width;

    position <= pct && c.close[i] > c.open[i] // bullish candle near bottom
}

/// 30. Volatility Contraction → Expansion: ATR was contracting, now expanding
fn vol_contract_expand(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let contract_ratio = cfg.p1;
    let expand_mult = cfg.p2;

    if ind.atr14[i].is_nan() || ind.avg_range20[i].is_nan() { return false; }
    if ind.avg_range7[i].is_nan() { return false; }
    if ind.avg_range20[i] <= 0.0 { return false; }

    // Recent volatility (7-bar avg range) was contracted vs 20-bar
    // Check if it was contracted 3-5 bars ago
    let mut was_contracted = false;
    for k in 2..=5 {
        if i >= k && !ind.avg_range7[i - k].is_nan() {
            if ind.avg_range7[i - k] / ind.avg_range20[i] < contract_ratio {
                was_contracted = true;
                break;
            }
        }
    }

    // Now expanding: current range > expand_mult × recent 7-bar avg
    let current_range = ind.avg_range7[i];
    was_contracted && current_range / ind.avg_range20[i] > expand_mult * contract_ratio
}

/// 31. Chandelier Exit as entry: price holds above chandelier support after a pullback
fn chandelier_entry(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let atr_mult = cfg.p1;

    if ind.atr14[i].is_nan() { return false; }

    // 22-period highest high
    let mut highest = f64::NEG_INFINITY;
    for k in 0..22 {
        if i >= k && c.high[i - k] > highest {
            highest = c.high[i - k];
        }
    }

    let chandelier_stop = highest - atr_mult * ind.atr14[i];

    // Price dipped near chandelier stop but held and bounced
    if i < 3 { return false; }
    let touched = c.low[i - 1] <= chandelier_stop * 1.005
        || c.low[i - 2] <= chandelier_stop * 1.005;
    let held = c.close[i] > chandelier_stop && c.close[i] > c.open[i];

    touched && held
}

/// 32. Range Contraction Breakout: narrowest range in N bars, then bullish breakout
fn range_contract_break(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    let period = cfg.p1 as usize;
    if i < period + 1 { return false; }

    if ind.avg_range20[i].is_nan() { return false; }

    // Check if bar i-1 had the narrowest range in the last `period` bars
    let range_prev = c.high[i - 1] - c.low[i - 1];
    if range_prev <= 0.0 { return false; }

    let mut is_narrowest = true;
    for k in 2..=period {
        if i < k { break; }
        let r = c.high[i - k] - c.low[i - k];
        if r <= range_prev {
            is_narrowest = false;
            break;
        }
    }
    if !is_narrowest { return false; }

    // Breakout above narrow bar's high
    c.close[i] > c.high[i - 1]
}

/// 33. Linear Regression Channel Reversion: price below lower band, bouncing back
fn linreg_revert(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 22 { return false; }
    let _band_mult = cfg.p1; // already used in indicator computation, but we use the pre-computed bands

    if ind.linreg[i].is_nan() || ind.linreg_lower[i].is_nan() { return false; }

    if ind.linreg_lower[i - 1].is_nan() { return false; }

    // Price was at or below lower regression band, now bouncing above
    let was_below = c.close[i - 1] <= ind.linreg_lower[i - 1] || c.low[i] <= ind.linreg_lower[i];
    was_below && c.close[i] > ind.linreg_lower[i] && c.close[i] > c.open[i]
}

/// Control: Z-score mean reversion (same as COINCLAW)
fn ctrl_mean_rev(ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    if ind.z_score[i].is_nan() || ind.vol_ratio[i].is_nan() { return false; }
    ind.z_score[i] < -cfg.p1 && ind.vol_ratio[i] > cfg.p2
}
