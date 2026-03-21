# RUN481 — Linear Regression Slope with Volume Trend Divergence

## Hypothesis

**Mechanism**: Linear Regression Slope measures the directional rate of price change over a period, providing a smoothed trend direction indicator. Volume Trend Divergence detects when price trend and volume trend disagree — if price is rising but volume is declining, the trend lacks conviction. When Linear Regression Slope confirms direction AND volume trend diverges against it, entries have both price momentum and volume-backed conviction check.

**Why not duplicate**: RUN417 uses Linear Regression Slope with Volume Trend Confirmation. This RUN uses Volume TREND DIVERGENCE instead — distinct mechanism is using divergence between price and volume trends (they disagree) rather than confirmation (they agree). This catches exhaustion and reversal signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN481: Linear Regression Slope with Volume Trend Divergence ─────────────────────────────────
// lin_reg_slope: slope of linear regression line over period
// lin_reg_cross: slope crosses above/below 0 threshold
// vol_trend: ema of volume indicating trend direction
// vol_trend_divergence: price and volume trends moving in opposite directions
// LONG: lin_reg_cross bullish AND vol_trend_divergence (price up, vol down)
// SHORT: lin_reg_cross bearish AND vol_trend_divergence (price down, vol up)

pub const LINREG_VOLDIV_ENABLED: bool = true;
pub const LINREG_VOLDIV_LINREG_PERIOD: usize = 20;
pub const LINREG_VOLDIV_SLOPE_THRESH: f64 = 0.0001;
pub const LINREG_VOLDIV_VOL_PERIOD: usize = 20;
pub const LINREG_VOLDIV_VOL_EMA_PERIOD: usize = 10;
pub const LINREG_VOLDIV_SL: f64 = 0.005;
pub const LINREG_VOLDIV_TP: f64 = 0.004;
pub const LINREG_VOLDIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run481_1_linreg_voldiv_backtest.py)
2. **Walk-forward** (run481_2_linreg_voldiv_wf.py)
3. **Combined** (run481_3_combined.py)

## Out-of-Sample Testing

- LINREG_PERIOD sweep: 14 / 20 / 30
- SLOPE_THRESH sweep: 0.00005 / 0.0001 / 0.0002
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_EMA_PERIOD sweep: 7 / 10 / 14
