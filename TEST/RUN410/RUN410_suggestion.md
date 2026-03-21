# RUN410 — Keltner Channel with Volume Percentage Delta Confirmation

## Hypothesis

**Mechanism**: Keltner Channel uses ATR-multiplied bands around an EMA to define dynamic support/resistance. Volume Percentage Delta measures how much volume is occurring at the extremes of the range vs the middle. When Keltner bands are touched AND Volume Percentage Delta shows volume is concentrating at the touch point (vs the middle), the touch has conviction. This distinguishes between exhausted touches (volume at extremes, likely reversal) and unconvinced touches.

**Why not duplicate**: RUN318 uses Keltner Channel with Volume Confirmation. RUN381 uses Keltner Channel with Stochastic Filter. This RUN specifically uses Volume Percentage Delta (volume concentration at price extremes vs mid-range) to confirm Keltner band touches — a distinctly different confirmation mechanism than volume surge or Stochastic filters.

## Proposed Config Changes (config.rs)

```rust
// ── RUN410: Keltner Channel with Volume Percentage Delta Confirmation ─────────────────────────
// keltner_upper = EMA(close, period) + MULT * ATR(period)
// keltner_lower = EMA(close, period) - MULT * ATR(period)
// vol_pct_delta = (volume_at_extremes - volume_at_mid) / total_volume
// high_delta = vol_pct_delta > DELTA_THRESH (volume concentrated at extremes)
// band_touch: price crosses keltner_upper or keltner_lower
// LONG: price touches keltner_lower AND high_delta bullish
// SHORT: price touches keltner_upper AND high_delta bearish

pub const KC_VPD_ENABLED: bool = true;
pub const KC_VPD_KC_PERIOD: usize = 20;
pub const KC_VPD_ATR_PERIOD: usize = 14;
pub const KC_VPD_ATR_MULT: f64 = 2.0;
pub const KC_VPD_VOL_PERIOD: usize = 20;
pub const KC_VPD_DELTA_THRESH: f64 = 0.3;    // volume delta threshold
pub const KC_VPD_SL: f64 = 0.005;
pub const KC_VPD_TP: f64 = 0.004;
pub const KC_VPD_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run410_1_kc_vpd_backtest.py)
2. **Walk-forward** (run410_2_kc_vpd_wf.py)
3. **Combined** (run410_3_combined.py)

## Out-of-Sample Testing

- KC_PERIOD sweep: 15 / 20 / 30
- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- VOL_PERIOD sweep: 14 / 20 / 30
- DELTA_THRESH sweep: 0.2 / 0.3 / 0.4
