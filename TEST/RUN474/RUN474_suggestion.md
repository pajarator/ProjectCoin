# RUN474 — Z-Score with VWAP Deviation Bands

## Hypothesis

**Mechanism**: Z-Score measures how many standard deviations price is from its moving average, identifying statistical extremes. VWAP Deviation Bands create dynamic support/resistance around the volume-weighted average price. When Z-Score reaches extreme levels AND price is at VWAP deviation band boundary, the statistical extreme has volume-weighted confirmation.

**Why not duplicate**: RUN413 uses Z-Score Convergence with Volume Divergence. This RUN uses VWAP Deviation Bands instead — distinct mechanism is VWAP-based deviation bands versus volume divergence. VWAP bands are volume-weighted support/resistance that adapt to intraday volume patterns.

## Proposed Config Changes (config.rs)

```rust
// ── RUN474: Z-Score with VWAP Deviation Bands ─────────────────────────────────
// zscore: (close - sma) / stdDev, measures statistical extreme
// zscore_cross: zscore crosses above/below thresholds (typically -2, 0, +2)
// vwap_dev_bands: vwap +/- multiples of stdDev of vwap deviations
// LONG: zscore < -2 (oversold) AND price near vwap_lower_band
// SHORT: zscore > +2 (overbought) AND price near vwap_upper_band

pub const ZSCORE_VWAP_ENABLED: bool = true;
pub const ZSCORE_VWAP_ZSCORE_PERIOD: usize = 20;
pub const ZSCORE_VWAP_ZSCORE_THRESH: f64 = 2.0;
pub const ZSCORE_VWAP_VWAP_PERIOD: usize = 14;
pub const ZSCORE_VWAP_BAND_MULT: f64 = 1.5;
pub const ZSCORE_VWAP_SL: f64 = 0.005;
pub const ZSCORE_VWAP_TP: f64 = 0.004;
pub const ZSCORE_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run474_1_zscore_vwap_backtest.py)
2. **Walk-forward** (run474_2_zscore_vwap_wf.py)
3. **Combined** (run474_3_combined.py)

## Out-of-Sample Testing

- ZSCORE_PERIOD sweep: 14 / 20 / 30
- ZSCORE_THRESH sweep: 1.5 / 2.0 / 2.5
- VWAP_PERIOD sweep: 10 / 14 / 20
- BAND_MULT sweep: 1.0 / 1.5 / 2.0
