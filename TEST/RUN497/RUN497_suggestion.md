# RUN497 — Volume Ratio Spike with DMI Direction

## Hypothesis

**Mechanism**: Volume Ratio Spike identifies unusual volume activity relative to its moving average, signaling potential institutional involvement. DMI (Directional Movement Index) provides trend direction via +DI and -DI. When Volume Ratio Spike occurs AND DMI shows clear directional bias, entries have both unusual volume conviction and clear trend direction.

**Why not duplicate**: RUN436 uses Volume Ratio Spike with KST Momentum Confirmation. This RUN uses DMI instead — distinct mechanism is DMI's directional trend confirmation versus KST's multi-ROC momentum. DMI specifically measures directional movement quality.

## Proposed Config Changes (config.rs)

```rust
// ── RUN497: Volume Ratio Spike with DMI Direction ─────────────────────────────────
// vol_ratio_spike: volume / sma(volume) ratio exceeding threshold
// dmi: directional_movement_index with +DI and -DI
// dmi_direction: +DI > -DI = bullish, -DI > +DI = bearish
// LONG: vol_ratio > 1.5 AND dmi_bullish (+DI > -DI)
// SHORT: vol_ratio > 1.5 AND dmi_bearish (-DI > +DI)

pub const VOLRATIO_DMI_ENABLED: bool = true;
pub const VOLRATIO_DMI_VOL_PERIOD: usize = 20;
pub const VOLRATIO_DMI_RATIO_THRESH: f64 = 1.5;
pub const VOLRATIO_DMI_DMI_PERIOD: usize = 14;
pub const VOLRATIO_DMI_SL: f64 = 0.005;
pub const VOLRATIO_DMI_TP: f64 = 0.004;
pub const VOLRATIO_DMI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run497_1_volratio_dmi_backtest.py)
2. **Walk-forward** (run497_2_volratio_dmi_wf.py)
3. **Combined** (run497_3_combined.py)

## Out-of-Sample Testing

- VOL_PERIOD sweep: 14 / 20 / 30
- RATIO_THRESH sweep: 1.3 / 1.5 / 2.0
- DMI_PERIOD sweep: 10 / 14 / 20
