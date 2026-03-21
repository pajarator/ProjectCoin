# RUN465 — Hull Suite with CMO Momentum Confirmation

## Hypothesis

**Mechanism**: Hull Suite uses weighted moving averages with a distinctive algorithm to produce smooth, low-lag trend signals. CMO (Chande Momentum Oscillator) measures momentum without smoothing, making it more responsive to recent price changes. When Hull Suite fires a signal AND CMO confirms momentum is also moving in the same direction, the trade has both smooth trend direction and raw momentum conviction.

**Why not duplicate**: RUN428 uses Hull Suite Adaptive Trend with RSI Extreme Filter. This RUN uses CMO instead — the distinct mechanism is CMO's raw momentum confirmation versus RSI's oscillator extremes. CMO doesn't have the smoothing artifacts of RSI and directly measures momentum strength.

## Proposed Config Changes (config.rs)

```rust
// ── RUN465: Hull Suite with CMO Momentum Confirmation ─────────────────────────────────
// hull_suite: smooth low-lag MA using weighted hull algorithm
// hull_cross: hull MA crosses above/below price or signal line
// cmo: chande_momentum_oscillator measuring raw momentum
// cmo_cross: cmo crosses above/below signal line (typically 0 or 50)
// LONG: hull_cross bullish AND cmo > cmo_signal AND cmo positive
// SHORT: hull_cross bearish AND cmo < cmo_signal AND cmo negative

pub const HULL_CMO_ENABLED: bool = true;
pub const HULL_CMO_HULL_PERIOD: usize = 20;
pub const HULL_CMO_CMO_PERIOD: usize = 14;
pub const HULL_CMO_CMO_SIGNAL: usize = 9;
pub const HULL_CMO_SL: f64 = 0.005;
pub const HULL_CMO_TP: f64 = 0.004;
pub const HULL_CMO_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run465_1_hull_cmo_backtest.py)
2. **Walk-forward** (run465_2_hull_cmo_wf.py)
3. **Combined** (run465_3_combined.py)

## Out-of-Sample Testing

- HULL_PERIOD sweep: 15 / 20 / 25 / 30
- CMO_PERIOD sweep: 10 / 14 / 20
- CMO_SIGNAL sweep: 7 / 9 / 12
