# RUN477 — Ichimoku Cloud with Volume Profile POC

## Hypothesis

**Mechanism**: Ichimoku Cloud provides comprehensive trend direction via the relationship between price and cloud boundaries (Senkou Span A/B). Conversion Line and Base Line crossovers give early momentum signals. Volume Profile Point of Control (POC) shows the price level with highest volume traded. When Ichimoku signals align with POC as support/resistance, entries have both cloud-based trend direction and volume profile structural confirmation.

**Why not duplicate**: RUN393 uses Ichimoku Cloud with RSI Extreme Zone Confirmation. This RUN uses Volume Profile POC instead — distinct mechanism is volume-based structural support/resistance versus RSI oscillator extremes. POC identifies where the most trading activity occurred.

## Proposed Config Changes (config.rs)

```rust
// ── RUN477: Ichimoku Cloud with Volume Profile POC ─────────────────────────────────
// ichimoku_cloud: tenkan_sen, kijun_sen, senkou_span_a/b cloud
// ichimoku_signal: price above cloud (bullish) or below (bearish)
// vol_profile_poc: price level with highest volume traded
// LONG: price above cloud AND price > poc AND conversion_cross bullish
// SHORT: price below cloud AND price < poc AND conversion_cross bearish

pub const ICHIMOKU_POC_ENABLED: bool = true;
pub const ICHIMOKU_POC_TENKAN: usize = 9;
pub const ICHIMOKU_POC_KIJUN: usize = 26;
pub const ICHIMOKU_POC_SENKOU: usize = 52;
pub const ICHIMOKU_POC_POC_PERIOD: usize = 20;
pub const ICHIMOKU_POC_SL: f64 = 0.005;
pub const ICHIMOKU_POC_TP: f64 = 0.004;
pub const ICHIMOKU_POC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run477_1_ichimoku_poc_backtest.py)
2. **Walk-forward** (run477_2_ichimoku_poc_wf.py)
3. **Combined** (run477_3_combined.py)

## Out-of-Sample Testing

- TENKAN sweep: 7 / 9 / 12
- KIJUN sweep: 20 / 26 / 30
- SENKOU sweep: 45 / 52 / 60
- POC_PERIOD sweep: 14 / 20 / 30
