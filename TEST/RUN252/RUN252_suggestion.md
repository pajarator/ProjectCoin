# RUN252 — Market Profile / TPO: Value Area Detection for Mean Reversion

## Hypothesis

**Mechanism**: Market Profile divides the price range into "value areas" where 70% of volume occurred. The Point of Control (POC) is the price with most volume. Prices above value area high → overpriced → SHORT. Prices below value area low → underpriced → LONG. The market tends to return to the POC and value area.

**Why not duplicate**: No prior RUN uses Market Profile or TPO. All prior support/resistance RUNs use static price levels (pivots, VWAP). Market Profile is unique because it defines support/resistance dynamically based on *volume distribution*.

## Proposed Config Changes (config.rs)

```rust
// ── RUN252: Market Profile / TPO ───────────────────────────────────────
// Build profile over N bars: bucket prices into fixed price increments
// Count volume in each bucket → POC = highest volume bucket
// Value Area High (VAH) = price level containing 70% of volume above POC
// Value Area Low (VAL) = price level containing 70% of volume below POC
// LONG: price < VAL (undervalued) → revert to value area
// SHORT: price > VAH (overvalued) → revert to value area

pub const MKT_PROFILE_ENABLED: bool = true;
pub const MKT_PROFILE_BARS: usize = 50;      // bars to build profile
pub const MKT_PROFILE_BUCKETS: usize = 50;    // price buckets
pub const MKT_PROFILE_VALUE_PCT: f64 = 0.70; // 70% of volume = value area
pub const MKT_PROFILE_SL: f64 = 0.005;
pub const MKT_PROFILE_TP: f64 = 0.004;
pub const MKT_PROFILE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run252_1_mkt_profile_backtest.py)
2. **Walk-forward** (run252_2_mkt_profile_wf.py)
3. **Combined** (run252_3_combined.py)

## Out-of-Sample Testing

- BARS sweep: 30 / 50 / 100
- BUCKETS sweep: 30 / 50 / 100
- VALUE_PCT sweep: 0.65 / 0.70 / 0.75
