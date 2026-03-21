# RUN460 — Elder Ray Index with Stochastic Confirmation

## Hypothesis

**Mechanism**: Elder Ray Index combines EMA analysis with price action to measure buying/selling pressure. Bull Power = High - EMA, Bear Power = Low - EMA. When EMA slopes upward AND Bull Power is positive, buyers are in control. Stochastic Confirmation ensures the momentum signal is corroborated by an overbought/oversold oscillator: only take Elder Ray signals when Stochastic is confirming the direction from non-extreme territory.

**Why not duplicate**: RUN453 uses Elder Ray Index with Volume Confirmation. This RUN uses Stochastic instead — the distinct mechanism is momentum corroboration via oscillator rather than volume. This ensures entries only when both trend (Elder Ray) and momentum (Stochastic) align, filtering out weak signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN460: Elder Ray Index with Stochastic Confirmation ─────────────────────────────────
// elder_ray: bull_power = high - ema, bear_power = low - ema
// elder_ray_cross: bull_power crosses above/below ema level
// stoch_confirm: stochastic in non-extreme territory (20-80)
// LONG: bull_power > 0 AND ema_slope_up AND stoch_cross bullish AND stoch 20-80
// SHORT: bear_power < 0 AND ema_slope_down AND stoch_cross bearish AND stoch 20-80

pub const ELDER_STOCH_ENABLED: bool = true;
pub const ELDER_STOCH_EMA_PERIOD: usize = 13;
pub const ELDER_STOCH_STOCH_PERIOD: usize = 14;
pub const ELDER_STOCH_STOCH_K: usize = 3;
pub const ELDER_STOCH_STOCH_D: usize = 3;
pub const ELDER_STOCH_STOCH_LOW: f64 = 20.0;
pub const ELDER_STOCH_STOCH_HIGH: f64 = 80.0;
pub const ELDER_STOCH_SL: f64 = 0.005;
pub const ELDER_STOCH_TP: f64 = 0.004;
pub const ELDER_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run460_1_elder_stoch_backtest.py)
2. **Walk-forward** (run460_2_elder_stoch_wf.py)
3. **Combined** (run460_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 13 / 20
- STOCH_PERIOD sweep: 10 / 14 / 20
- STOCH_K sweep: 2 / 3 / 5
- STOCH_LOW sweep: 15 / 20 / 25
- STOCH_HIGH sweep: 75 / 80 / 85
