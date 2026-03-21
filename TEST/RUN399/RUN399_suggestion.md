# RUN399 — ATR Ratio with RSI Z-Score Confluence

## Hypothesis

**Mechanism**: ATR (Average True Range) measures absolute volatility, but different price levels make ATR values incomparable across coins or over time. The ATR Ratio normalizes ATR relative to price (ATR/price), making it comparable. RSI Z-Score measures how far RSI has deviated from its mean in standard deviations. When ATR Ratio spikes (sudden volatility increase) AND RSI Z-Score reaches extreme values, both volatility AND momentum are confirming the same directional move. This confluence of volatility expansion + momentum extremes identifies high-probability turning points.

**Why not duplicate**: RUN346 uses ATR-Adjusted Williams %R. RUN360 uses ATR Percentile Rank with Trend Mode. RUN326 uses Z-Score Distance from VWAP. This RUN specifically combines ATR Ratio (a volatility-normalized measure) with RSI Z-Score (an oscillator deviation measure) as a confluence pair — the distinct mechanism is using ATR Ratio spikes to time RSI Z-Score entries, catching volatility expansions at extreme oscillator levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN399: ATR Ratio with RSI Z-Score Confluence ───────────────────────────────────────
// atr_ratio = ATR(period) / close  (normalized volatility)
// rsi_zscore = (rsi - SMA(rsi, period)) / STD(rsi, period)
// atr_spike: atr_ratio > ATR_RATIO_THRESH (sudden volatility increase)
// rsi_extreme: |rsi_zscore| > ZSCORE_THRESH (oscillator far from mean)
// LONG: atr_spike AND rsi_zscore < -ZSCORE_THRESH (oversold deviation)
// SHORT: atr_spike AND rsi_zscore > ZSCORE_THRESH (overbought deviation)

pub const ATR_RSI_Z_ENABLED: bool = true;
pub const ATR_RSI_Z_ATR_PERIOD: usize = 14;
pub const ATR_RSI_Z_ATR_RATIO_THRESH: f64 = 0.02;   // atr_ratio above this = spike
pub const ATR_RSI_Z_RSI_PERIOD: usize = 14;
pub const ATR_RSI_Z_RSI_Z_PERIOD: usize = 20;     // lookback for RSI mean/std
pub const ATR_RSI_Z_ZSCORE_THRESH: f64 = 2.0;      // standard deviations from mean
pub const ATR_RSI_Z_SL: f64 = 0.005;
pub const ATR_RSI_Z_TP: f64 = 0.004;
pub const ATR_RSI_Z_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run399_1_atr_rsi_z_backtest.py)
2. **Walk-forward** (run399_2_atr_rsi_z_wf.py)
3. **Combined** (run399_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 21
- ATR_RATIO_THRESH sweep: 0.015 / 0.02 / 0.025
- RSI_Z_PERIOD sweep: 15 / 20 / 30
- ZSCORE_THRESH sweep: 1.5 / 2.0 / 2.5
