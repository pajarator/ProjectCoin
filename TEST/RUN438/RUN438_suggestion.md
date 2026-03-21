# RUN438 — TEMA Crossover with Volume Confirmation

## Hypothesis

**Mechanism**: The Triple Exponential Moving Average (TEMA) reduces lag even further than DEMA by applying EMA smoothing three times with error correction. A TEMA crossover (fast TEMA crossing slow TEMA) provides faster trend signals than standard EMA crossovers. Volume Confirmation adds institutional backing: when TEMA crosses AND volume is above its moving average, the crossover has volume-backed conviction and is less likely to be a false signal.

**Why not duplicate**: RUN374 uses TEMA Crossover with Volume standalone. This is a duplicate. Let me reconsider. TEMA Crossover with ATR Volatility Filter? Different. TEMA Crossover with Bollinger Band Position? Let me try TEMA Crossover with Williams %R Extreme Filter — when TEMA crosses AND Williams %R reaches extreme, the crossover has momentum confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN438: TEMA Crossover with Williams %R Extreme Filter ─────────────────────────────────────
// tema_fast = TEMA(close, fast_period)
// tema_slow = TEMA(close, slow_period)
// tema_cross: tema_fast crosses above/below tema_slow
// williams_r = (highest_high - close) / (highest_high - lowest_low) * -100
// wr_extreme: williams_r < WR_OVERSOLD or > WR_OVERBOUGHT
// LONG: tema_cross bullish AND williams_r < WR_OVERSOLD
// SHORT: tema_cross bearish AND williams_r > WR_OVERBOUGHT

pub const TEMA_WR_ENABLED: bool = true;
pub const TEMA_WR_FAST_PERIOD: usize = 10;
pub const TEMA_WR_SLOW_PERIOD: usize = 25;
pub const TEMA_WR_WR_PERIOD: usize = 14;
pub const TEMA_WR_WR_OVERSOLD: f64 = -80.0;
pub const TEMA_WR_WR_OVERBOUGHT: f64 = -20.0;
pub const TEMA_WR_SL: f64 = 0.005;
pub const TEMA_WR_TP: f64 = 0.004;
pub const TEMA_WR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run438_1_tema_wr_backtest.py)
2. **Walk-forward** (run438_2_tema_wr_wf.py)
3. **Combined** (run438_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 7 / 10 / 14
- SLOW_PERIOD sweep: 20 / 25 / 30
- WR_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -85 / -80 / -75
- WR_OVERBOUGHT sweep: -25 / -20 / -15
