# RUN416 — Trend Resonance Factor with Williams %R Extreme

## Hypothesis

**Mechanism**: Trend Resonance Factor measures how many different trend indicators (EMA slope, MACD direction, ADX level) are aligned in the same direction. When all indicators agree, the trend has resonance and is stronger. Williams %R identifies overbought/oversold extremes. When Trend Resonance Factor is high (multiple indicators aligned) AND Williams %R reaches extreme, the trending move is reaching an extreme that has trend confirmation — a high-probability reversal point.

**Why not duplicate**: RUN322 uses Trend Resonance Factor standalone. RUN371 uses Williams %R with RSI Confluence. This RUN specifically combines Trend Resonance Factor (multiple trend indicators aligned) with Williams %R extremes — the distinct mechanism is using multi-indicator trend alignment to confirm Williams %R extremes, filtering out extremes that occur against the trend.

## Proposed Config Changes (config.rs)

```rust
// ── RUN416: Trend Resonance Factor with Williams %R Extreme ─────────────────────────────────
// trend_resonance = count of indicators aligned (ema_slope, macd_dir, adx_level)
// high_resonance = trend_resonance >= RESONANCE_THRESH
// williams_r = (highest_high - close) / (highest_high - lowest_low) * -100
// wr_extreme: williams_r < WR_OVERSOLD or > WR_OVERBOUGHT
// LONG: high_resonance AND williams_r < WR_OVERSOLD (oversold in strong uptrend)
// SHORT: high_resonance AND williams_r > WR_OVERBOUGHT (overbought in strong downtrend)

pub const TRF_WR_ENABLED: bool = true;
pub const TRF_WR_EMA_PERIOD: usize = 20;
pub const TRF_WR_MACD_FAST: usize = 12;
pub const TRF_WR_MACD_SLOW: usize = 26;
pub const TRF_WR_ADX_PERIOD: usize = 14;
pub const TRF_WR_RESONANCE_THRESH: u32 = 3;    // all 3 indicators aligned
pub const TRF_WR_WR_PERIOD: usize = 14;
pub const TRF_WR_WR_OVERSOLD: f64 = -80.0;
pub const TRF_WR_WR_OVERBOUGHT: f64 = -20.0;
pub const TRF_WR_SL: f64 = 0.005;
pub const TRF_WR_TP: f64 = 0.004;
pub const TRF_WR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run416_1_trf_wr_backtest.py)
2. **Walk-forward** (run416_2_trf_wr_wf.py)
3. **Combined** (run416_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 15 / 20 / 30
- RESONANCE_THRESH sweep: 2 / 3 / 4
- WR_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -85 / -80 / -75
- WR_OVERBOUGHT sweep: -25 / -20 / -15
