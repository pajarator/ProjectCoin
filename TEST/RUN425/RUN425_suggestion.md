# RUN425 — Keltner Channel with CMO Momentum Confirmation

## Hypothesis

**Mechanism**: Keltner Channel uses ATR-multiplied bands around an EMA to define dynamic support/resistance levels. The Chande Momentum Oscillator (CMO) is a normalized momentum indicator that's more responsive than RSI because it doesn't smooth internal data. When Keltner bands are touched AND CMO confirms momentum in the same direction, the signal has both channel structure AND raw momentum confirmation.

**Why not duplicate**: RUN318 uses Keltner Channel with Volume Confirmation. RUN381 uses Keltner Channel with Stochastic Filter. This RUN specifically uses CMO (distinct momentum calculation) as the confirmation filter — CMO's unsmoothed nature makes it more responsive than Stochastic, providing earlier momentum confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN425: Keltner Channel with CMO Momentum Confirmation ─────────────────────────────
// keltner_upper = EMA(close, period) + MULT * ATR(period)
// keltner_lower = EMA(close, period) - MULT * ATR(period)
// band_touch: price crosses keltner_upper or keltner_lower
// cmo = (sum_gains - sum_losses) / (sum_gains + sum_losses) * 100
// cmo_confirm: cmo > CMO_THRESH for longs, cmo < -CMO_THRESH for shorts
// LONG: price touches keltner_lower AND cmo > CMO_THRESH
// SHORT: price touches keltner_upper AND cmo < -CMO_THRESH

pub const KC_CMO_ENABLED: bool = true;
pub const KC_CMO_KC_PERIOD: usize = 20;
pub const KC_CMO_ATR_PERIOD: usize = 14;
pub const KC_CMO_ATR_MULT: f64 = 2.0;
pub const KC_CMO_CMO_PERIOD: usize = 14;
pub const KC_CMO_CMO_THRESH: f64 = 20.0;   // cmo must exceed this to confirm
pub const KC_CMO_SL: f64 = 0.005;
pub const KC_CMO_TP: f64 = 0.004;
pub const KC_CMO_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run425_1_kc_cmo_backtest.py)
2. **Walk-forward** (run425_2_kc_cmo_wf.py)
3. **Combined** (run425_3_combined.py)

## Out-of-Sample Testing

- KC_PERIOD sweep: 15 / 20 / 30
- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- CMO_PERIOD sweep: 10 / 14 / 21
- CMO_THRESH sweep: 15 / 20 / 25
