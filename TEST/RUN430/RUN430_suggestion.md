# RUN430 — Price Volume Correlation Divergence with SuperTrend

## Hypothesis

**Mechanism**: Price Volume Correlation measures how tightly price and volume move together. When they have high positive correlation, price moves have institutional backing. When they diverge, price moves lack conviction and often reverse. SuperTrend provides the actual entry direction. When Price Volume Correlation diverges (price and volume disagree) AND SuperTrend flips in the direction of the divergence, you have both a lack-of-institutional-backing signal AND trend confirmation working together.

**Why not duplicate**: RUN329 uses Price Volume Rank Correlation. RUN409 uses Volume-Price Correlation with Williams %R Extreme. This RUN specifically uses Price Volume Correlation Divergence with SuperTrend flip — the distinct mechanism is using SuperTrend flip direction to confirm the divergence signal, rather than Williams %R extremes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN430: Price Volume Correlation Divergence with SuperTrend ─────────────────────────────────
// price_vol_corr = correlation(price_changes, volume_changes, period)
// corr_divergence: corr drops below CORR_DIVERGENCE_THRESH (institutional backing weakening)
// supertrend: atr-based trend direction
// supertrend_flip: trend changes direction
// LONG: corr_divergence AND supertrend_flip bullish
// SHORT: corr_divergence AND supertrend_flip bearish

pub const PVC_ST_ENABLED: bool = true;
pub const PVC_ST_CORR_PERIOD: usize = 20;
pub const PVC_ST_CORR_DIV_THRESH: f64 = 0.3;  // below this = divergence
pub const PVC_ST_ST_PERIOD: usize = 10;
pub const PVC_ST_ST_MULT: f64 = 3.0;
pub const PVC_ST_SL: f64 = 0.005;
pub const PVC_ST_TP: f64 = 0.004;
pub const PVC_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run430_1_pvc_st_backtest.py)
2. **Walk-forward** (run430_2_pvc_st_wf.py)
3. **Combined** (run430_3_combined.py)

## Out-of-Sample Testing

- CORR_PERIOD sweep: 14 / 20 / 30
- CORR_DIV_THRESH sweep: 0.2 / 0.3 / 0.4
- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
