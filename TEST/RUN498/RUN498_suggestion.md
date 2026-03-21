# RUN498 — MACD Double Signal Line with Williams %R Extreme

## Hypothesis

**Mechanism**: MACD Double Signal Line uses two signal lines (fast and slow) to create a more nuanced entry timing mechanism than standard single signal line MACD. Williams %R Extreme detects when price is at extreme levels relative to the recent high/low range. When the MACD double signal lines align AND Williams %R confirms extreme reading, entries have both MACD momentum timing and statistical extreme confirmation.

**Why not duplicate**: RUN437 uses MACD Double Signal Line Crossover with Volume Confirmation. This RUN uses Williams %R Extreme instead — distinct mechanism is Williams %R as the confirming oscillator versus volume. Williams %R provides direct overbought/oversold extremes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN498: MACD Double Signal Line with Williams %R Extreme ─────────────────────────────────
// macd_double_signal: macd with two signal lines (fast and slow)
// macd_fast_cross_slow: fast signal crosses slow signal
// williams_r: close position relative to highest_high/lowest_low range
// williams_extreme: williams_r < -80 (oversold) or > -20 (overbought)
// LONG: macd_fast_cross_slow bullish AND williams_r < -80
// SHORT: macd_fast_cross_slow bearish AND williams_r > -20

pub const MACDDouble_WILLR_ENABLED: bool = true;
pub const MACDDouble_WILLR_MACD_FAST: usize = 12;
pub const MACDDouble_WILLR_MACD_SLOW: usize = 26;
pub const MACDDouble_WILLR_SIGNAL_FAST: usize = 5;
pub const MACDDouble_WILLR_SIGNAL_SLOW: usize = 15;
pub const MACDDouble_WILLR_WILLR_PERIOD: usize = 14;
pub const MACDDouble_WILLR_WILLR_OVERSOLD: f64 = -80.0;
pub const MACDDouble_WILLR_WILLR_OVERBOUGHT: f64 = -20.0;
pub const MACDDouble_WILLR_SL: f64 = 0.005;
pub const MACDDouble_WILLR_TP: f64 = 0.004;
pub const MACDDouble_WILLR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run498_1_macddouble_willr_backtest.py)
2. **Walk-forward** (run498_2_macddouble_willr_wf.py)
3. **Combined** (run498_3_combined.py)

## Out-of-Sample Testing

- MACD_FAST sweep: 10 / 12 / 15
- MACD_SLOW sweep: 20 / 26 / 30
- SIGNAL_FAST sweep: 3 / 5 / 7
- SIGNAL_SLOW sweep: 12 / 15 / 18
- WILLR_PERIOD sweep: 10 / 14 / 20
