# RUN191 — SuperTrend: ATR-Based Trend Following with Dynamic Stops

## Hypothesis

**Mechanism**: SuperTrend = price subject to ATR-based bands that flip polarity when volatility changes. Upper band = HL2 (high+low)/2 + ATR × factor. Lower band = HL2 - ATR × factor. When close crosses above the lower band → trend flips bullish (LONG). When close crosses below the upper band → trend flips bearish (SHORT). The ATR multiplier controls sensitivity: lower = more flips, higher = fewer flips.

**Why not duplicate**: No prior RUN uses SuperTrend. All prior ATR-based signals (RUN181, RUN26, RUN188) use ATR for stop calculation, not for trend direction. SuperTrend is a complete trend-following system, not just a stop mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN191: SuperTrend ───────────────────────────────────────────────────
// hl2 = (high + low) / 2
// upper_band = hl2 + ATR(atr_period) × mult
// lower_band = hl2 - ATR(atr_period) × mult
// Supertrend value = continuous line that flips when price crosses opposite band
// Close above SuperTrend → bullish mode
// Close below SuperTrend → bearish mode
// LONG entry: SuperTrend flips from bearish to bullish
// SHORT entry: SuperTrend flips from bullish to bearish

pub const SUPERTREND_ENABLED: bool = true;
pub const SUPERTREND_ATR_PERIOD: usize = 10;   // ATR period
pub const SUPERTREND_MULT: f64 = 3.0;          // ATR multiplier for band width
pub const SUPERTREND_SL: f64 = 0.005;          // initial stop loss
pub const SUPERTREND_TP: f64 = 0.004;          // take profit
pub const SUPERTREND_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn supertrend(highs: &[f64], lows: &[f64], closes: &[f64],
                  atr_period: usize, mult: f64) -> (f64, f64, bool) {
    // HL2 = (high + low) / 2
    let hl2 = (highs[highs.len()-1] + lows[lows.len()-1]) / 2.0;
    let atr_val = atr(highs, lows, closes, atr_period);

    let upper_band = hl2 + atr_val * mult;
    let lower_band = hl2 - atr_val * mult;

    // Simple current-value SuperTrend (full ST calculation is iterative)
    let close = closes[closes.len()-1];
    let bullish = close > lower_band;
    let st_value = if bullish { lower_band } else { upper_band };

    (st_value, atr_val, bullish)
}
```

---

## Validation Method

1. **Historical backtest** (run191_1_st_backtest.py)
2. **Walk-forward** (run191_2_st_wf.py)
3. **Combined** (run191_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 7 / 10 / 14 / 20
- MULT sweep: 2.0 / 3.0 / 4.0 / 5.0
