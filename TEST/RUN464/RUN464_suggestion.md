# RUN464 — TEMA Crossover with RSI Adaptive Filter

## Hypothesis

**Mechanism**: TEMA (Triple Exponential Moving Average) reduces lag while maintaining smoothness compared to single or double EMAs. When TEMA crosses the price or another MA, it signals trend changes with less delay. RSI Adaptive Filter makes RSI's period dynamic based on volatility: in high volatility, RSI uses a shorter period for faster signals; in low volatility, a longer period for smoother signals. This prevents TEMA crossovers from triggering during choppy, low-volatility periods.

**Why not duplicate**: RUN438 uses TEMA Crossover with Williams %R Extreme Filter. This RUN uses RSI Adaptive instead — the distinct mechanism is adaptive RSI period that changes with volatility conditions, filtering TEMA signals only when volatility regime supports momentum moves.

## Proposed Config Changes (config.rs)

```rust
// ── RUN464: TEMA Crossover with RSI Adaptive Filter ─────────────────────────────────
// tema: triple_exponential_moving_average with reduced lag
// tema_cross: tema crosses above/below price or signal MA
// rsi_adaptive: rsi period = base_period * vol_ratio (adapts to volatility)
// LONG: tema_cross bullish AND rsi_adaptive in non-extreme zone (30-70)
// SHORT: tema_cross bearish AND rsi_adaptive in non-extreme zone (30-70)

pub const TEMA_RSIADAPT_ENABLED: bool = true;
pub const TEMA_RSIADAPT_TEMA_PERIOD: usize = 20;
pub const TEMA_RSIADAPT_RSI_BASE: usize = 14;
pub const TEMA_RSIADAPT_RSI_VOL_PERIOD: usize = 20;
pub const TEMA_RSIADAPT_RSI_LOW: f64 = 30.0;
pub const TEMA_RSIADAPT_RSI_HIGH: f64 = 70.0;
pub const TEMA_RSIADAPT_SL: f64 = 0.005;
pub const TEMA_RSIADAPT_TP: f64 = 0.004;
pub const TEMA_RSIADAPT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run464_1_tema_rsiadapt_backtest.py)
2. **Walk-forward** (run464_2_tema_rsiadapt_wf.py)
3. **Combined** (run464_3_combined.py)

## Out-of-Sample Testing

- TEMA_PERIOD sweep: 15 / 20 / 25 / 30
- RSI_BASE sweep: 10 / 14 / 20
- RSI_VOL_PERIOD sweep: 14 / 20 / 30
- RSI_LOW sweep: 25 / 30 / 35
- RSI_HIGH sweep: 65 / 70 / 75
