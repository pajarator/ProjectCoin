# RUN272 — EMA Distance Percentile: How Unusual is the Current Deviation?

## Hypothesis

**Mechanism**: Price's distance from EMA (as a percentage) varies historically. When the current distance is at an extreme percentile (e.g., >95%) → price is unusually far from its moving average → mean reversion likely. When at a low percentile (<5%) → price is unusually close to EMA → trending continuation possible.

**Why not duplicate**: RUN242 uses EMA distance but with fixed thresholds. EMA Distance Percentile uses *percentile rank* of the distance, making it adaptive to each coin's historical distribution.

## Proposed Config Changes (config.rs)

```rust
// ── RUN272: EMA Distance Percentile ──────────────────────────────────────
// ema_dist = (close - EMA(close, period)) / EMA(close, period) × 100
// dist_percentile = percentile rank of ema_dist in its history
// dist_percentile > 95 → extremely extended → mean-revert SHORT
// dist_percentile < 5 → extremely compressed → trend continuation LONG

pub const EMA_DIST_PCT_ENABLED: bool = true;
pub const EMA_DIST_PCT_EMA_PERIOD: usize = 20;
pub const EMA_DIST_PCT_WINDOW: usize = 100;
pub const EMA_DIST_PCT_EXTENDED: f64 = 95.0;  // extended threshold
pub const EMA_DIST_PCT_COMPRESSED: f64 = 5.0;  // compressed threshold
pub const EMA_DIST_PCT_SL: f64 = 0.005;
pub const EMA_DIST_PCT_TP: f64 = 0.004;
pub const EMA_DIST_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run272_1_ema_dist_pct_backtest.py)
2. **Walk-forward** (run272_2_ema_dist_pct_wf.py)
3. **Combined** (run272_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 50
- WINDOW sweep: 50 / 100 / 200
- EXTENDED sweep: 90 / 95 / 98
- COMPRESSED sweep: 2 / 5 / 10
