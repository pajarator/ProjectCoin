# RUN337 — Stochastic Extreme Shift: %K-%D Crossover with Regime Filter

## Hypothesis

**Mechanism**: Standard stochastic crossover uses fixed levels (e.g., %K crosses %D anywhere). This RUN requires the crossover to occur in extreme zones — %K crossing %D while %K is below 20 (oversold) or above 80 (overbought). The extreme zone crossover signals exhaustion followed by shift in momentum. Regime filter: only enter LONG when in LONG/ISO_SHORT regime, only enter SHORT when in SHORT/ISO_SHORT regime.

**Why not duplicate**: RUN270 uses VW Stochastic (different calculation). RUN282 uses Stochastic divergence. No RUN specifically uses extreme-zone crossover with regime filtering. The combination of extreme zone + crossover + regime filter is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN337: Stochastic Extreme Shift ───────────────────────────────────────────
// %K = stochastic(close, 14)
// %D = SMA(%K, 3)
// crossover_in_oversold = %K crosses above %D AND %K < 20
// crossover_in_overbought = %K crosses below %D AND %K > 80
// LONG: crossover_in_oversold AND regime in [LONG, ISO_SHORT]
// SHORT: crossover_in_overbought AND regime in [SHORT, ISO_SHORT]

pub const STOCH_SHIFT_ENABLED: bool = true;
pub const STOCH_SHIFT_K_PERIOD: usize = 14;
pub const STOCH_SHIFT_D_PERIOD: usize = 3;
pub const STOCH_SHIFT_OVERSOLD: f64 = 20.0;
pub const STOCH_SHIFT_OVERBOUGHT: f64 = 80.0;
pub const STOCH_SHIFT_SL: f64 = 0.005;
pub const STOCH_SHIFT_TP: f64 = 0.004;
pub const STOCH_SHIFT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run337_1_stoch_shift_backtest.py)
2. **Walk-forward** (run337_2_stoch_shift_wf.py)
3. **Combined** (run337_3_combined.py)

## Out-of-Sample Testing

- K_PERIOD sweep: 10 / 14 / 21
- D_PERIOD sweep: 2 / 3 / 5
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
