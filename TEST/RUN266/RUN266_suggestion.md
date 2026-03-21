# RUN266 — ADX Trend Strength Percentile: Historical Position of Trend Intensity

## Hypothesis

**Mechanism**: ADX at 40 vs ADX at 20 doesn't tell you if 40 is historically high for this coin. ADX percentile rank = where does current ADX fall in its N-bar history? ADX at 95th percentile = extremely strong trend. ADX at 5th percentile = extremely weak/no trend. Trade ADX extremes with directional bias.

**Why not duplicate**: No prior RUN uses ADX percentile rank. All prior ADX RUNs use fixed thresholds. ADX percentile rank is unique because it tells you whether the *current trend strength* is extreme for that specific coin's historical distribution.

## Proposed Config Changes (config.rs)

```rust
// ── RUN266: ADX Trend Strength Percentile ────────────────────────────────
// adx_percentile = percentile rank of current ADX within ADX history
// adx_percentile > 90 → extremely strong trend → ride the trend
// adx_percentile < 10 → extremely weak trend → mean-reversion

pub const ADX_PCT_ENABLED: bool = true;
pub const ADX_PCT_PERIOD: usize = 14;        // ADX period
pub const ADX_PCT_WINDOW: usize = 100;       // history window
pub const ADX_PCT_STRONG: f64 = 90.0;       // strong trend threshold
pub const ADX_PCT_WEAK: f64 = 10.0;         // weak trend threshold
```

Modify engine: when ADX_PCT > ADX_PCT_STRONG → use momentum strategies; when ADX_PCT < ADX_PCT_WEAK → use mean-reversion strategies.

---

## Validation Method

1. **Historical backtest** (run266_1_adx_pct_backtest.py)
2. **Walk-forward** (run266_2_adx_pct_wf.py)
3. **Combined** (run266_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- WINDOW sweep: 50 / 100 / 200
- STRONG sweep: 85 / 90 / 95
- WEAK sweep: 5 / 10 / 15
