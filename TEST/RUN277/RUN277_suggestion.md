# RUN277 — MACD Histogram Slope Reversal: Momentum Fading Detection

## Hypothesis

**Mechanism**: MACD Histogram slope = difference between current histogram value and prior N bars. When slope turns from positive to negative → momentum fading → exit LONG. When slope turns from negative to positive → momentum building → entry LONG. The slope reversal often precedes the MACD line crossover.

**Why not duplicate**: RUN249 uses MACD Histogram ROC. This RUN uses slope (simple difference) rather than rate of change percentage. Slope reversal is a distinct momentum fading signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN277: MACD Histogram Slope Reversal ─────────────────────────────────
// histogram = MACD_line - Signal_line
// hist_slope = histogram - histogram[N]
// hist_slope crosses 0 = momentum reversal
// LONG: hist_slope crosses above 0 (momentum improving)
// SHORT: hist_slope crosses below 0 (momentum deteriorating)

pub const HIST_SLOPE_ENABLED: bool = true;
pub const HIST_SLOPE_FAST: usize = 12;
pub const HIST_SLOPE_SLOW: usize = 26;
pub const HIST_SLOPE_SIGNAL: usize = 9;
pub const HIST_SLOPE_LOOKBACK: usize = 3;
pub const HIST_SLOPE_SL: f64 = 0.005;
pub const HIST_SLOPE_TP: f64 = 0.004;
pub const HIST_SLOPE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run277_1_hist_slope_backtest.py)
2. **Walk-forward** (run277_2_hist_slope_wf.py)
3. **Combined** (run277_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 2 / 3 / 5
- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
