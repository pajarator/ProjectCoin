# RUN393 — Ichimoku Cloud with RSI Extreme Zone Confirmation

## Hypothesis

**Mechanism**: The Ichimoku Cloud (Tenkan-sen, Kijun-sen, Senkou Span A/B) provides comprehensive trend direction and support/resistance via the cloud (Kumo). The cloud itself acts as a dynamic support/resistance zone. Add RSI extreme zone confirmation: when price is retesting the cloud boundary AND RSI is in extreme territory (oversold <30 or overbought >70), the cloud boundary rejection has oscillator confirmation. This combines Ichimoku's structural support/resistance with RSI's reversal timing.

**Why not duplicate**: RUN299 uses Ichimoku Cloud Twist (TK cross + Kumo). RUN369 uses Ichimoku Cloud with Volume Confirmation. This RUN specifically uses Ichimoku cloud boundary retests combined with RSI extreme zones — the distinct mechanism is using cloud boundaries as structural retest levels with RSI extremes for timing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN393: Ichimoku Cloud with RSI Extreme Zone Confirmation ──────────────────────────
// tenkan_sen = (highest_high + lowest_low) / 2 over lookback
// kijun_sen = (highest_high + lowest_low) / 2 over standard period
// senkou_span_a = (tenkan + kijun) / 2, plotted ahead
// senkou_span_b = (highest_high + lowest_low) / 2 over period, plotted ahead
// kumo_cloud = area between senkou_span_a and senkou_span_b
// cloud_retest: price approaches cloud boundary after being rejected
// rsi_extreme: rsi < 30 (oversold) or rsi > 70 (overbought)
// LONG: price approaches cloud from below AND rsi < 30 AND price bounces from cloud
// SHORT: price approaches cloud from above AND rsi > 70 AND price bounces from cloud

pub const ICHIMOKU_RSI_ENABLED: bool = true;
pub const ICHIMOKU_RSI_TENKAN_PERIOD: usize = 9;
pub const ICHIMOKU_RSI_KIJUN_PERIOD: usize = 26;
pub const ICHIMOKU_RSI_SENKOU_PERIOD: usize = 52;
pub const ICHIMOKU_RSI_RSI_PERIOD: usize = 14;
pub const ICHIMOKU_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const ICHIMOKU_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const ICHIMOKU_RSI_SL: f64 = 0.005;
pub const ICHIMOKU_RSI_TP: f64 = 0.004;
pub const ICHIMOKU_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run393_1_ichimoku_rsi_backtest.py)
2. **Walk-forward** (run393_2_ichimoku_rsi_wf.py)
3. **Combined** (run393_3_combined.py)

## Out-of-Sample Testing

- TENKAN_PERIOD sweep: 7 / 9 / 12
- KIJUN_PERIOD sweep: 22 / 26 / 30
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
