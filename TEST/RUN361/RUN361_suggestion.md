# RUN361 — MACD Histogram Slope with RSI Overbought Filter

## Hypothesis

**Mechanism**: MACD histogram slope (RUN277 already tested slope) measures momentum acceleration. This RUN adds an RSI filter: only take LONG when RSI < 65 (not overbought, room to run) and only take SHORT when RSI > 35 (not oversold, room to fall). The RSI filter prevents entering at the end of a move when there's no room left for the trend to continue.

**Why not duplicate**: RUN277 uses MACD histogram slope without RSI filter. This RUN specifically adds the RSI room filter — the distinct mechanism is only trading momentum signals when there's sufficient RSI room for the move to continue.

## Proposed Config Changes (config.rs)

```rust
// ── RUN361: MACD Histogram Slope with RSI Overbought Filter ────────────────────────────
// macd_histogram = MACD - Signal
// hist_slope = histogram - histogram[N]
// hist_slope_cross_up = hist_slope crosses above 0
// hist_slope_cross_down = hist_slope crosses below 0
// LONG: hist_slope_cross_up AND RSI < RSI_MAX (not overbought)
// SHORT: hist_slope_cross_down AND RSI > RSI_MIN (not oversold)

pub const HIST_SLOPE_RSI_ENABLED: bool = true;
pub const HIST_SLOPE_RSI_FAST: usize = 12;
pub const HIST_SLOPE_RSI_SLOW: usize = 26;
pub const HIST_SLOPE_RSI_SIGNAL: usize = 9;
pub const HIST_SLOPE_RSI_LOOKBACK: usize = 3;
pub const HIST_SLOPE_RSI_RSI_MAX: f64 = 65.0;
pub const HIST_SLOPE_RSI_RSI_MIN: f64 = 35.0;
pub const HIST_SLOPE_RSI_SL: f64 = 0.005;
pub const HIST_SLOPE_RSI_TP: f64 = 0.004;
pub const HIST_SLOPE_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run361_1_hist_slope_rsi_backtest.py)
2. **Walk-forward** (run361_2_hist_slope_rsi_wf.py)
3. **Combined** (run361_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 2 / 3 / 5
- FAST sweep: 8 / 12 / 16
- RSI_MAX sweep: 60 / 65 / 70
- RSI_MIN sweep: 30 / 35 / 40
