# RUN360 — ATR Percentile Rank with Trend Mode

## Hypothesis

**Mechanism**: ATR percentile rank tells you whether current volatility is historically high or low. When ATR percentile is high (>80%) → volatility is elevated → mean-reversion works well (markets tend to revert from extremes). When ATR percentile is low (<20%) → volatility is compressed → breakout strategies work better. Use ADX to determine trend vs range within each volatility regime.

**Why not duplicate**: RUN250 uses ATR percentile rank but without trend mode filtering. This RUN specifically adds ADX to determine whether to use mean-reversion or breakout within each volatility regime. The distinct mechanism is the 2-dimensional market classification: volatility regime (ATR percentile) × trend regime (ADX).

## Proposed Config Changes (config.rs)

```rust
// ── RUN360: ATR Percentile Rank with Trend Mode ───────────────────────────────
// atr_rank = percentile_rank(ATR(period), lookback)
// High vol (atr_rank > 80): use mean-reversion signals
// Low vol (atr_rank < 20): use breakout signals
// ADX < 20: range-bound (confirm with ADX)
// ADX >= 20: trending
//
// Range + High vol: RSI extremes
// Range + Low vol: Bollinger Band bounce
// Trending + High vol: ADX filter (suppress)
// Trending + Low vol: breakout continuation

pub const ATR_PCT_TREND_ENABLED: bool = true;
pub const ATR_PCT_PERIOD: usize = 14;
pub const ATR_PCT_LOOKBACK: usize = 100;
pub const ATR_PCT_VOL_HIGH: f64 = 80.0;   // above this = high vol
pub const ATR_PCT_VOL_LOW: f64 = 20.0;    // below this = low vol
pub const ATR_PCT_ADX_THRESH: f64 = 20.0;
pub const ATR_PCT_SL: f64 = 0.005;
pub const ATR_PCT_TP: f64 = 0.004;
pub const ATR_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run360_1_atr_pct_trend_backtest.py)
2. **Walk-forward** (run360_2_atr_pct_trend_wf.py)
3. **Combined** (run360_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 50 / 100 / 200
- VOL_HIGH sweep: 70 / 80 / 90
- VOL_LOW sweep: 10 / 20 / 30
- ADX_THRESH sweep: 15 / 20 / 25
