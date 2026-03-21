# RUN429 — Volume-Weighted EMA Crossover with ATR Volatility Confirmation

## Hypothesis

**Mechanism**: Volume-Weighted EMA crossover uses volume as weights in the exponential moving average calculation, making the crossover more responsive to volume-backed price moves. A standard EMA crossover fires when prices cross, but VW-EMA crossover fires when the volume-weighted average prices cross — only when the move has institutional backing. ATR Volatility Confirmation adds a volatility filter: only take crossover signals when ATR is above a minimum threshold (ensuring sufficient volatility for the trade to develop).

**Why not duplicate**: RUN364 uses Volume-Weighted EMA Crossover standalone. This RUN adds ATR Volatility Confirmation — the distinct mechanism is using ATR as a volatility filter to prevent crossover signals in low-volatility environments where EWMA crossovers tend to be false.

## Proposed Config Changes (config.rs)

```rust
// ── RUN429: Volume-Weighted EMA Crossover with ATR Volatility Confirmation ─────────────────────
// vw_ema_fast = volume-weighted EMA of fast period
// vw_ema_slow = volume-weighted EMA of slow period
// vw_cross: vw_ema_fast crosses above/below vw_ema_slow
// atr = average true range over period
// atr_confirm: atr > ATR_MIN_THRESH (sufficient volatility for trade)
// LONG: vw_cross bullish AND atr > ATR_MIN_THRESH
// SHORT: vw_cross bearish AND atr > ATR_MIN_THRESH

pub const VwEMA_ATR_ENABLED: bool = true;
pub const VwEMA_ATR_FAST_PERIOD: usize = 9;
pub const VwEMA_ATR_SLOW_PERIOD: usize = 21;
pub const VwEMA_ATR_VOL_PERIOD: usize = 20;
pub const VwEMA_ATR_ATR_PERIOD: usize = 14;
pub const VwEMA_ATR_ATR_MIN: f64 = 0.005;  // minimum ATR for trade viability
pub const VwEMA_ATR_SL: f64 = 0.005;
pub const VwEMA_ATR_TP: f64 = 0.004;
pub const VwEMA_ATR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run429_1_vwema_atr_backtest.py)
2. **Walk-forward** (run429_2_vwema_atr_wf.py)
3. **Combined** (run429_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 7 / 9 / 12
- SLOW_PERIOD sweep: 18 / 21 / 26
- VOL_PERIOD sweep: 14 / 20 / 30
- ATR_MIN sweep: 0.003 / 0.005 / 0.007
