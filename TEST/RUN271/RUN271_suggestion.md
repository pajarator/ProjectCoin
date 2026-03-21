# RUN271 — ATR Volatility Surface: Multi-Period ATR Consensus

## Hypothesis

**Mechanism**: ATR at different periods (7, 14, 28) tells you different things about volatility. ATR(7) > ATR(14) = short-term volatility spike. ATR(14) > ATR(28) = medium-term. When all three agree (all rising or all falling) → strong signal. When they disagree → transition. Trade when short-term and medium-term ATR agree on direction.

**Why not duplicate**: No prior RUN uses multi-period ATR. All prior ATR RUNs use single-period ATR for stops. Multi-period ATR surface is distinct because it compares *volatility across timeframes*.

## Proposed Config Changes (config.rs)

```rust
// ── RUN271: ATR Volatility Surface ───────────────────────────────────────
// atr_fast = ATR(closes, 7)
// atr_med = ATR(closes, 14)
// atr_slow = ATR(closes, 28)
// atr_rising = atr_fast > atr_fast[prior] AND atr_med > atr_med[prior]
// atr_falling = atr_fast < atr_fast[prior] AND atr_med < atr_med[prior]
// Rising volatility → wider stops needed
// Falling volatility → tighter stops possible

pub const ATR_SURFACE_ENABLED: bool = true;
pub const ATR_SURFACE_FAST: usize = 7;
pub const ATR_SURFACE_MED: usize = 14;
pub const ATR_SURFACE_SLOW: usize = 28;
pub const ATR_SURFACE_SL_ATR: f64 = 2.0;   // SL = ATR × this
pub const ATR_SURFACE_TP_ATR: f64 = 2.0;   // TP = ATR × this
```

Modify engine's SL/TP calculation to use the multi-period ATR surface for adaptive stop/target sizing.

---

## Validation Method

1. **Historical backtest** (run271_1_atr_surface_backtest.py)
2. **Walk-forward** (run271_2_atr_surface_wf.py)
3. **Combined** (run271_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 5 / 7 / 10
- MED sweep: 10 / 14 / 20
- SLOW sweep: 20 / 28 / 40
