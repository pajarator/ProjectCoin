# RUN391 — Elder Ray Index with Chande Momentum Oscillator Confirmation

## Hypothesis

**Mechanism**: Elder Ray Index measures the "bull power" and "bear power" of each candle relative to an EMA filter. Bull power measures how far the high exceeds the EMA; bear power measures how far the low is below the EMA. The Chande Momentum Oscillator (CMO) is a normalized momentum indicator that doesn't smooth like RSI. Combine them: when Elder Ray shows strong bull/bear power divergence AND CMO crosses in the same direction, the signal has both price-structure power measurement AND raw momentum confirmation.

**Why not duplicate**: RUN319 uses Elder Ray Bull/Bear Power with EMA. RUN307 uses Chande Momentum Oscillator standalone. This RUN specifically combines Elder Ray power measurement with CMO as a momentum confirmation filter — the distinct mechanism is using CMO (non-smoothed normalized momentum) to confirm Elder Ray's price structure signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN391: Elder Ray Index with Chande Momentum Oscillator Confirmation ─────────────
// elder_ray_bull = high - EMA(close, PERIOD)
// elder_ray_bear = low - EMA(close, PERIOD)
// elder_ray_flip: bull power crosses above/below bear power
// cmo = (sum_gains - sum_losses) / (sum_gains + sum_losses) * 100
// cmo_cross: cmo crosses above/below 0
// LONG: elder_ray_flip bullish AND cmo > CMO_CONFIRM (positive momentum confirmed)
// SHORT: elder_ray_flip bearish AND cmo < -CMO_CONFIRM (negative momentum confirmed)

pub const ELDER_CMO_ENABLED: bool = true;
pub const ELDER_CMO_ELDER_PERIOD: usize = 13;   // EMA period for Elder Ray
pub const ELDER_CMO_CMO_PERIOD: usize = 14;
pub const ELDER_CMO_CMO_CONFIRM: f64 = 10.0;   // cmo must be above/below this
pub const ELDER_CMO_SL: f64 = 0.005;
pub const ELDER_CMO_TP: f64 = 0.004;
pub const ELDER_CMO_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run391_1_elder_cmo_backtest.py)
2. **Walk-forward** (run391_2_elder_cmo_wf.py)
3. **Combined** (run391_3_combined.py)

## Out-of-Sample Testing

- ELDER_PERIOD sweep: 10 / 13 / 20
- CMO_PERIOD sweep: 10 / 14 / 21
- CMO_CONFIRM sweep: 5 / 10 / 15
