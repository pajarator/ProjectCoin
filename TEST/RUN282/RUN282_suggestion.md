# RUN282 — Stochastic Divergence: Price-Oscillator Discord for Reversal

## Hypothesis

**Mechanism**: Like RSI divergence (RUN215), Stochastic divergence identifies when price and the Stochastic oscillator disagree. Bullish divergence: price makes lower low, Stochastic makes higher low. Bearish divergence: price makes higher high, Stochastic makes lower high. Divergence signals the end of a move.

**Why not duplicate**: RUN215 uses RSI divergence. RUN166 uses Stochastic divergence. RUN199 uses Stochastic RSI. This is Stochastic (standard) divergence — a distinct oscillator from RSI.

## Proposed Config Changes (config.rs)

```rust
// ── RUN282: Stochastic Divergence ────────────────────────────────────────
// stoch = Stochastic(high, low, close, 14)
// Look for 5-bar swing in price and stoch
// BULLISH: price.lower_low AND stoch.higher_low → LONG
// BEARISH: price.higher_high AND stoch.lower_high → SHORT

pub const STOCH_DIV_ENABLED: bool = true;
pub const STOCH_DIV_PERIOD: usize = 14;
pub const STOCH_DIV_SIGNAL: usize = 3;
pub const STOCH_DIV_SWING: usize = 5;
pub const STOCH_DIV_SL: f64 = 0.005;
pub const STOCH_DIV_TP: f64 = 0.004;
pub const STOCH_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run282_1_stoch_div_backtest.py)
2. **Walk-forward** (run282_2_stoch_div_wf.py)
3. **Combined** (run282_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- SIGNAL sweep: 3 / 5 / 7
- SWING sweep: 3 / 5 / 7
