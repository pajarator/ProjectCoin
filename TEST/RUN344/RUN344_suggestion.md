# RUN344 — SuperTrend with MACD Histogram Divergence

## Hypothesis

**Mechanism**: SuperTrend uses ATR to create a trailing stop that adapts to volatility. When price crosses above the SuperTrend band → bullish. When price crosses below → bearish. Combine with MACD histogram divergence: when SuperTrend gives a signal AND MACD histogram shows divergence in the same direction → high-conviction entry. When they disagree (SuperTrend up but MACD histogram bearish divergence) → suppress signal.

**Why not duplicate**: RUN191 uses SuperTrend as standalone. This RUN specifically combines SuperTrend signals with MACD histogram divergence as a confirmation filter. The SuperTrend gives the directional bias, MACD histogram divergence gives the timing/conviction filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN344: SuperTrend with MACD Histogram Divergence ──────────────────────────────
// supertrend = ATR-based trailing stop with period × multiplier
// macd_histogram = MACD - Signal
// macd_bearish_div: price higher_high AND macd_hist lower_high
// macd_bullish_div: price lower_low AND macd_hist higher_low
//
// LONG: supertrend_cross_up AND NOT macd_bearish_div (no divergence conflict)
// SHORT: supertrend_cross_down AND NOT macd_bullish_div (no divergence conflict)
// Confluence: if MACD divergence confirms SuperTrend direction = stronger signal

pub const ST_MACD_ENABLED: bool = true;
pub const ST_MACD_ATR_PERIOD: usize = 10;
pub const ST_MACD_ATR_MULT: f64 = 3.0;
pub const ST_MACD_MACD_FAST: usize = 12;
pub const ST_MACD_MACD_SLOW: usize = 26;
pub const ST_MACD_MACD_SIGNAL: usize = 9;
pub const ST_MACD_DIV_LOOKBACK: usize = 20;
pub const ST_MACD_SL: f64 = 0.005;
pub const ST_MACD_TP: f64 = 0.004;
pub const ST_MACD_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run344_1_st_macd_backtest.py)
2. **Walk-forward** (run344_2_st_macd_wf.py)
3. **Combined** (run344_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 2.0 / 3.0 / 4.0
- ATR_PERIOD sweep: 7 / 10 / 14
- MACD_FAST sweep: 8 / 12 / 16
- MACD_SLOW sweep: 20 / 26 / 34
