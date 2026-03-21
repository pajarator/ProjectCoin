# RUN407 — Williams %R with ADX Trend Strength and EMA Alignment

## Hypothesis

**Mechanism**: Williams %R is an overbought/oversold oscillator that measures the current close relative to the high-low range. It oscillates between 0 and -100. Many traders use it incorrectly by just buying when oversold — but in a strong trend, %R can stay overbought/oversold for extended periods. Add ADX as a trend strength filter AND require EMA alignment: only take %R signals when ADX > 25 (trend has strength) AND price is aligned with the EMA direction. This triple confirmation ensures signals fire in trending markets with directional alignment.

**Why not duplicate**: RUN306 uses Williams %R Multi-Timeframe Confluence. RUN346 uses ATR-Adjusted Williams %R. RUN371 uses Williams %R with RSI Confluence. This RUN specifically uses the combination of ADX trend strength filter AND EMA direction alignment with Williams %R — a triple-confirmation approach distinct from any single or dual confirmation approach.

## Proposed Config Changes (config.rs)

```rust
// ── RUN407: Williams %R with ADX Trend Strength and EMA Alignment ─────────────────────────────
// williams_r = (highest_high - close) / (highest_high - lowest_low) * -100
// adx = ADX(close, period) measuring trend strength
// ema_direction: close > EMA(close, ema_period) = bullish, else bearish
// LONG: williams_r < WR_OVERSOLD AND adx > ADX_THRESH AND close > EMA
// SHORT: williams_r > WR_OVERBOUGHT AND adx > ADX_THRESH AND close < EMA

pub const WR_ADX_EMA_ENABLED: bool = true;
pub const WR_ADX_EMA_WR_PERIOD: usize = 14;
pub const WR_ADX_EMA_WR_OVERSOLD: f64 = -80.0;
pub const WR_ADX_EMA_WR_OVERBOUGHT: f64 = -20.0;
pub const WR_ADX_EMA_ADX_PERIOD: usize = 14;
pub const WR_ADX_EMA_ADX_THRESH: f64 = 25.0;   // above this = trending
pub const WR_ADX_EMA_EMA_PERIOD: usize = 20;
pub const WR_ADX_EMA_SL: f64 = 0.005;
pub const WR_ADX_EMA_TP: f64 = 0.004;
pub const WR_ADX_EMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run407_1_wr_adx_ema_backtest.py)
2. **Walk-forward** (run407_2_wr_adx_ema_wf.py)
3. **Combined** (run407_3_combined.py)

## Out-of-Sample Testing

- WR_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -85 / -80 / -75
- WR_OVERBOUGHT sweep: -25 / -20 / -15
- ADX_THRESH sweep: 20 / 25 / 30
