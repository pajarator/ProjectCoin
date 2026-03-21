# RUN358 — Stochastic RSI with ADX Trend Confirmation

## Hypothesis

**Mechanism**: Stochastic RSI provides early overbought/oversold signals. But in a strong trend, oscillators stay extended. Use ADX to confirm the trend is not too strong: if ADX > 30, the market is trending strongly, and mean-reversion (Stochastic RSI) signals will fail. Only trade Stochastic RSI signals when ADX < 30 (choppy or weak trend). This prevents fighting strong trends.

**Why not duplicate**: RUN199 uses Stochastic RSI. RUN282 uses Stochastic divergence. RUN270 uses VW Stochastic. No RUN combines Stochastic RSI with an ADX trend filter that gates mean-reversion based on trend strength.

## Proposed Config Changes (config.rs)

```rust
// ── RUN358: Stochastic RSI with ADX Trend Confirmation ────────────────────────────────
// stoch_rsi = stochastic(RSI(close, period), period)
// adx_choppy = ADX(period) < ADX_MAX (market is not strongly trending)
// LONG: stoch_rsi < STOCH_OVERSOLD AND adx_choppy
// SHORT: stoch_rsi > STOCH_OVERBOUGHT AND adx_choppy
// When ADX > ADX_MAX → no trades (trending too strongly for mean-reversion)

pub const STOCH_RSI_ADX_ENABLED: bool = true;
pub const STOCH_RSI_ADX_RSI_PERIOD: usize = 14;
pub const STOCH_RSI_ADX_STOCH_PERIOD: usize = 14;
pub const STOCH_RSI_ADX_OVERSOLD: f64 = 20.0;
pub const STOCH_RSI_ADX_OVERBOUGHT: f64 = 80.0;
pub const STOCH_RSI_ADX_ADX_PERIOD: usize = 14;
pub const STOCH_RSI_ADX_ADX_MAX: f64 = 30.0;
pub const STOCH_RSI_ADX_SL: f64 = 0.005;
pub const STOCH_RSI_ADX_TP: f64 = 0.004;
pub const STOCH_RSI_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run358_1_stoch_rsi_adx_backtest.py)
2. **Walk-forward** (run358_2_stoch_rsi_adx_wf.py)
3. **Combined** (run358_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- STOCH_PERIOD sweep: 14 / 20 / 28
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
- ADX_MAX sweep: 25 / 30 / 35
