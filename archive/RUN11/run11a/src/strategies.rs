use crate::indicators::Indicators;

/// Each strategy config: name + up to 3 params + Z-score entry filter.
#[derive(Debug, Clone)]
pub struct StratConfig {
    pub name: &'static str,
    pub p1: f64,
    pub p2: f64,
    pub p3: f64,
    /// Z-score must be below this to enter (0.0 = no filter, -0.5 = mild, -1.0 = moderate)
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

/// Candle data needed by strategies.
pub struct Candles<'a> {
    pub open: &'a [f64],
    pub high: &'a [f64],
    pub low: &'a [f64],
    pub close: &'a [f64],
    pub volume: &'a [f64],
}

/// Returns true if strategy fires a LONG entry at index `i`.
pub fn check_entry(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    // Z-score entry filter: only enter when price is below mean
    if cfg.z_filter != 0.0 {
        if ind.z_score[i].is_nan() || ind.z_score[i] > cfg.z_filter {
            return false;
        }
    }

    match cfg.name {
        "morning_star" => morning_star(c, ind, i, cfg),
        "three_soldiers" => three_soldiers(c, ind, i, cfg),
        "tweezer_bottom" => tweezer_bottom(c, ind, i, cfg),
        "inside_bar" => inside_bar(c, ind, i, cfg),
        "marubozu" => marubozu(c, ind, i, cfg),
        "double_bottom" => double_bottom(c, ind, i, cfg),
        "displacement" => displacement(c, ind, i, cfg),
        "nr_breakout" => nr_breakout(c, ind, i, cfg),
        "obv_divergence" => obv_divergence(c, ind, i, cfg),
        "cmf_reversal" => cmf_reversal(c, ind, i, cfg),
        "mfi_divergence" => mfi_divergence(c, ind, i, cfg),
        "vsa_absorption" => vsa_absorption(c, ind, i, cfg),
        "vol_dryup_spike" => vol_dryup_spike(c, ind, i, cfg),
        "effort_result" => effort_result(c, ind, i, cfg),
        "stoch_rsi" => stoch_rsi_entry(c, ind, i, cfg),
        "connors_rsi" => connors_rsi_entry(c, ind, i, cfg),
        "cci_mean_rev" => cci_mean_rev(c, ind, i, cfg),
        "force_index" => force_index_entry(c, ind, i, cfg),
        // Control strategy — known-good from RUN1, verifies engine correctness
        "ctrl_mean_rev" => ctrl_mean_rev(c, ind, i, cfg),
        _ => false,
    }
}

