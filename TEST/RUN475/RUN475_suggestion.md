# RUN475 — ADX Momentum with Volume Percentile Rank

## Hypothesis

**Mechanism**: ADX (Average Directional Index) measures trend strength without direction - high ADX indicates strong trend, low ADX indicates ranging market. Volume Percentile Rank shows how current volume compares to recent history. When ADX is rising (strengthening trend) AND Volume Percentile Rank is high, the trend has both directional strength and volume-backed conviction.

**Why not duplicate**: RUN396 uses Fisher Transform with ADX Trend Strength Filter. This RUN uses ADX with Volume Percentile Rank instead — distinct mechanism is volume percentile rank as a relative volume measure versus Fisher Transform's Gaussian normalization.

## Proposed Config Changes (config.rs)

```rust
// ── RUN475: ADX Momentum with Volume Percentile Rank ─────────────────────────────────
// adx: average_directional_index measuring trend_strength
// adx_rising: adx increasing indicating strengthening trend
// vol_percentile: current volume rank vs historical volume distribution
// LONG: adx_rising AND adx > 20 AND vol_percentile > 70
// SHORT: adx_rising AND adx > 20 AND vol_percentile > 70

pub const ADX_VOLPR_ENABLED: bool = true;
pub const ADX_VOLPR_ADX_PERIOD: usize = 14;
pub const ADX_VOLPR_ADX_THRESH: f64 = 20.0;
pub const ADX_VOLPR_ADX_CHANGE_THRESH: f64 = 2.0;
pub const ADX_VOLPR_VOL_PERIOD: usize = 20;
pub const ADX_VOLPR_VOL_PCT_THRESH: f64 = 70.0;
pub const ADX_VOLPR_SL: f64 = 0.005;
pub const ADX_VOLPR_TP: f64 = 0.004;
pub const ADX_VOLPR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run475_1_adx_volpr_backtest.py)
2. **Walk-forward** (run475_2_adx_volpr_wf.py)
3. **Combined** (run475_3_combined.py)

## Out-of-Sample Testing

- ADX_PERIOD sweep: 10 / 14 / 20
- ADX_THRESH sweep: 15 / 20 / 25
- ADX_CHANGE_THRESH sweep: 1.5 / 2.0 / 2.5
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_PCT_THRESH sweep: 60 / 70 / 80
