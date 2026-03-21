# RUN328 — ADX Momentum Angle: Rate of Trend Strength Change

## Hypothesis

**Mechanism**: ADX measures trend strength but not direction. +DI and -DI measure direction. ADX rising = trend strengthening. ADX falling = trend weakening. The "momentum angle" = rate of change of ADX over N bars. When ADX is rising AND +DI > -DI → strengthening bullish trend → LONG. When ADX is falling AND -DI > +DI → weakening bearish trend → SHORT. The angle of ADX rise/fall predicts trend continuation vs exhaustion.

**Why not duplicate**: RUN200 uses DMI/ADX for trend strength. RUN292 uses ADX disposition (how +DI and -DI relate). RUN266 uses ADX percentile rank. No RUN specifically uses the rate-of-change of ADX (ADX momentum angle) combined with directional DI alignment. The ADX momentum angle is the key differentiator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN328: ADX Momentum Angle ────────────────────────────────────────────────
// adx_momentum_angle = (adx - adx[n]) / n   (rate of ADX change)
// adx_rising = adx_momentum_angle > ANGLE_THRESH
// adx_falling = adx_momentum_angle < -ANGLE_THRESH
// LONG: adx_rising AND +DI > -DI (strengthening uptrend)
// SHORT: adx_rising AND -DI > +DI (strengthening downtrend)
// Exit: adx starts falling OR DI crosses against direction

pub const ADX_ANGLE_ENABLED: bool = true;
pub const ADX_ANGLE_PERIOD: usize = 14;
pub const ADX_ANGLE_MOMENTUM_BARS: u32 = 5;  // bars to measure ADX rate of change
pub const ADX_ANGLE_THRESH: f64 = 0.5;       // min ADX change per bar for signal
pub const ADX_ANGLE_SL: f64 = 0.005;
pub const ADX_ANGLE_TP: f64 = 0.004;
pub const ADX_ANGLE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run328_1_adx_angle_backtest.py)
2. **Walk-forward** (run328_2_adx_angle_wf.py)
3. **Combined** (run328_3_combined.py)

## Out-of-Sample Testing

- MOMENTUM_BARS sweep: 3 / 5 / 8
- THRESH sweep: 0.3 / 0.5 / 0.8
- PERIOD sweep: 10 / 14 / 21
