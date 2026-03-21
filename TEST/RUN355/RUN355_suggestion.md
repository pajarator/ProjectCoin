# RUN355 — Laguerre RSI with ATR Volatility Filter

## Hypothesis

**Mechanism**: Laguerre RSI uses a gamma parameter to create a more responsive RSI variant. Lower gamma = more smoothing. Higher gamma = more responsiveness. Add an ATR filter: only trade Laguerre RSI signals when ATR is above its 20-bar SMA (market is volatile enough for mean-reversion to work). When ATR is below its moving average (low volatility), suppress signals because mean-reversion strategies underperform in quiet markets.

**Why not duplicate**: RUN13 uses Laguerre RSI as a complement signal (basic Laguerre). This RUN specifically adds the ATR volatility filter — the distinct mechanism is only trading Laguerre RSI signals when volatility is elevated.

## Proposed Config Changes (config.rs)

```rust
// ── RUN355: Laguerre RSI with ATR Volatility Filter ────────────────────────────────
// laguerre_rsi = laguerre_rsi(close, gamma)
// atr_above_ma = ATR(period) > SMA(ATR(period), ATR_MA_PERIOD)
// LONG: laguerre_rsi < LRSI_OVERSOLD AND atr_above_ma
// SHORT: laguerre_rsi > LRSI_OVERBOUGHT AND atr_above_ma
// Filter: if ATR < ATR_SMA → no trades (low volatility = suppress mean-reversion)

pub const LRSI_ATR_ENABLED: bool = true;
pub const LRSI_ATR_GAMMA: f64 = 0.5;
pub const LRSI_ATR_OVERSOLD: f64 = 0.15;
pub const LRSI_ATR_OVERBOUGHT: f64 = 0.85;
pub const LRSI_ATR_ATR_PERIOD: usize = 14;
pub const LRSI_ATR_ATR_MA_PERIOD: usize = 20;
pub const LRSI_ATR_SL: f64 = 0.005;
pub const LRSI_ATR_TP: f64 = 0.004;
pub const LRSI_ATR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run355_1_lgrsi_atr_backtest.py)
2. **Walk-forward** (run355_2_lgrsi_atr_wf.py)
3. **Combined** (run355_3_combined.py)

## Out-of-Sample Testing

- GAMMA sweep: 0.3 / 0.5 / 0.7
- OVERSOLD sweep: 0.10 / 0.15 / 0.20
- OVERBOUGHT sweep: 0.80 / 0.85 / 0.90
- ATR_MA_PERIOD sweep: 14 / 20 / 30