/// Generate all strategy configs (each strategy × param grid × z-filter).
/// z_filter: 0.0 = no filter, -0.5 = mild, -1.0 = moderate, -1.5 = strict
pub fn all_configs() -> Vec<StratConfig> {
    let mut base = Vec::new();

    // 1. Morning Star
    for &r in &[0.3, 0.5] {
        base.push(("morning_star", r, 0.0, 0.0));
    }
    // 2. Three White Soldiers
    for &b in &[0.5, 0.7] {
        base.push(("three_soldiers", b, 0.0, 0.0));
    }
    // 3. Tweezer Bottom
    for &t in &[0.001, 0.003] {
        base.push(("tweezer_bottom", t, 0.0, 0.0));
    }
    // 4. Inside Bar Breakout
    for &b in &[0.001, 0.003] {
        base.push(("inside_bar", b, 0.0, 0.0));
    }
    // 5. Marubozu
    for &b in &[0.85, 0.95] {
        base.push(("marubozu", b, 0.0, 0.0));
    }
    // 6. Double Bottom
    for &lb in &[20.0, 40.0] {
        base.push(("double_bottom", lb, 0.01, 0.0));
    }
    // 7. Displacement Candle
    for &m in &[1.5, 2.5] {
        base.push(("displacement", m, 0.0, 0.0));
    }
    // 8. NR Breakout
    for &p in &[4.0, 7.0] {
        base.push(("nr_breakout", p, 0.0, 0.0));
    }
    // 9. OBV Divergence
    for &lb in &[10.0, 20.0] {
        base.push(("obv_divergence", lb, 0.0, 0.0));
    }
    // 10. CMF Reversal
    for &t in &[-0.15, -0.25] {
        base.push(("cmf_reversal", t, 0.0, 0.0));
    }
    // 11. MFI Divergence
    for &t in &[20.0, 30.0] {
        base.push(("mfi_divergence", t, 0.0, 0.0));
    }
    // 12. VSA Absorption
    for &vm in &[1.5, 2.0] {
        base.push(("vsa_absorption", vm, 0.5, 0.0));
    }
    // 13. Volume Dry Up → Spike
    for &dt in &[0.5, 0.7] {
        base.push(("vol_dryup_spike", dt, 2.0, 0.0));
    }
    // 14. Effort vs Result (VSA): vol_mult × max_spread_ratio
    for &vm in &[1.2, 1.5, 2.0] {
        for &sr in &[0.4, 0.5, 0.6, 0.7] {
            base.push(("effort_result", vm, sr, 0.0));
        }
    }
    // 15. Stochastic RSI (%K/%D crossover in oversold zone)
    for &t in &[10.0, 20.0, 30.0, 40.0] {
        base.push(("stoch_rsi", t, 0.0, 0.0));
    }
    // 16. Connors RSI
    for &t in &[10.0, 20.0] {
        base.push(("connors_rsi", t, 0.0, 0.0));
    }
    // 17. CCI Mean Reversion
    for &p in &[14.0, 20.0] {
        for &t in &[-100.0, -200.0] {
            base.push(("cci_mean_rev", p, t, 0.0));
        }
    }
    // 18. Force Index
    for &p in &[2.0, 13.0] {
        base.push(("force_index", p, 0.0, 0.0));
    }

    // Control: plain mean reversion (Z < -1.5, vol > 1.2× avg) — should get 70%+ WR
    base.push(("ctrl_mean_rev", 1.5, 1.2, 0.0));

    // Expand with z_filter variants: no filter, mild, moderate, strict
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

/// 1. Morning Star: bearish candle, small body doji, bullish candle
fn morning_star(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 3 { return false; }
    let max_mid_ratio = cfg.p1;

    // Candle i-2: bearish, decent body
    let body2 = c.close[i - 2] - c.open[i - 2]; // negative if bearish
    let range2 = c.high[i - 2] - c.low[i - 2];
    if range2 <= 0.0 || body2 >= 0.0 { return false; } // must be bearish
    if body2.abs() / range2 < 0.5 { return false; } // decent body

    // Candle i-1: small body (doji-like)
    let body1 = (c.close[i - 1] - c.open[i - 1]).abs();
    let range1 = c.high[i - 1] - c.low[i - 1];
    if range1 <= 0.0 { return false; }
    if body1 / range1 > max_mid_ratio { return false; } // small body

    // Candle i: bullish, decent body
    let body0 = c.close[i] - c.open[i]; // positive if bullish
    let range0 = c.high[i] - c.low[i];
    if range0 <= 0.0 || body0 <= 0.0 { return false; }
    if body0 / range0 < 0.5 { return false; }

    // Close of bullish candle should be above midpoint of bearish candle
    let mid = (c.open[i - 2] + c.close[i - 2]) / 2.0;
    c.close[i] > mid
}

/// 2. Three White Soldiers: 3 consecutive bullish candles with substantial bodies
fn three_soldiers(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 3 { return false; }
    let min_body = cfg.p1;

    for k in 0..3 {
        let idx = i - 2 + k;
        let body = c.close[idx] - c.open[idx];
        let range = c.high[idx] - c.low[idx];
        if range <= 0.0 || body <= 0.0 { return false; } // must be bullish
        if body / range < min_body { return false; }
    }
    // Each close higher than previous
    c.close[i - 1] > c.close[i - 2] && c.close[i] > c.close[i - 1]
}

/// 3. Tweezer Bottom: two candles with very similar lows, first bearish, second bullish
fn tweezer_bottom(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 2 { return false; }
    let tol = cfg.p1; // as fraction of price

    let prev_bearish = c.close[i - 1] < c.open[i - 1];
    let curr_bullish = c.close[i] > c.open[i];
    if !prev_bearish || !curr_bullish { return false; }

    let low_diff = (c.low[i] - c.low[i - 1]).abs();
    let avg_price = (c.close[i] + c.close[i - 1]) / 2.0;
    if avg_price <= 0.0 { return false; }

    low_diff / avg_price < tol
}

/// 4. Inside Bar Breakout: bar i-1 is inside bar i-2, bar i breaks above high of i-1
fn inside_bar(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 3 { return false; }
    let breakout_pct = cfg.p1;

    // i-1 is inside i-2
    let inside = c.high[i - 1] <= c.high[i - 2] && c.low[i - 1] >= c.low[i - 2];
    if !inside { return false; }

    // i breaks above high of i-1
    let threshold = c.high[i - 1] * (1.0 + breakout_pct);
    c.close[i] > threshold
}

/// 5. Marubozu: bullish candle with body comprising nearly all of the range
fn marubozu(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 1 { return false; }
    let min_ratio = cfg.p1;

    let body = c.close[i] - c.open[i]; // positive = bullish
    let range = c.high[i] - c.low[i];
    if range <= 0.0 || body <= 0.0 { return false; }

    body / range >= min_ratio
}

/// 6. Double Bottom: W pattern — two lows within tolerance in lookback window
fn double_bottom(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    let lookback = cfg.p1 as usize;
    let tol = cfg.p2;
    if i < lookback + 5 { return false; }

    // Find lowest low in lookback window
    let start = i - lookback;
    let mut min_low = f64::INFINITY;
    let mut min_idx = start;
    for j in start..i {
        if c.low[j] < min_low {
            min_low = c.low[j];
            min_idx = j;
        }
    }

    // Current low should be near that previous low (forming the second bottom)
    // But there should be a higher point between them (the "W" middle)
    if min_idx >= i - 3 { return false; } // first bottom must not be too recent
    if min_low <= 0.0 { return false; }

    let low_diff = (c.low[i] - min_low).abs() / min_low;
    if low_diff > tol { return false; }

    // Check that there's a higher point between first bottom and now
    let mut max_between = f64::NEG_INFINITY;
    for j in (min_idx + 1)..i {
        if c.high[j] > max_between { max_between = c.high[j]; }
    }

    // The middle high must be at least 1% above the bottoms
    max_between > min_low * 1.01 && c.close[i] > c.open[i] // current candle bullish
}

/// 7. Displacement Candle: large body candle (body > N× average body), bullish
fn displacement(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    let mult = cfg.p1;

    let body = c.close[i] - c.open[i]; // positive = bullish
    if body <= 0.0 { return false; }

    if ind.avg_body20[i].is_nan() || ind.avg_body20[i] <= 0.0 { return false; }

    body > ind.avg_body20[i] * mult
}

/// 8. NR4/NR7 Breakout: narrowest range in last N bars, then bullish breakout
fn nr_breakout(c: &Candles, _ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    let period = cfg.p1 as usize;
    if i < period + 1 { return false; }

    // Check if bar i-1 had the narrowest range in the last `period` bars
    let range_prev = c.high[i - 1] - c.low[i - 1];
    if range_prev <= 0.0 { return false; }

    let mut is_narrowest = true;
    for k in 2..=period {
        let r = c.high[i - k] - c.low[i - k];
        if r <= range_prev {
            is_narrowest = false;
            break;
        }
    }
    if !is_narrowest { return false; }

    // Current candle breaks above the narrow bar's high
    c.close[i] > c.high[i - 1]
}

/// 9. OBV Divergence: price makes new low but OBV doesn't (bullish divergence)
fn obv_divergence(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    let lookback = cfg.p1 as usize;
    if i < lookback + 1 { return false; }

    let start = i - lookback;

    // Find if current price is near the lowest in lookback
    let mut min_price = f64::INFINITY;
    let mut min_price_idx = start;
    for j in start..i {
        if c.low[j] < min_price {
            min_price = c.low[j];
            min_price_idx = j;
        }
    }

    // Current low is at or below previous low (price making new low)
    if c.low[i] > min_price * 1.005 { return false; }

    // But OBV at current should be HIGHER than OBV at that previous low (divergence)
    ind.obv[i] > ind.obv[min_price_idx]
}

/// 10. CMF Reversal: CMF deeply negative then turning up (bearish exhaustion)
fn cmf_reversal(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 21 { return false; }
    let threshold = cfg.p1; // negative

    if ind.cmf20[i].is_nan() || ind.cmf20[i - 1].is_nan() { return false; }

    // CMF was below threshold and is now improving
    ind.cmf20[i - 1] < threshold && ind.cmf20[i] > ind.cmf20[i - 1]
}

/// 11. MFI Divergence: price new low but MFI not (or MFI oversold bounce)
fn mfi_divergence(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    let threshold = cfg.p1;

    if ind.mfi14[i].is_nan() || ind.mfi14[i - 1].is_nan() { return false; }

    // Simple version: MFI oversold and bouncing
    ind.mfi14[i - 1] < threshold && ind.mfi14[i] > ind.mfi14[i - 1]
}

/// 12. VSA Absorption: high volume + small spread = smart money absorbing
fn vsa_absorption(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    let vol_mult = cfg.p1;
    let max_spread_ratio = cfg.p2;

    if ind.vol_ratio[i].is_nan() || ind.avg_range20[i].is_nan() { return false; }
    if ind.avg_range20[i] <= 0.0 { return false; }

    let spread = c.high[i] - c.low[i];
    let spread_ratio = spread / ind.avg_range20[i];

    // High volume but small spread (absorption) in a down move
    ind.vol_ratio[i] > vol_mult && spread_ratio < max_spread_ratio && c.close[i] < c.open[i]
}

/// 13. Volume Dry Up → Spike: low volume period then sudden volume spike
fn vol_dryup_spike(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 6 { return false; }
    let dry_threshold = cfg.p1;
    let spike_mult = cfg.p2;

    if ind.vol_ratio[i].is_nan() { return false; }

    // Check previous 3-5 bars had low volume (dry up)
    let mut dry_count = 0;
    for k in 1..=5 {
        if i < k { break; }
        if !ind.vol_ratio[i - k].is_nan() && ind.vol_ratio[i - k] < dry_threshold {
            dry_count += 1;
        }
    }

    // At least 3 of last 5 bars were dry, and current bar is a spike with bullish close
    dry_count >= 3 && ind.vol_ratio[i] > spike_mult && c.close[i] > c.open[i]
}

/// 14. Effort vs Result (VSA): high volume + narrow spread + close near high = absorption
/// p1 = volume multiplier (e.g. 1.5), p2 = max spread ratio vs avg (e.g. 0.4-0.5)
fn effort_result(c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }
    let vol_mult = cfg.p1;
    let max_spread_ratio = cfg.p2; // spread must be < this × avg_range20

    if ind.vol_ratio[i].is_nan() || ind.avg_range20[i].is_nan() { return false; }
    if ind.avg_range20[i] <= 0.0 { return false; }

    let spread = c.high[i] - c.low[i];
    let spread_ratio = spread / ind.avg_range20[i];

    // Close position within bar: 0=low, 1=high
    let close_pos = if spread > 0.0 {
        (c.close[i] - c.low[i]) / spread
    } else {
        0.5
    };

    // High volume + narrow spread (relative) + close near high = bullish absorption
    ind.vol_ratio[i] > vol_mult
        && spread_ratio < max_spread_ratio
        && close_pos > 0.5  // close in upper half of bar (buyers winning)
}

