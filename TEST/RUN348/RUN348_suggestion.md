# RUN348 — ADX Disposition with EMA Trend Confirmation

## Hypothesis

**Mechanism**: ADX Disposition = (+DI - -DI) / (+DI + -DI). This normalized measure of directional bias ranges from -1 (strong bearish) to +1 (strong bullish). When disposition crosses above +DISP_THRESH → bullish momentum dominates. When it crosses below -DISP_THRESH → bearish momentum dominates. Filter with EMA trend: only take LONG when price > EMA AND disposition turning positive; only take SHORT when price < EMA AND disposition turning negative. The EMA filter prevents fighting the trend.

**Why not duplicate**: RUN292 uses ADX disposition as a filter. This RUN specifically adds the EMA trend confirmation — the EMA adds a directional filter that prevents counter-trend trades when disposition turns but the overall trend is opposing it.

## Proposed Config Changes (config.rs)

```rust
// ── RUN348: ADX Disposition with EMA Trend Confirmation ─────────────────────────────
// disposition = (+DI - -DI) / (+DI + -DI)
// disposition_smooth = EMA(disposition, period)
// ema_trend_up = close > EMA(close, trend_period)
// ema_trend_down = close < EMA(close, trend_period)
// LONG: disposition_smooth crosses above DISP_THRESH AND ema_trend_up
// SHORT: disposition_smooth crosses below -DISP_THRESH AND ema_trend_down

pub const ADX_DISP_EMA_ENABLED: bool = true;
pub const ADX_DISP_PERIOD: usize = 14;
pub const ADX_DISP_EMA_PERIOD: usize = 20;
pub const ADX_DISP_THRESH: f64 = 0.1;      // disposition threshold
pub const ADX_DISP_SL: f64 = 0.005;
pub const ADX_DISP_TP: f64 = 0.004;
pub const ADX_DISP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run348_1_adx_disp_ema_backtest.py)
2. **Walk-forward** (run348_2_adx_disp_ema_wf.py)
3. **Combined** (run348_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- THRESH sweep: 0.05 / 0.1 / 0.15
- EMA_PERIOD sweep: 15 / 20 / 30
