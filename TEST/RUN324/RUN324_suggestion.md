# RUN324 — Stochastic RSI Divergence with Volume Confirmation

## Hypothesis

**Mechanism**: Stochastic RSI applies the stochastic formula to RSI values rather than prices — this produces a much more responsive oscillator. Divergence between StochRSI and price is a powerful signal: price making higher highs but StochRSI making lower highs = bearish divergence. When volume confirms the divergence (volume declining as price rises = distribution), the signal is stronger. Entry triggers on the StochRSI crossover from extreme zone.

**Why not duplicate**: RUN199 uses basic Stochastic RSI. RUN273 uses StochRSI percentile. RUN282 uses Stochastic divergence (not StochRSI). RUN215 uses RSI divergence. This RUN specifically combines StochRSI divergence with volume confirmation — the volume dimension added to StochRSI divergence is what makes it distinct.

## Proposed Config Changes (config.rs)

```rust
// ── RUN324: Stochastic RSI Divergence with Volume Confirmation ──────────────────
// stoch_rsi = stochastic(RSI(close, period), period)
// divergence lookback: find swing high (price) vs lower-high (stoch_rsi)
// volume_confirmation: volume declining during price rise = distribution (bearish)
// LONG: stoch_rsi crosses above STOCH_LEVEL from below AND price making higher low
// SHORT: stoch_rsi crosses below (100-STOCH_LEVEL) from above AND price making lower high

pub const STOCH_RSI_DIV_ENABLED: bool = true;
pub const STOCH_RSI_DIV_PERIOD: usize = 14;
pub const STOCH_RSI_DIV_STOCH: usize = 14;
pub const STOCH_RSI_DIV_LEVEL: f64 = 20.0;    // crossover from below for LONG
pub const STOCH_RSI_DIV_VOL_CONFIRM: bool = true;
pub const STOCH_RSI_DIV_SL: f64 = 0.005;
pub const STOCH_RSI_DIV_TP: f64 = 0.004;
pub const STOCH_RSI_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run324_1_stoch_rsi_div_backtest.py)
2. **Walk-forward** (run324_2_stoch_rsi_div_wf.py)
3. **Combined** (run324_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- STOCH period sweep: 14 / 20 / 28
- LEVEL sweep: 15 / 20 / 25
- VOL_CONFIRM sweep: true / false
