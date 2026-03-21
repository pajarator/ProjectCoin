# RUN343 — VWAP Deviation Percentile with Trend Mode

## Hypothesis

**Mechanism**: Compute the percentage deviation of price from VWAP. Rank this deviation across its own historical distribution. When deviation percentile > 90% → price is historically extended above VWAP → expect mean-reversion back down. When deviation percentile < 10% → price is historically extended below VWAP → expect bounce up. Combine with trend mode (ADX) to filter: only take LONG deviations when ADX < 25 (not trending).

**Why not duplicate**: RUN243 uses VWAP standard deviation bands (raw %). RUN129 uses VWAP deviation percentile (no ADX filter). This RUN adds the ADX trend filter — VWAP deviation signals are suppressed when the market is strongly trending, only firing when the market is range-bound.

## Proposed Config Changes (config.rs)

```rust
// ── RUN343: VWAP Deviation Percentile with Trend Mode ──────────────────────────────
// vwap_dev_pct = (price - vwap) / vwap * 100
// vwap_dev_rank = percentile_rank(vwap_dev_pct, lookback)
// ADX < ADX_TREND_THRESH = range-bound (valid for mean-reversion)
// ADX >= ADX_TREND_THRESH = trending (suppress mean-reversion signals)
//
// LONG: vwap_dev_rank < 10 AND ADX < ADX_THRESH
// SHORT: vwap_dev_rank > 90 AND ADX < ADX_THRESH

pub const VWAP_DEV_TREND_ENABLED: bool = true;
pub const VWAP_DEV_TREND_LOOKBACK: usize = 100;
pub const VWAP_DEV_TREND_PCT_LOW: f64 = 10.0;   // bottom 10th percentile
pub const VWAP_DEV_TREND_PCT_HIGH: f64 = 90.0;  // top 10th percentile
pub const VWAP_DEV_TREND_ADX_THRESH: f64 = 25.0;
pub const VWAP_DEV_TREND_SL: f64 = 0.005;
pub const VWAP_DEV_TREND_TP: f64 = 0.004;
pub const VWAP_DEV_TREND_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run343_1_vwap_dev_trend_backtest.py)
2. **Walk-forward** (run343_2_vwap_dev_trend_wf.py)
3. **Combined** (run343_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 50 / 100 / 200
- PCT_LOW sweep: 5 / 10 / 15
- PCT_HIGH sweep: 85 / 90 / 95
- ADX_THRESH sweep: 20 / 25 / 30
