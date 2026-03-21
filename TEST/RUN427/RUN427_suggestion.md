# RUN427 — MACD Histogram Slope with Bollinger Band Width Compression

## Hypothesis

**Mechanism**: The MACD Histogram shows the difference between MACD and its signal line, visually representing momentum acceleration and deceleration. The slope of the histogram (rising vs falling) indicates whether momentum is building or fading. Bollinger Band Width Compression (squeeze) indicates low volatility. When the MACD Histogram slope is steepening (strong momentum building) AND BB Width is compressing (low volatility), the squeeze is being loaded for an explosive release in the direction of the momentum.

**Why not duplicate**: RUN361 uses MACD Histogram Slope with RSI Filter. RUN398 uses Stochastic with BB Squeeze. This RUN specifically uses MACD Histogram slope direction (acceleration/deceleration of momentum) combined with BB squeeze — the distinct mechanism is using MACD histogram slope steepness to measure momentum building while waiting for the squeeze to release.

## Proposed Config Changes (config.rs)

```rust
// ── RUN427: MACD Histogram Slope with Bollinger Band Width Compression ────────────────────────
// macd_histogram = MACD - MACD_signal
// hist_slope = change in histogram over SLOPE_PERIOD
// steepening: hist_slope > SLOPE_THRESH (momentum building rapidly)
// bb_width = (bb_upper - bb_lower) / bb_middle
// squeeze: bb_width < BB_SQUEEZE_THRESH (compressed volatility)
// squeeze_release: bb_width expanding after being compressed
// LONG: hist_steepening bullish AND squeeze_release to upside
// SHORT: hist_steepening bearish AND squeeze_release to downside

pub const MACDHST_BBW_ENABLED: bool = true;
pub const MACDHST_BBW_FAST_PERIOD: usize = 12;
pub const MACDHST_BBW_SLOW_PERIOD: usize = 26;
pub const MACDHST_BBW_SIGNAL_PERIOD: usize = 9;
pub const MACDHST_BBW_SLOPE_PERIOD: usize = 3;
pub const MACDHST_BBW_SLOPE_THRESH: f64 = 0.001;
pub const MACDHST_BBW_BB_PERIOD: usize = 20;
pub const MACDHST_BBW_BB_STD: f64 = 2.0;
pub const MACDHST_BBW_SQUEEZE_THRESH: f64 = 0.05;
pub const MACDHST_BBW_SL: f64 = 0.005;
pub const MACDHST_BBW_TP: f64 = 0.004;
pub const MACDHST_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run427_1_macdhst_bbw_backtest.py)
2. **Walk-forward** (run427_2_macdhst_bbw_wf.py)
3. **Combined** (run427_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 8 / 12 / 16
- SLOW_PERIOD sweep: 21 / 26 / 30
- SLOPE_PERIOD sweep: 2 / 3 / 5
- SQUEEZE_THRESH sweep: 0.04 / 0.05 / 0.06
