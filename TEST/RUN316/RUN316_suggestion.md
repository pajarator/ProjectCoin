# RUN316 — Linear Regression Slope Percentile: Trend Persistence Rank

## Hypothesis

**Mechanism**: Linear regression slope measures the direction and strength of the trend (slope of best-fit line through N bars). Compute the percentile rank of current slope against its own historical distribution. A slope at the 90th percentile = historically strong upward momentum. A slope at the 10th percentile = historically strong downward momentum. Rank-based entry filters out weak signals — only enter when momentum is at historically extreme levels.

**Why not duplicate**: RUN232 uses linear regression slope but as a standalone signal. No RUN uses percentile ranking of linear regression slope. This distinction is key: percentile rank makes the signal adaptive to each coin's own historical slope distribution, rather than using fixed slope thresholds.

## Proposed Config Changes (config.rs)

```rust
// ── RUN316: Linear Regression Slope Percentile ────────────────────────────────
// lr_slope = slope of linear regression through N bars
// lr_slope_percentile = percentile_rank(lr_slope, lookback)
// LONG: lr_slope_percentile > UPPER_THRESH (strong upward momentum)
// SHORT: lr_slope_percentile < LOWER_THRESH (strong downward momentum)
// Confirm: require percentile also crosses threshold (not just at extreme)

pub const LRS_PCT_ENABLED: bool = true;
pub const LRS_PCT_PERIOD: usize = 20;         // regression lookback
pub const LRS_PCT_LOOKBACK: usize = 100;     // percentile lookback
pub const LRS_PCT_UPPER: f64 = 85.0;        // 85th percentile = strong up
pub const LRS_PCT_LOWER: f64 = 15.0;        // 15th percentile = strong down
pub const LRS_PCT_SL: f64 = 0.005;
pub const LRS_PCT_TP: f64 = 0.004;
pub const LRS_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run316_1_lrs_pct_backtest.py)
2. **Walk-forward** (run316_2_lrs_pct_wf.py)
3. **Combined** (run316_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- LOOKBACK sweep: 50 / 100 / 200
- UPPER sweep: 80 / 85 / 90
- LOWER sweep: 10 / 15 / 20
