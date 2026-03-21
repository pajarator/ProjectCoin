# RUN351 — Volume-Price Trend with Z-Score Confirmation

## Hypothesis

**Mechanism**: PVT (Price Volume Trend) = cumulative volume × percentage price change. PVT rising = buying pressure. PVT falling = selling pressure. Z-score confirms the PVT direction: when PVT is rising AND Z-score > 0 → confirmed bullish (price and volume aligned). When PVT is falling AND Z-score < 0 → confirmed bearish. When they disagree (PVT rising but Z-score < 0) → weaker signal, require stronger PVT momentum.

**Why not duplicate**: RUN223 uses Volume Price Trend. This RUN specifically adds Z-score as a confirmation filter — the combination of PVT direction and Z-score confirmation is what makes this distinct from plain PVT.

## Proposed Config Changes (config.rs)

```rust
// ── RUN351: Volume-Price Trend with Z-Score Confirmation ──────────────────────────────
// pvt = prior_pvt + volume * (close - prior_close) / prior_close
// z_score = (pvt - sma(pvt, period)) / stddev(pvt, period)
// LONG: pvt rising AND z_score > Z_THRESH_LONG
// SHORT: pvt falling AND z_score < Z_THRESH_SHORT
// Divergence: price rising but pvt falling = weakening (suppress LONG)

pub const PVT_Z_ENABLED: bool = true;
pub const PVT_Z_PERIOD: usize = 20;
pub const PVT_Z_Z_PERIOD: usize = 20;
pub const PVT_Z_Z_THRESH_LONG: f64 = 0.0;   // z-score > 0 confirms bullish
pub const PVT_Z_Z_THRESH_SHORT: f64 = 0.0;  // z-score < 0 confirms bearish
pub const PVT_Z_SL: f64 = 0.005;
pub const PVT_Z_TP: f64 = 0.004;
pub const PVT_Z_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run351_1_pvt_z_backtest.py)
2. **Walk-forward** (run351_2_pvt_z_wf.py)
3. **Combined** (run351_3_combined.py)

## Out-of-Sample Testing

- Z_PERIOD sweep: 14 / 20 / 30
- Z_THRESH sweep: -0.5/0 / 0/0 / 0.5/0
