# RUN381 — Keltner Channel with Stochastic Overbought/Oversold Filter

## Hypothesis

**Mechanism**: Keltner Channel breakout occurs when price closes beyond the bands. However, many Keltner breakouts fail. Add a Stochastic filter: only take LONG when Stochastic is oversold (<20) AND price breaks above upper band. Only take SHORT when Stochastic is overbought (>80) AND price breaks below lower band. Stochastic acts as a confirmation that the market is in a reversal state rather than a continuation state.

**Why not duplicate**: RUN188 uses Keltner breakout. RUN318 uses Keltner with volume. This RUN specifically adds Stochastic overbought/oversold filter — the distinct mechanism is using Stochastic to confirm reversals rather than continuations.

## Proposed Config Changes (config.rs)

```rust
// ── RUN381: Keltner Channel with Stochastic Filter ────────────────────────────────
// keltner_upper = EMA(close, period) + ATR_mult * ATR
// keltner_lower = EMA(close, period) - ATR_mult * ATR
// LONG: close crosses above upper_band AND stoch_k < STOCH_OVERSOLD
// SHORT: close crosses below lower_band AND stoch_k > STOCH_OVERBOUGHT

pub const KC_STOCH_ENABLED: bool = true;
pub const KC_STOCH_ATR_PERIOD: usize = 14;
pub const KC_STOCH_EMA_PERIOD: usize = 20;
pub const KC_STOCH_ATR_MULT: f64 = 2.0;
pub const KC_STOCH_STOCH_PERIOD: usize = 14;
pub const KC_STOCH_STOCH_OVERSOLD: f64 = 20.0;
pub const KC_STOCH_STOCH_OVERBOUGHT: f64 = 80.0;
pub const KC_STOCH_SL: f64 = 0.005;
pub const KC_STOCH_TP: f64 = 0.004;
pub const KC_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run381_1_kc_stoch_backtest.py)
2. **Walk-forward** (run381_2_kc_stoch_wf.py)
3. **Combined** (run381_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- EMA_PERIOD sweep: 15 / 20 / 30
- STOCH_OVERSOLD sweep: 15 / 20 / 25
- STOCH_OVERBOUGHT sweep: 75 / 80 / 85
