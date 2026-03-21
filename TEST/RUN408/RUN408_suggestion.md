# RUN408 — Heikin-Ashi Momentum with Volume Trend Divergence

## Hypothesis

**Mechanism**: Heikin-Ashi candles smooth price data by averaging open/close/high/low, reducing noise and making trends easier to see. The momentum of Heikin-Ashi candles (how many consecutive candles are the same color) measures trend conviction. Volume Trend Divergence compares price momentum to volume momentum — when price is making strong moves in one direction but volume is weakening, a divergence forms. When HA momentum is strong AND volume trend diverges (confirming the move lacks volume backing), the reversal has high probability.

**Why not duplicate**: RUN327 uses Heikin-Ashi Smoothed Momentum standalone. RUN377 uses Momentum Exhaustion with Volume Divergence. This RUN specifically uses Heikin-Ashi candle momentum (which filters noise) with Volume Trend Divergence — the distinct mechanism is using HA's smooth momentum signal with volume divergence confirming the lack of institutional backing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN408: Heikin-Ashi Momentum with Volume Trend Divergence ───────────────────────────────
// heikin_ashi: ha_close = (open + high + low + close) / 4
// ha_momentum = consecutive HA candles in same direction
// volume_trend = slope of volume over lookback period
// vol_divergence: price making new highs but volume_trend declining
// LONG: ha_momentum weakens (fewer consecutive bullish candles) AND vol_divergence bullish
// SHORT: ha_momentum weakens (fewer consecutive bearish candles) AND vol_divergence bearish

pub const HA_VOL_DIV_ENABLED: bool = true;
pub const HA_VOL_DIV_MOM_PERIOD: usize = 5;     // consecutive candle count
pub const HA_VOL_DIV_VOL_PERIOD: usize = 20;     // volume trend lookback
pub const HA_VOL_DIV_SLOPE_THRESH: f64 = -0.1;   // volume trend must be declining
pub const HA_VOL_DIV_SL: f64 = 0.005;
pub const HA_VOL_DIV_TP: f64 = 0.004;
pub const HA_VOL_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run408_1_ha_vol_div_backtest.py)
2. **Walk-forward** (run408_2_ha_vol_div_wf.py)
3. **Combined** (run408_3_combined.py)

## Out-of-Sample Testing

- MOM_PERIOD sweep: 3 / 5 / 7
- VOL_PERIOD sweep: 14 / 20 / 30
- SLOPE_THRESH sweep: -0.05 / -0.1 / -0.15
