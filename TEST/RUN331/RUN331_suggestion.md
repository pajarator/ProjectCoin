# RUN331 — Momentum Acceleration Divergence: Second-Derivative Reversal

## Hypothesis

**Mechanism**: Measure the rate of change of momentum itself — momentum acceleration. Momentum = ROC(N). Acceleration = ROC(M) of momentum. When acceleration crosses above 0 → momentum was decelerating but now speeding up → LONG. When acceleration crosses below 0 → momentum was accelerating but now slowing → SHORT. Acceleration divergence from price = the most advanced warning of reversal.

**Why not duplicate**: No prior RUN measures momentum's momentum (second derivative of price). ROC RUNs (RUN196, RUN13) measure first derivative. Acceleration is strictly a second-derivative signal — it predicts the turning point before momentum itself turns.

## Proposed Config Changes (config.rs)

```rust
// ── RUN331: Momentum Acceleration Divergence ───────────────────────────────────
// momentum = ROC(close, MOM_PERIOD)
// acceleration = ROC(momentum, ACC_PERIOD)  (momentum of momentum)
// acc_positive = acceleration > 0
// acc_negative = acceleration < 0
// LONG: acc_positive AND momentum > 0 (rising market with accelerating momentum)
// SHORT: acc_negative AND momentum < 0 (falling market with decelerating momentum)
// Exit: acceleration flips sign

pub const MOM_ACCEL_ENABLED: bool = true;
pub const MOM_ACCEL_MOM_PERIOD: usize = 10;   // momentum lookback
pub const MOM_ACCEL_ACC_PERIOD: usize = 5;    // acceleration lookback
pub const MOM_ACCEL_SL: f64 = 0.005;
pub const MOM_ACCEL_TP: f64 = 0.004;
pub const MOM_ACCEL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run331_1_mom_accel_backtest.py)
2. **Walk-forward** (run331_2_mom_accel_wf.py)
3. **Combined** (run331_3_combined.py)

## Out-of-Sample Testing

- MOM_PERIOD sweep: 5 / 10 / 20
- ACC_PERIOD sweep: 3 / 5 / 8
