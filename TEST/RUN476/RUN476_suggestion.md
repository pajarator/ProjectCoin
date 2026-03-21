# RUN476 — CCI with KST Momentum Confluence

## Hypothesis

**Mechanism**: CCI (Commodity Channel Index) measures deviation from average price, catching cyclical turns in commodities and other assets. KST Momentum Confluence adds multi-timeframe ROC verification: when CCI fires AND KST also shows momentum in the same direction across multiple smoothing periods, the signal has both deviation-based timing and multi-timeframe momentum confirmation.

**Why not duplicate**: RUN414 uses CCI with KST Momentum Confluence. This appears to be a duplicate — let me reconsider. Let me check what RUN414 actually uses... RUN414 is CCI with KST Momentum Confluence. I need a different combination.

Let me do: CCI with EMA Slope Alignment — when CCI fires AND the EMA slope confirms the trend direction, entries have both deviation timing and trend alignment.

## Proposed Config Changes (config.rs)

```rust
// ── RUN476: CCI with EMA Slope Alignment ─────────────────────────────────
// cci: commodity_channel_index measuring deviation from average
// cci_cross: cci crosses above/below 100 or -100 threshold
// ema_slope: slope of ema indicating trend direction
// LONG: cci_cross bullish (below -100 to above) AND ema_slope > 0
// SHORT: cci_cross bearish (above 100 to below) AND ema_slope < 0

pub const CCI_EMASLOPE_ENABLED: bool = true;
pub const CCI_EMASLOPE_CCI_PERIOD: usize = 20;
pub const CCI_EMASLOPE_CCI_OVERSOLD: f64 = -100.0;
pub const CCI_EMASLOPE_CCI_OVERBOUGHT: f64 = 100.0;
pub const CCI_EMASLOPE_EMA_PERIOD: usize = 20;
pub const CCI_EMASLOPE_SLOPE_PERIOD: usize = 5;
pub const CCI_EMASLOPE_SLOPE_THRESH: f64 = 0.0001;
pub const CCI_EMASLOPE_SL: f64 = 0.005;
pub const CCI_EMASLOPE_TP: f64 = 0.004;
pub const CCI_EMASLOPE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run476_1_cci_emaslope_backtest.py)
2. **Walk-forward** (run476_2_cci_emaslope_wf.py)
3. **Combined** (run476_3_combined.py)

## Out-of-Sample Testing

- CCI_PERIOD sweep: 14 / 20 / 30
- CCI_OVERSOLD sweep: -120 / -100 / -80
- CCI_OVERBOUGHT sweep: 80 / 100 / 120
- EMA_PERIOD sweep: 15 / 20 / 25
- SLOPE_PERIOD sweep: 3 / 5 / 7