/// 15. Stochastic RSI: %K/%D crossover in oversold zone
/// p1 = oversold threshold (e.g. 20)
fn stoch_rsi_entry(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 30 { return false; }
    let threshold = cfg.p1;

    if ind.stoch_rsi_k[i].is_nan() || ind.stoch_rsi_d[i].is_nan() { return false; }
    if ind.stoch_rsi_k[i - 1].is_nan() || ind.stoch_rsi_d[i - 1].is_nan() { return false; }

    // %K crosses above %D while both are in oversold zone
    ind.stoch_rsi_k[i - 1] <= ind.stoch_rsi_d[i - 1]  // was below or equal
        && ind.stoch_rsi_k[i] > ind.stoch_rsi_d[i]      // now above (crossover)
        && ind.stoch_rsi_k[i] < threshold                // still in oversold zone
        && ind.stoch_rsi_d[i] < threshold
}

/// 16. Connors RSI: oversold entry
fn connors_rsi_entry(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 105 { return false; }
    let threshold = cfg.p1;

    if ind.connors_rsi[i].is_nan() || ind.connors_rsi[i - 1].is_nan() { return false; }

    // ConnorsRSI below threshold and turning up
    ind.connors_rsi[i - 1] < threshold && ind.connors_rsi[i] > ind.connors_rsi[i - 1]
}

