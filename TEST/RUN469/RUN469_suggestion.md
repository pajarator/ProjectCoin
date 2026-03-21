# RUN469 — TRIX Momentum with ATR Volatility Confirmation

## Hypothesis

**Mechanism**: TRIX (Triple Exponential Average) is a momentum oscillator that filters out insignificant price movements by triple-smoothing. TRIX crossover of its signal line indicates trend changes with low lag. ATR Volatility Confirmation ensures volatility is expanding in the direction of the trade: when TRIX fires AND ATR is above its moving average, the momentum move has volatility-backed conviction and is less likely to be a false signal.

**Why not duplicate**: RUN386 uses TRIX Momentum with RSI Trade Timing Filter. This RUN uses ATR instead — the distinct mechanism is ATR volatility expansion confirmation versus RSI oscillator timing. ATR specifically measures market volatility rather than overbought/oversold conditions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN469: TRIX Momentum with ATR Volatility Confirmation ─────────────────────────────────
// trix: triple_exponential_average_oscillator momentum indicator
// trix_cross: trix crosses above/below signal line
// atr_expanding: atr > atr_sma AND atr increasing
// LONG: trix_cross bullish AND atr_expanding
// SHORT: trix_cross bearish AND atr_expanding

pub const TRIX_ATR_ENABLED: bool = true;
pub const TRIX_ATR_TRIX_PERIOD: usize = 15;
pub const TRIX_ATR_TRIX_SIGNAL: usize = 9;
pub const TRIX_ATR_ATR_PERIOD: usize = 14;
pub const TRIX_ATR_ATR_SMA_PERIOD: usize = 20;
pub const TRIX_ATR_SL: f64 = 0.005;
pub const TRIX_ATR_TP: f64 = 0.004;
pub const TRIX_ATR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run469_1_trix_atr_backtest.py)
2. **Walk-forward** (run469_2_trix_atr_wf.py)
3. **Combined** (run469_3_combined.py)

## Out-of-Sample Testing

- TRIX_PERIOD sweep: 10 / 15 / 20
- TRIX_SIGNAL sweep: 7 / 9 / 12
- ATR_PERIOD sweep: 10 / 14 / 20
- ATR_SMA_PERIOD sweep: 14 / 20 / 30
