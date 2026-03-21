# RUN433 — Ichimoku Cloud with Volume Percentile Rank Confirmation

## Hypothesis

**Mechanism**: The Ichimoku Cloud provides comprehensive trend direction and support/resistance via the Kumo (cloud). The cloud boundaries act as dynamic support/resistance levels. Volume Percentile Rank measures where current volume sits relative to its historical range — high percentile means unusual volume. When price approaches the cloud boundary AND Volume Percentile Rank is high, the cloud boundary touch has institutional backing and is more likely to hold.

**Why not duplicate**: RUN299 uses Ichimoku Cloud Twist (TK cross + Kumo). RUN369 uses Ichimoku Cloud with Volume Confirmation. RUN393 uses Ichimoku Cloud with RSI Extreme Zone. This RUN specifically uses Volume Percentile Rank (distinct from volume confirmation or RSI extremes) to confirm Ichimoku cloud boundary touches — the distinct mechanism is using volume percentile ranking to confirm the strength of cloud boundary interactions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN433: Ichimoku Cloud with Volume Percentile Rank Confirmation ─────────────────────────────
// tenkan_sen = (highest_high + lowest_low) / 2 over lookback
// kijun_sen = (highest_high + lowest_low) / 2 over standard period
// senkou_span_a = (tenkan + kijun) / 2, plotted ahead
// senkou_span_b = (highest_high + lowest_low) / 2 over period, plotted ahead
// kumo_cloud = area between senkou_span_a and senkou_span_b
// cloud_boundary_touch: price approaches cloud boundary
// volume_percentile = percentile rank of volume within lookback window
// vol_confirm: volume_percentile > VOL_PCT_THRESH
// LONG: price approaches cloud from below AND vol_percentile > threshold
// SHORT: price approaches cloud from above AND vol_percentile > threshold

pub const ICHIMOKU_VOL_PCT_ENABLED: bool = true;
pub const ICHIMOKU_VOL_PCT_TENKAN_PERIOD: usize = 9;
pub const ICHIMOKU_VOL_PCT_KIJUN_PERIOD: usize = 26;
pub const ICHIMOKU_VOL_PCT_SENKOU_PERIOD: usize = 52;
pub const ICHIMOKU_VOL_PCT_VOL_PERIOD: usize = 20;
pub const ICHIMOKU_VOL_PCT_VOL_PCT_THRESH: f64 = 75.0;  // top 25% volume
pub const ICHIMOKU_VOL_PCT_SL: f64 = 0.005;
pub const ICHIMOKU_VOL_PCT_TP: f64 = 0.004;
pub const ICHIMOKU_VOL_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run433_1_ichimoku_vol_pct_backtest.py)
2. **Walk-forward** (run433_2_ichimoku_vol_pct_wf.py)
3. **Combined** (run433_3_combined.py)

## Out-of-Sample Testing

- TENKAN_PERIOD sweep: 7 / 9 / 12
- KIJUN_PERIOD sweep: 22 / 26 / 30
- SENKOU_PERIOD sweep: 45 / 52 / 60
- VOL_PCT_THRESH sweep: 70 / 75 / 80
