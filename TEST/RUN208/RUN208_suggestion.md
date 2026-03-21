# RUN208 — VWAP Deviation Bands: Mean Reversion from Volume-Weighted Fair Value

## Hypothesis

**Mechanism**: VWAP = cumulative volume-weighted average price since session open. When price deviates significantly above VWAP (e.g., > 1.5% of VWAP) → extended, likely to revert short. When price deviates below VWAP → extended low, likely to revert long. The deviation is measured in standard deviations of the price-VWAP spread.

**Why not duplicate**: No prior RUN uses VWAP deviation. All prior VWAP RUNs use VWAP crossover or VWAP Reversion. VWAP Deviation Bands is distinct: it measures the *distance* from VWAP rather than the crossover, creating a fundamentally different mean-reversion signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN208: VWAP Deviation Bands ────────────────────────────────────────
// vwap = cumulative(close × volume) / cumulative(volume) (session-reset)
// deviation = (close - vwap) / vwap
// dev_ma = SMA(deviation, period)
// dev_std = stddev(deviation, period)
// upper_band = dev_ma + dev_std × mult
// lower_band = dev_ma - dev_std × mult
// LONG: deviation < lower_band → price too far below fair value
// SHORT: deviation > upper_band → price too far above fair value

pub const VWAP_DEV_ENABLED: bool = true;
pub const VWAP_DEV_PERIOD: usize = 20;      // rolling period for MA/std
pub const VWAP_DEV_MULT: f64 = 1.5;         // std dev multiplier for bands
pub const VWAP_DEV_SL: f64 = 0.005;
pub const VWAP_DEV_TP: f64 = 0.004;
pub const VWAP_DEV_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run208_1_vwap_dev_backtest.py)
2. **Walk-forward** (run208_2_vwap_dev_wf.py)
3. **Combined** (run208_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 40
- MULT sweep: 1.0 / 1.5 / 2.0 / 2.5
