# RUN326 — Z-Score Distance from VWAP: VWAP-Centered Deviation Reversion

## Hypothesis

**Mechanism**: Instead of Z-score using SMA as the mean, compute Z-score using VWAP as the mean. VWAP is volume-weighted, so it reflects where the market has actually traded, not just the simple average. When price deviates significantly from VWAP (Z-score > 2), it's extended from fair value → mean-reversion back to VWAP. When price is below VWAP and Z-score < -2, it's undervalued → bounce back to VWAP.

**Why not duplicate**: RUN243 uses VWAP standard deviation bands (distance from VWAP in %, not Z-score). RUN129 uses VWAP deviation percentile. This RUN specifically computes Z-score using VWAP as the baseline mean — the statistical framing is distinct. VWAP as mean is more responsive to volume-weighted fair value than SMA.

## Proposed Config Changes (config.rs)

```rust
// ── RUN326: Z-Score Distance from VWAP ────────────────────────────────────────
// vwap_z = (price - vwap) / stddev(price - vwap, lookback)
// LONG: vwap_z < -Z_THRESH (undervalued, extended below VWAP)
// SHORT: vwap_z > Z_THRESH (overvalued, extended above VWAP)
// Exit: vwap_z crosses back through 0 (returned to fair value)

pub const VWAP_Z_ENABLED: bool = true;
pub const VWAP_Z_LOOKBACK: usize = 20;
pub const VWAP_Z_THRESH_LONG: f64 = -2.0;   // deviation threshold for LONG
pub const VWAP_Z_THRESH_SHORT: f64 = 2.0;    // deviation threshold for SHORT
pub const VWAP_Z_SL: f64 = 0.005;
pub const VWAP_Z_TP: f64 = 0.004;
pub const VWAP_Z_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run326_1_vwap_z_backtest.py)
2. **Walk-forward** (run326_2_vwap_z_wf.py)
3. **Combined** (run326_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 14 / 20 / 30
- THRESH_LONG sweep: -1.5 / -2.0 / -2.5
- THRESH_SHORT sweep: 1.5 / 2.0 / 2.5
