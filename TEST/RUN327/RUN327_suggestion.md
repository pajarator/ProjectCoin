# RUN327 — Heikin-Ashi Smoothed Momentum: Noise-Filtered Candle Patterns

## Hypothesis

**Mechanism**: Heikin-Ashi candles smooth price data using: HA_Close = (O+H+L+C)/2, HA_Open = prior HA_Open + (prior HA_Close - prior HA_Open)/2. This filters out noise. A HA candle with close > open (green) and higher low than prior = strong uptrend. HA with close < open (red) and lower high than prior = strong downtrend. Momentum signals from HA trend changes combined with RSI to avoid entries at extremes.

**Why not duplicate**: No prior RUN uses Heikin-Ashi candles directly. RUN239 uses inside bar patterns (raw OHLCV). RUN240 uses outside bar reversal (raw OHLCV). HA smoothing is fundamentally different from raw candlestick patterns — it creates a smoothed trend line effect, reducing false signals from noise.

## Proposed Config Changes (config.rs)

```rust
// ── RUN327: Heikin-Ashi Smoothed Momentum ─────────────────────────────────────
// ha_close = (open + high + low + close) / 2
// ha_open = prior_ha_open + (prior_ha_close - prior_ha_open) / 2
// ha_high = max(high, ha_open, ha_close)
// ha_low = min(low, ha_open, ha_close)
// bullish_ha = ha_close > ha_open AND ha_low > ha_low[1]
// bearish_ha = ha_close < ha_open AND ha_high < ha_high[1]
// LONG: bullish_ha AND RSI < RSI_MAX (avoid overbought entries)
// SHORT: bearish_ha AND RSI > RSI_MIN (avoid oversold entries)

pub const HA_MOM_ENABLED: bool = true;
pub const HA_MOM_RSI_MAX: f64 = 70.0;        // max RSI for LONG (avoid overbought)
pub const HA_MOM_RSI_MIN: f64 = 30.0;        // min RSI for SHORT (avoid oversold)
pub const HA_MOM_SL: f64 = 0.005;
pub const HA_MOM_TP: f64 = 0.004;
pub const HA_MOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run327_1_ha_mom_backtest.py)
2. **Walk-forward** (run327_2_ha_mom_wf.py)
3. **Combined** (run327_3_combined.py)

## Out-of-Sample Testing

- RSI_MAX sweep: 60 / 70 / 80
- RSI_MIN sweep: 20 / 30 / 40
