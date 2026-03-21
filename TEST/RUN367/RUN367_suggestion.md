# RUN367 — EMA Distance Percentile with Volume Surge

## Hypothesis

**Mechanism**: The distance of price from its EMA (as a percentage) tells you how extended the market is. When price is far above EMA, it's extended — likely to mean-revert down. When far below EMA, extended — likely to bounce. Compute percentile rank of EMA distance to know when it's at historical extremes. Volume surge confirms the reversal: when extended AND volume surges in the direction of reversal → high probability mean-reversion.

**Why not duplicate**: RUN272 uses EMA distance percentile but without volume confirmation. This RUN specifically adds volume surge as the confirmation signal — the distinct mechanism is volume-surge-confirmed EMA distance mean reversion.

## Proposed Config Changes (config.rs)

```rust
// ── RUN367: EMA Distance Percentile with Volume Surge ────────────────────────────────
// ema_distance_pct = (close - EMA(close, period)) / EMA(close, period) * 100
// ema_dist_rank = percentile_rank(ema_distance_pct, lookback)
// volume_surge = volume > avg_vol * VOL_MULT
// LONG: ema_dist_rank < DIST_LOW AND volume_surge AND price recovering
// SHORT: ema_dist_rank > DIST_HIGH AND volume_surge AND price declining

pub const EMA_DIST_VOL_ENABLED: bool = true;
pub const EMA_DIST_VOL_EMA_PERIOD: usize = 20;
pub const EMA_DIST_VOL_LOOKBACK: usize = 100;
pub const EMA_DIST_VOL_DIST_LOW: f64 = 10.0;    // bottom 10th percentile
pub const EMA_DIST_VOL_DIST_HIGH: f64 = 90.0;   // top 10th percentile
pub const EMA_DIST_VOL_VOL_MULT: f64 = 2.0;
pub const EMA_DIST_VOL_SL: f64 = 0.005;
pub const EMA_DIST_VOL_TP: f64 = 0.004;
pub const EMA_DIST_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run367_1_ema_dist_vol_backtest.py)
2. **Walk-forward** (run367_2_ema_dist_vol_wf.py)
3. **Combined** (run367_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 15 / 20 / 30
- LOOKBACK sweep: 50 / 100 / 200
- DIST_LOW sweep: 5 / 10 / 15
- DIST_HIGH sweep: 85 / 90 / 95
- VOL_MULT sweep: 1.5 / 2.0 / 2.5
