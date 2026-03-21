# RUN486 — MACD Histogram Slope with Choppiness Index

## Hypothesis

**Mechanism**: MACD Histogram Slope measures the rate of change of the MACD histogram, identifying momentum acceleration/deceleration. Choppiness Index (CI) distinguishes trending from ranging markets — low CI (<38) indicates trending conditions where trend-following strategies work. When MACD histogram slope is steepening AND CI indicates trending conditions, entries have both momentum acceleration and favorable market regime.

**Why not duplicate**: RUN427 uses MACD Histogram Slope with Bollinger Band Width Compression. This RUN uses Choppiness Index instead — distinct mechanism is CI as a regime filter versus BB Width as a volatility expansion measure. CI specifically addresses whether the market is tradeable for trend strategies.

## Proposed Config Changes (config.rs)

```rust
// ── RUN486: MACD Histogram Slope with Choppiness Index ─────────────────────────────────
// macd_histogram: macd_line - signal_line visualized as histogram
// macd_hist_slope: rate of change of histogram bars (accelerating/decelerating)
// choppiness_index: ci value indicating trending vs ranging
// LONG: macd_hist_slope > 0 AND rising (histogram bars growing) AND ci < 45
// SHORT: macd_hist_slope < 0 AND falling (histogram bars shrinking) AND ci < 45

pub const MACDHS_CI_ENABLED: bool = true;
pub const MACDHS_CI_MACD_FAST: usize = 12;
pub const MACDHS_CI_MACD_SLOW: usize = 26;
pub const MACDHS_CI_MACD_SIGNAL: usize = 9;
pub const MACDHS_CI_SLOPE_PERIOD: usize = 5;
pub const MACDHS_CI_CI_PERIOD: usize = 14;
pub const MACDHS_CI_CI_THRESH: f64 = 45.0;
pub const MACDHS_CI_SL: f64 = 0.005;
pub const MACDHS_CI_TP: f64 = 0.004;
pub const MACDHS_CI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run486_1_macdhs_ci_backtest.py)
2. **Walk-forward** (run486_2_macdhs_ci_wf.py)
3. **Combined** (run486_3_combined.py)

## Out-of-Sample Testing

- MACD_FAST sweep: 10 / 12 / 15
- MACD_SLOW sweep: 20 / 26 / 30
- MACD_SIGNAL sweep: 7 / 9 / 12
- SLOPE_PERIOD sweep: 3 / 5 / 7
- CI_PERIOD sweep: 10 / 14 / 20
- CI_THRESH sweep: 40 / 45 / 50
