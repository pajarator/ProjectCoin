# RUN456 — SuperTrend with RSI Extreme Filter

## Hypothesis

**Mechanism**: SuperTrend provides clear trend direction but can whipsaw in ranging markets. RSI Extreme Filter adds timing precision: only take SuperTrend signals when RSI is at extreme levels (oversold <30 or overbought >70). This ensures trades are taken at the beginning of new trends when momentum is also extreme, improving entry timing.

**Why not duplicate**: RUN455 uses SuperTrend with Volume Confirmation. This RUN uses RSI Extreme Filter instead — the distinct mechanism is using RSI extremes to filter SuperTrend signals, ensuring entries only when momentum oscillators confirm the trend change.

## Proposed Config Changes (config.rs)

```rust
// ── RUN456: SuperTrend with RSI Extreme Filter ─────────────────────────────────
// supertrend: ATR-based trend direction and trailing stop
// supertrend_flip: trend changes from bullish to bearish or vice versa
// rsi_extreme: rsi < RSI_OVERSOLD or rsi > RSI_OVERBOUGHT
// LONG: supertrend_flip bullish AND rsi < RSI_OVERSOLD
// SHORT: supertrend_flip bearish AND rsi > RSI_OVERBOUGHT

pub const ST_RSI_ENABLED: bool = true;
pub const ST_RSI_ST_PERIOD: usize = 10;
pub const ST_RSI_ST_MULT: f64 = 3.0;
pub const ST_RSI_RSI_PERIOD: usize = 14;
pub const ST_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const ST_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const ST_RSI_SL: f64 = 0.005;
pub const ST_RSI_TP: f64 = 0.004;
pub const ST_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run456_1_st_rsi_backtest.py)
2. **Walk-forward** (run456_2_st_rsi_wf.py)
3. **Combined** (run456_3_combined.py)

## Out-of-Sample Testing

- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
