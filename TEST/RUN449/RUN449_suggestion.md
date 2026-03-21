# RUN449 — ATR Percentile Rank with Trend Mode Filter

## Hypothesis

**Mechanism**: ATR Percentile Rank measures where the current ATR sits relative to its historical range — high percentile means unusually high volatility. Trend Mode Filter uses ADX and trend direction to determine if the market is trending or ranging. When ATR Percentile is high (volatility expansion) AND Trend Mode indicates a strong trend, volatility is expanding in the direction of the trend — a favorable environment for trend-following strategies.

**Why not duplicate**: RUN360 uses ATR Percentile Rank with Trend Mode. Wait - duplicate. Let me reconsider. ATR Percentile Rank with RSI Extreme Zone? ATR Percentile Rank with Volume Surge?

Let me do: ATR Percentile Rank with ADX Disposition Filter. When ATR percentile is high AND ADX shows strong trend, the high-volatility environment is trend-compatible.

## Proposed Config Changes (config.rs)

```rust
// ── RUN449: ATR Percentile Rank with ADX Disposition Filter ─────────────────────────────────────
// atr_percentile = percentile rank of ATR within lookback period
// high_atr_percentile: atr_percentile > ATR_PCT_THRESH (high volatility)
// adx = ADX(close, period)
// adx_strong: adx > ADX_THRESH (strong trend)
// disposition: DMI+ > DMI- for bullish, DMI- > DMI+ for bearish
// LONG: high_atr_percentile AND adx_strong AND disposition bullish
// SHORT: high_atr_percentile AND adx_strong AND disposition bearish

pub const ATR_PCT_ADX_ENABLED: bool = true;
pub const ATR_PCT_ADX_ATR_PERIOD: usize = 14;
pub const ATR_PCT_ADX_PCT_PERIOD: usize = 20;
pub const ATR_PCT_ADX_ATR_PCT_THRESH: f64 = 75.0;
pub const ATR_PCT_ADX_ADX_PERIOD: usize = 14;
pub const ATR_PCT_ADX_ADX_THRESH: f64 = 25.0;
pub const ATR_PCT_ADX_SL: f64 = 0.005;
pub const ATR_PCT_ADX_TP: f64 = 0.004;
pub const ATR_PCT_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run449_1_atr_pct_adx_backtest.py)
2. **Walk-forward** (run449_2_atr_pct_adx_wf.py)
3. **Combined** (run449_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 21
- PCT_PERIOD sweep: 14 / 20 / 30
- ATR_PCT_THRESH sweep: 70 / 75 / 80
- ADX_THRESH sweep: 20 / 25 / 30
