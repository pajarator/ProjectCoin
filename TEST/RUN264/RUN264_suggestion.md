# RUN264 — Volume Profile POC Rejection: Point of Control as Sticky Level

## Hypothesis

**Mechanism**: The Point of Control (POC — highest volume price in the profile) is a "sticky" level where price tends to revisit. When price approaches POC from below and gets rejected → bearish. When price approaches POC from above and gets rejected → bullish. The rejection at POC confirms institutional activity at that level.

**Why not duplicate**: No prior RUN uses POC rejection. All prior POC RUNs (RUN186) use HVN/LVN as zones. POC rejection is distinct because it specifically identifies the *highest volume price* and uses the approach-and-rejection pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN264: Volume Profile POC Rejection ─────────────────────────────────
// POC = price level with highest volume in the profile
// Rejection at POC: price approaches within 0.2% of POC AND reverses
// LONG: price approaches POC from below AND reverses
// SHORT: price approaches POC from above AND reverses

pub const POC_REJ_ENABLED: bool = true;
pub const POC_REJ_WINDOW: usize = 50;         // profile lookback
pub const POC_REJ_BUCKETS: usize = 50;        // price buckets
pub const POC_REJ_DIST: f64 = 0.002;         // within 0.2% of POC
pub const POC_REJ_SL: f64 = 0.005;
pub const POC_REJ_TP: f64 = 0.004;
pub const POC_REJ_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run264_1_poc_rej_backtest.py)
2. **Walk-forward** (run264_2_poc_rej_wf.py)
3. **Combined** (run264_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 30 / 50 / 100
- BUCKETS sweep: 30 / 50 / 100
- DIST sweep: 0.001 / 0.002 / 0.003
