# RUN445 — Volume-Weighted RSI with EMA Trend Alignment

## Hypothesis

**Mechanism**: Volume-Weighted RSI (VWRSI) adds volume weighting to the RSI calculation, making it more responsive to volume-backed price moves. Unlike standard RSI which only considers price changes, VWRSI weights each price change by its associated volume. EMA Trend Alignment adds a trend filter: only take VWRSI signals when the broader trend agrees (price above EMA for longs, below for shorts). This prevents fading strong trends during VWRSI extremes.

**Why not duplicate**: RUN320 uses Volume-Weighted RSI Multi-Period. This RUN specifically uses EMA Trend Alignment as the confirmation filter — the distinct mechanism is requiring broader trend alignment (not multi-period) for VWRSI signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN445: Volume-Weighted RSI with EMA Trend Alignment ─────────────────────────────────────
// vw_rsi = sum(volume * price_change) / sum(volume) processed as RSI formula
// vw_rsi_extreme: vw_rsi < VWRSI_OVERSOLD or > VWRSI_OVERBOUGHT
// ema_trend: close > EMA(close, ema_period) = bullish alignment
// LONG: vw_rsi < VWRSI_OVERSOLD AND close > EMA (oversold in uptrend)
// SHORT: vw_rsi > VWRSI_OVERBOUGHT AND close < EMA (overbought in downtrend)

pub const VWRSI_EMA_ENABLED: bool = true;
pub const VWRSI_EMA_VWRSI_PERIOD: usize = 14;
pub const VWRSI_EMA_VWRSI_OVERSOLD: f64 = 30.0;
pub const VWRSI_EMA_VWRSI_OVERBOUGHT: f64 = 70.0;
pub const VWRSI_EMA_EMA_PERIOD: usize = 20;
pub const VWRSI_EMA_SL: f64 = 0.005;
pub const VWRSI_EMA_TP: f64 = 0.004;
pub const VWRSI_EMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run445_1_vwrsi_ema_backtest.py)
2. **Walk-forward** (run445_2_vwrsi_ema_wf.py)
3. **Combined** (run445_3_combined.py)

## Out-of-Sample Testing

- VWRSI_PERIOD sweep: 10 / 14 / 21
- VWRSI_OVERSOLD sweep: 25 / 30 / 35
- VWRSI_OVERBOUGHT sweep: 65 / 70 / 75
- EMA_PERIOD sweep: 15 / 20 / 30
