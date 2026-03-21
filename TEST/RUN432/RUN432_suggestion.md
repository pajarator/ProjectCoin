# RUN432 — EMA Distance Percentile with Volume Surge

## Hypothesis

**Mechanism**: EMA Distance Percentile measures how far the current price has deviated from its EMA relative to historical deviations — tracking whether the price is unusually far from its EMA. When price gets unusually far from the EMA (high percentile), it tends to mean-revert. Volume Surge confirms institutional participation in the move away from EMA. When EMA Distance Percentile is extreme AND Volume Surge confirms the move, the mean reversion probability is elevated with institutional backing.

**Why not duplicate**: RUN367 uses EMA Distance Percentile with Volume Surge. Wait — that's a duplicate. Let me reconsider. EMA Distance Percentile with Momentum Rotation? No, similar to RUN297. Let me do EMA Distance Percentile with Trend Mode Filter instead — different mechanism.

Actually, let me do: EMA Distance Percentile with ADX Trend Strength Filter. When price is far from EMA (high percentile) AND ADX shows a strong trend (price truly diverging from mean in a trending way), the signal distinguishes between mean-reversion opportunities and genuine trend continuation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN432: EMA Distance Percentile with ADX Trend Strength Filter ─────────────────────────────
// ema_distance_pct = |close - EMA(close, period)| / EMA(close, period)
// ema_distance_percentile = percentile rank of ema_distance over lookback
// extreme_distance: ema_distance_percentile > PERCENTILE_THRESH
// adx = ADX(close, period) measuring trend strength
// strong_trend: adx > ADX_THRESH
// valid_setup: extreme_distance AND strong_trend means it's a trending divergence (don't fade)
// fade_extreme: when extreme_distance but adx < ADX_THRESH (range-bound deviation)
// LONG: fade_extreme (price far from EMA in range-bound market = mean reversion)
// SHORT: fade_extreme (price far from EMA in range-bound market = mean reversion)

pub const EMA_PCT_ADX_ENABLED: bool = true;
pub const EMA_PCT_ADX_EMA_PERIOD: usize = 20;
pub const EMA_PCT_ADX_PCT_PERIOD: usize = 20;
pub const EMA_PCT_ADX_PERCENTILE_THRESH: f64 = 85.0;  // top 15% of deviations
pub const EMA_PCT_ADX_ADX_PERIOD: usize = 14;
pub const EMA_PCT_ADX_ADX_THRESH: f64 = 25.0;   // below = range-bound
pub const EMA_PCT_ADX_SL: f64 = 0.005;
pub const EMA_PCT_ADX_TP: f64 = 0.004;
pub const EMA_PCT_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run432_1_ema_pct_adx_backtest.py)
2. **Walk-forward** (run432_2_ema_pct_adx_wf.py)
3. **Combined** (run432_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 15 / 20 / 30
- PCT_PERIOD sweep: 14 / 20 / 30
- PERCENTILE_THRESH sweep: 80 / 85 / 90
- ADX_THRESH sweep: 20 / 25 / 30
