# RUN487 — Stochastic RSI with EMA Crossover Confirmation

## Hypothesis

**Mechanism**: Stochastic RSI applies the Stochastic oscillator formula to RSI values instead of price, making it more sensitive to changes in RSI momentum. EMA Crossover provides trend direction via faster EMA crossing above/below slower EMA. When Stochastic RSI signals from oversold/overbought AND the EMA crossover confirms the same direction, entries have both oscillator timing and trend direction alignment.

**Why not duplicate**: RUN447 uses Stochastic RSI with VWAP Deviation Filter. This RUN uses EMA Crossover instead — distinct mechanism is EMA crossover trend confirmation versus VWAP deviation as a volatility filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN487: Stochastic RSI with EMA Crossover Confirmation ─────────────────────────────────
// stoch_rsi: stochastic applied to rsi values for sensitivity
// stoch_rsi_cross: stoch_rsi crosses above/below signal (20/80 lines)
// ema_crossover: fast_ema crosses above/below slow_ema
// LONG: stoch_rsi crosses above 20 AND ema_fast > ema_slow AND stoch_rsi rising
// SHORT: stoch_rsi crosses below 80 AND ema_fast < ema_slow AND stoch_rsi falling

pub const STOCHRSI_EMA_ENABLED: bool = true;
pub const STOCHRSI_EMA_RSI_PERIOD: usize = 14;
pub const STOCHRSI_EMA_STOCH_PERIOD: usize = 14;
pub const STOCHRSI_EMA_STOCH_K: usize = 3;
pub const STOCHRSI_EMA_STOCH_D: usize = 3;
pub const STOCHRSI_EMA_EMA_FAST: usize = 9;
pub const STOCHRSI_EMA_EMA_SLOW: usize = 21;
pub const STOCHRSI_EMA_SL: f64 = 0.005;
pub const STOCHRSI_EMA_TP: f64 = 0.004;
pub const STOCHRSI_EMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run487_1_stochrsi_ema_backtest.py)
2. **Walk-forward** (run487_2_stochrsi_ema_wf.py)
3. **Combined** (run487_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 20
- STOCH_PERIOD sweep: 10 / 14 / 20
- STOCH_K sweep: 2 / 3 / 5
- EMA_FAST sweep: 7 / 9 / 12
- EMA_SLOW sweep: 18 / 21 / 25
