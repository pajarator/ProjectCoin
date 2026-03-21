# RUN377 — Momentum Exhaustion with Volume Divergence

## Hypothesis

**Mechanism**: Momentum exhaustion occurs when a sustained move loses steam — price continues but volume declines, indicating the move lacks conviction. Measure momentum (ROC) and compare to its own volume trend. When price makes a new high but volume is declining = hidden bearish divergence (exhaustion). When price makes a new low but volume is declining = hidden bullish divergence (accumulation).

**Why not duplicate**: No prior RUN combines momentum exhaustion detection with volume divergence. RUN99 uses Z-score momentum divergence. This RUN specifically uses volume divergence as the exhaustion signal — price momentum continuing without volume backing it = high probability reversal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN377: Momentum Exhaustion with Volume Divergence ────────────────────────────────
// momentum = ROC(close, MOM_PERIOD)
// volume_trend = slope of volume over lookback
// exhaustion_up = price.new_high AND volume_trend < 0 (price rising but volume falling)
// exhaustion_down = price.new_low AND volume_trend < 0 (price falling but volume falling)
// LONG: exhaustion_down AND RSI < RSI_OVERSOLD
// SHORT: exhaustion_up AND RSI > RSI_OVERBOUGHT

pub const MOM_EXHAUST_ENABLED: bool = true;
pub const MOM_EXHAUST_MOM_PERIOD: usize = 10;
pub const MOM_EXHAUST_VOL_LOOKBACK: usize = 20;
pub const MOM_EXHAUST_RSI_PERIOD: usize = 14;
pub const MOM_EXHAUST_RSI_OVERSOLD: f64 = 35.0;
pub const MOM_EXHAUST_RSI_OVERBOUGHT: f64 = 65.0;
pub const MOM_EXHAUST_SL: f64 = 0.005;
pub const MOM_EXHAUST_TP: f64 = 0.004;
pub const MOM_EXHAUST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run377_1_mom_exhaust_backtest.py)
2. **Walk-forward** (run377_2_mom_exhaust_wf.py)
3. **Combined** (run377_3_combined.py)

## Out-of-Sample Testing

- MOM_PERIOD sweep: 5 / 10 / 20
- VOL_LOOKBACK sweep: 14 / 20 / 30
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
