# RUN403 — Volume-Weighted MACD Histogram with SuperTrend Confirmation

## Hypothesis

**Mechanism**: Volume-Weighted MACD replaces the standard EMA smoothing in MACD with volume-weighted EMA, making the MACD more responsive to volume-backed price moves. The MACD Histogram shows the difference between MACD and its signal line, visually representing momentum strength. SuperTrend provides trend direction and trailing stop. When the VW-MACD Histogram flips direction AND SuperTrend confirms the same direction, the signal has both volume-weighted momentum AND trend confirmation working together.

**Why not duplicate**: RUN361 uses MACD Histogram Slope with RSI Filter. RUN344 uses SuperTrend with MACD Histogram Divergence. This RUN specifically uses Volume-Weighted MACD (distinct calculation from standard MACD) with SuperTrend confirmation — the distinct mechanism is VW-MACD's volume-weighted smoothing making the histogram more responsive to volume-backed moves.

## Proposed Config Changes (config.rs)

```rust
// ── RUN403: Volume-Weighted MACD Histogram with SuperTrend Confirmation ─────────────────────
// vw_macd = vw_ema(close, fast_period) - vw_ema(close, slow_period)
// vw_macd_signal = vw_ema(vw_macd, signal_period)
// vw_macd_hist = vw_macd - vw_macd_signal
// hist_flip: histogram crosses above/below 0
// supertrend: atr-based trend direction
// LONG: histogram flips bullish AND supertrend bullish
// SHORT: histogram flips bearish AND supertrend bearish

pub const VWMACD_ST_ENABLED: bool = true;
pub const VWMACD_ST_FAST_PERIOD: usize = 12;
pub const VWMACD_ST_SLOW_PERIOD: usize = 26;
pub const VWMACD_ST_SIGNAL_PERIOD: usize = 9;
pub const VWMACD_ST_VOL_PERIOD: usize = 20;    // volume weighting period
pub const VWMACD_ST_ST_PERIOD: usize = 10;
pub const VWMACD_ST_ST_MULT: f64 = 3.0;
pub const VWMACD_ST_SL: f64 = 0.005;
pub const VWMACD_ST_TP: f64 = 0.004;
pub const VWMACD_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run403_1_vwmacd_st_backtest.py)
2. **Walk-forward** (run403_2_vwmacd_st_wf.py)
3. **Combined** (run403_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 8 / 12 / 16
- SLOW_PERIOD sweep: 21 / 26 / 30
- SIGNAL_PERIOD sweep: 7 / 9 / 12
- ST_MULT sweep: 2.0 / 3.0 / 4.0
