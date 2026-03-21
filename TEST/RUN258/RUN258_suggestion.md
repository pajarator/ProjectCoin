# RUN258 — EMA Angle Trend Strength: Rate of Change of Moving Average

## Hypothesis

**Mechanism**: The angle (slope) of an EMA measures trend strength. A steep upward angle = strong bullish trend. A flat or declining angle = weakening trend. Compute the angle as the difference between current EMA and EMA N bars ago. When the angle crosses above a threshold → trend accelerating → entry. When angle crosses below threshold → trend decelerating → exit.

**Why not duplicate**: No prior RUN uses EMA angle. All prior trend RUNs use EMA crossover or MACD. EMA angle is unique because it measures *how fast* the EMA is changing, not just whether price is above or below it.

## Proposed Config Changes (config.rs)

```rust
// ── RUN258: EMA Angle Trend Strength ─────────────────────────────────────
// ema_angle = (EMA(close, period) - EMA(close[N], period)) / N
// angle expressed as % per bar
// angle > 0.001 (0.1% per bar) = bullish acceleration
// angle < -0.001 = bearish acceleration
// LONG: angle crosses above 0.001
// SHORT: angle crosses below -0.001

pub const EMA_ANGLE_ENABLED: bool = true;
pub const EMA_ANGLE_PERIOD: usize = 20;       // EMA period
pub const EMA_ANGLE_LOOKBACK: usize = 5;    // bars to measure angle over
pub const EMA_ANGLE_THRESH: f64 = 0.001;   // trend threshold (% per bar)
pub const EMA_ANGLE_SL: f64 = 0.005;
pub const EMA_ANGLE_TP: f64 = 0.004;
pub const EMA_ANGLE_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn ema_angle(closes: &[f64], ema_period: usize, lookback: usize) -> f64 {
    let n = closes.len();
    if n < ema_period + lookback {
        return 0.0;
    }

    let ema_now = ema(closes, ema_period);
    let prior_idx = n - 1 - lookback;
    let prior_closes = &closes[..=prior_idx];
    let ema_prior = ema(prior_closes, ema_period);

    (ema_now - ema_prior) / (lookback as f64)
}
```

---

## Validation Method

1. **Historical backtest** (run258_1_ema_angle_backtest.py)
2. **Walk-forward** (run258_2_ema_angle_wf.py)
3. **Combined** (run258_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 50
- LOOKBACK sweep: 3 / 5 / 10
- THRESH sweep: 0.0005 / 0.001 / 0.002
