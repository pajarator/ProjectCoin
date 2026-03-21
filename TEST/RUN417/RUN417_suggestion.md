# RUN417 — Linear Regression Slope with Volume Trend Confirmation

## Hypothesis

**Mechanism**: Linear Regression Slope measures the direction and steepness of the price trend over a period. Unlike simple moving averages, it's mathematically precise and less prone to lag. Volume Trend measures whether volume is increasing or decreasing in the direction of the trend. When Linear Regression Slope is positive (uptrend) AND Volume Trend confirms by rising in the same direction, the trend has mathematical slope AND volume-backed conviction.

**Why not duplicate**: RUN316 uses Linear Regression Slope Percentile Rank. This RUN specifically uses Linear Regression Slope with Volume Trend Confirmation — the distinct mechanism is requiring volume to confirm the slope direction, filtering out slopes that occur without volume backing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN417: Linear Regression Slope with Volume Trend Confirmation ─────────────────────────────
// lin_reg_slope = slope of linear regression line over period
// positive_slope = lin_reg_slope > SLOPE_THRESH (uptrend)
// volume_trend = slope of volume over same period
// vol_confirms = volume_trend > 0 for long, volume_trend < 0 for short
// LONG: lin_reg_slope > SLOPE_THRESH AND volume_trend confirms
// SHORT: lin_reg_slope < -SLOPE_THRESH AND volume_trend confirms

pub const LINREG_VOL_ENABLED: bool = true;
pub const LINREG_VOL_REG_PERIOD: usize = 21;
pub const LINREG_VOL_SLOPE_THRESH: f64 = 0.001;  // minimum slope for trend
pub const LINREG_VOL_VOL_PERIOD: usize = 21;
pub const LINREG_VOL_SL: f64 = 0.005;
pub const LINREG_VOL_TP: f64 = 0.004;
pub const LINREG_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run417_1_linreg_vol_backtest.py)
2. **Walk-forward** (run417_2_linreg_vol_wf.py)
3. **Combined** (run417_3_combined.py)

## Out-of-Sample Testing

- REG_PERIOD sweep: 14 / 21 / 30
- SLOPE_THRESH sweep: 0.0005 / 0.001 / 0.002
- VOL_PERIOD sweep: 14 / 21 / 30
