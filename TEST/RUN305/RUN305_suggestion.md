# RUN305 — Pivot Point Mean Reversion: Bouncing Off Key Daily Levels

## Hypothesis

**Mechanism**: Daily pivot points (P, S1, S2, R1, R2, R3) act as gravitational support and resistance. When price approaches a support level from above and bounces → LONG. When price approaches resistance from below and rejects → SHORT. Combine with RSI at the level: if price bounces at S1 but RSI is still below 40 → stronger signal because there's room to run back to the pivot.

**Why not duplicate**: No prior RUN uses pivot points. Fibonacci retracements (RUN244) use similar logic but with different level calculations. Pivot points are mathematically distinct (P = (H+L+O)/3 vs Fibonacci ratios) and more adaptive to daily range. Pivot-based mean reversion is a time-tested approach distinct from percentage-based support/resistance.

## Proposed Config Changes (config.rs)

```rust
// ── RUN305: Pivot Point Mean Reversion ──────────────────────────────────────
// pivot = (high_prev + low_prev + close_prev) / 3
// s1 = pivot - (high_prev - low_prev) * 0.382
// r1 = pivot + (high_prev - low_prev) * 0.382
// s2 = pivot - (high_prev - low_prev) * 0.618
// r2 = pivot + (high_prev - low_prev) * 0.618
// LONG: price crosses above S1 from below AND RSI < 60 (room to run)
// SHORT: price crosses below R1 from above AND RSI > 40 (room to drop)
// Stop: below S2 for longs, above R2 for shorts

pub const PIVOT_ENABLED: bool = true;
pub const PIVOTLookback: usize = 1;           // use prior N daily candles
pub const PIVOT_S_LEVEL: usize = 1;           // S1 or S2 for LONG bounce
pub const PIVOT_R_LEVEL: usize = 1;           // R1 or R2 for SHORT bounce
pub const PIVOT_RSI_LONG: f64 = 60.0;         // max RSI for valid LONG bounce
pub const PIVOT_RSI_SHORT: f64 = 40.0;        // min RSI for valid SHORT bounce
pub const PIVOT_SL: f64 = 0.005;
pub const PIVOT_TP: f64 = 0.004;
pub const PIVOT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run305_1_pivot_backtest.py)
2. **Walk-forward** (run305_2_pivot_wf.py)
3. **Combined** (run305_3_combined.py)

## Out-of-Sample Testing

- S_LEVEL sweep: 1 / 2
- R_LEVEL sweep: 1 / 2
- RSI_LONG sweep: 50 / 60 / 70
- RSI_SHORT sweep: 30 / 40 / 50