/// 17. CCI Mean Reversion: CCI deeply negative, entry on bounce
fn cci_mean_rev(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 25 { return false; }
    let period = cfg.p1 as usize;
    let threshold = cfg.p2; // negative value like -100

    let cci = if period == 14 { &ind.cci14 } else { &ind.cci20 };
    if i < 1 { return false; }
    if cci[i].is_nan() || cci[i - 1].is_nan() { return false; }

    // CCI was below threshold and is now rising
    cci[i - 1] < threshold && cci[i] > cci[i - 1]
}

/// 18. Force Index: negative force (selling pressure) exhaustion, then reversal
fn force_index_entry(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 15 { return false; }
    let period = cfg.p1 as usize;

    let force = if period == 2 { &ind.force2 } else { &ind.force13 };
    if i < 2 { return false; }
    if force[i].is_nan() || force[i - 1].is_nan() || force[i - 2].is_nan() { return false; }

    // Force was negative (selling) and crossed back above zero (buyers returning)
    force[i - 1] < 0.0 && force[i] > 0.0
}

/// Control: Z-score mean reversion (matches COINCLAW mean_rev strategy).
/// p1 = z_threshold (e.g., 1.5), p2 = vol_multiplier (e.g., 1.2)
/// NOTE: z_filter on StratConfig is applied BEFORE this, so if z_filter=-1.5 AND
/// this also checks Z < -1.5, both must pass. Use z_filter=0.0 for this strategy.
fn ctrl_mean_rev(_c: &Candles, ind: &Indicators, i: usize, cfg: &StratConfig) -> bool {
    if i < 20 { return false; }

    if ind.z_score[i].is_nan() || ind.vol_ratio[i].is_nan() { return false; }

    ind.z_score[i] < -cfg.p1 && ind.vol_ratio[i] > cfg.p2
}
