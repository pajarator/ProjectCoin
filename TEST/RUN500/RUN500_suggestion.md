# RUN500 — Z-Score with Volume Profile POC

## Hypothesis

**Mechanism**: Z-Score measures how many standard deviations price is from its moving average, identifying statistical extremes. Volume Profile POC (Point of Control) identifies the price level with highest traded volume. When Z-Score reaches extreme AND price is at a POC level, the statistical extreme has volume-based structural confirmation. POC levels often act as reversal points when price reaches them at statistical extremes.

**Why not duplicate**: RUN474 uses Z-Score with VWAP Deviation Bands. This RUN uses Volume Profile POC instead — distinct mechanism is volume-based structural support/resistance versus VWAP bands. POC identifies the single most important price level by volume.

## Proposed Config Changes (config.rs)

```rust
// ── RUN500: Z-Score with Volume Profile POC ─────────────────────────────────
// zscore: (close - sma) / stdDev, measures statistical extreme
// zscore_cross: zscore crosses above/below extreme threshold
// vol_profile_poc: price level with highest volume traded
// price_at_poc: price within tolerance of POC
// LONG: zscore < -2 (oversold statistical extreme) AND price at POC support
// SHORT: zscore > +2 (overbought statistical extreme) AND price at POC resistance

pub const ZSCORE_POC_ENABLED: bool = true;
pub const ZSCORE_POC_ZSCORE_PERIOD: usize = 20;
pub const ZSCORE_POC_ZSCORE_THRESH: f64 = 2.0;
pub const ZSCORE_POC_VP_PERIOD: usize = 20;
pub const ZSCORE_POC_POC_TOLERANCE: f64 = 0.001;
pub const ZSCORE_POC_SL: f64 = 0.005;
pub const ZSCORE_POC_TP: f64 = 0.004;
pub const ZSCORE_POC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run500_1_zscore_poc_backtest.py)
2. **Walk-forward** (run500_2_zscore_poc_wf.py)
3. **Combined** (run500_3_combined.py)

## Out-of-Sample Testing

- ZSCORE_PERIOD sweep: 14 / 20 / 30
- ZSCORE_THRESH sweep: 1.5 / 2.0 / 2.5
- VP_PERIOD sweep: 14 / 20 / 30
- POC_TOLERANCE sweep: 0.0005 / 0.001 / 0.002
