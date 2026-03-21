# RUN296 — EMA 50-200 Golden/Death Cross: Major Trend Reversal

## Hypothesis

**Mechanism**: The golden cross (EMA50 crosses above EMA200) = major bullish shift. The death cross (EMA50 crosses below EMA200) = major bearish shift. These are major market structure changes. Trade in the direction of the cross on the confirmation bar.

**Why not duplicate**: No prior RUN uses EMA 50-200 cross. All prior EMA cross RUNs use shorter periods. EMA 50-200 is distinct because it's a *major trend* signal, not a short-term trend signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN296: EMA 50-200 Golden/Death Cross ───────────────────────────────
// golden_cross = EMA50 crosses above EMA200
// death_cross = EMA50 crosses below EMA200
// LONG: golden_cross confirmed
// SHORT: death_cross confirmed

pub const GOLDEN_DEATH_ENABLED: bool = true;
pub const GOLDEN_DEATH_FAST: usize = 50;     // EMA 50
pub const GOLDEN_DEATH_SLOW: usize = 200;    // EMA 200
pub const GOLDEN_DEATH_SL: f64 = 0.005;
pub const GOLDEN_DEATH_TP: f64 = 0.004;
pub const GOLDEN_DEATH_MAX_HOLD: u32 = 96;   // ~24 hours at 15m
```

---

## Validation Method

1. **Historical backtest** (run296_1_golden_death_backtest.py)
2. **Walk-forward** (run296_2_golden_death_wf.py)
3. **Combined** (run296_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 30 / 50 / 75
- SLOW sweep: 150 / 200 / 300
