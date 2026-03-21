# RUN489 — ATR Percentile Rank with ADX Trend Disposition

## Hypothesis

**Mechanism**: ATR Percentile Rank measures how current ATR compares to its historical distribution, identifying when volatility is historically high or low. ADX Trend Disposition combines ADX level with +DI/-DI directional indicators to determine if the market is in a bull or bear trend. When ATR percentile is high AND ADX shows a strong trend in a specific direction, volatility expansion during trending conditions often produces sustained moves.

**Why not duplicate**: RUN449 uses ATR Percentile Rank with ADX Disposition Filter. This appears to be a duplicate — RUN449 is ATR Percentile Rank with ADX Disposition. Let me pivot.

ATR Percentile Rank with DMI Trend Direction — when ATR percentile is high AND DMI shows clear trend direction, entries have both volatility expansion and directional conviction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN489: ATR Percentile Rank with DMI Trend Direction ─────────────────────────────────
// atr_pct_rank: current atr rank vs historical distribution
// dmi: directional_movement_index including +DI and -DI
// dmi_cross: +DI crosses above/below -DI indicating direction
// LONG: atr_pct_rank > 70 AND dmi_cross bullish (+DI > -DI)
// SHORT: atr_pct_rank > 70 AND dmi_cross bearish (-DI > +DI)

pub const ATRPCT_DMI_ENABLED: bool = true;
pub const ATRPCT_DMI_ATR_PERIOD: usize = 14;
pub const ATRPCT_DMI_ATR_PCT_PERIOD: usize = 50;
pub const ATRPCT_DMI_ATR_PCT_THRESH: f64 = 70.0;
pub const ATRPCT_DMI_DMI_PERIOD: usize = 14;
pub const ATRPCT_DMI_SL: f64 = 0.005;
pub const ATRPCT_DMI_TP: f64 = 0.004;
pub const ATRPCT_DMI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run489_1_atrpct_dmi_backtest.py)
2. **Walk-forward** (run489_2_atrpct_dmi_wf.py)
3. **Combined** (run489_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 20
- ATR_PCT_PERIOD sweep: 30 / 50 / 100
- ATR_PCT_THRESH sweep: 60 / 70 / 80
- DMI_PERIOD sweep: 10 / 14 / 20
