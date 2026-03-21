# RUN278 — VWAP Session Distance: Fair Value Deviation Entry

## Hypothesis

**Mechanism**: VWAP represents the fair value for the session. Price's deviation from VWAP (as a percentage) identifies when the market is overpriced or underpriced. When price is >1% above VWAP → shorting opportunity (expect reversion to VWAP). When price is >1% below VWAP → buying opportunity.

**Why not duplicate**: RUN208 uses VWAP deviation bands. RUN243 uses VWAP stddev bands. This RUN uses simple VWAP distance with fixed percentage thresholds — a simpler, more direct approach.

## Proposed Config Changes (config.rs)

```rust
// ── RUN278: VWAP Session Distance ────────────────────────────────────────
// vwap_distance = (close - vwap) / vwap × 100
// vwap_distance > 1.0 → overpriced → SHORT
// vwap_distance < -1.0 → underpriced → LONG

pub const VWAP_DIST_ENABLED: bool = true;
pub const VWAP_DIST_LONG_THRESH: f64 = -1.0;  // % below VWAP
pub const VWAP_DIST_SHORT_THRESH: f64 = 1.0; // % above VWAP
pub const VWAP_DIST_SL: f64 = 0.005;
pub const VWAP_DIST_TP: f64 = 0.004;
pub const VWAP_DIST_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run278_1_vwap_dist_backtest.py)
2. **Walk-forward** (run278_2_vwap_dist_wf.py)
3. **Combined** (run278_3_combined.py)

## Out-of-Sample Testing

- LONG_THRESH sweep: -0.5 / -1.0 / -1.5
- SHORT_THRESH sweep: 0.5 / 1.0 / 1.5
