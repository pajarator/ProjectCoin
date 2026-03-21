# RUN376 — DMI Smoothed Oscillator with Volume

## Hypothesis

**Mechanism**: The DMI Oscillator (DMI + ADX combination) can be smoothed using a triple EMA to reduce noise. This RUN applies triple smoothing to the DMI difference (+DI - -DI) to create a very clean, low-noise directional oscillator. Volume confirmation: only take signals when volume is above average — this ensures the directional move has institutional backing.

**Why not duplicate**: No prior RUN uses triple-smoothed DMI. This RUN applies three layers of EMA smoothing to DMI, creating a distinctly noise-free directional oscillator. The distinct mechanism is the triple-smoothed DMI oscillator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN376: DMI Smoothed Oscillator with Volume ────────────────────────────────
// dmi_diff = +DI - -DI
// dmi_smooth1 = EMA(dmi_diff, SMOOTH1)
// dmi_smooth2 = EMA(dmi_smooth1, SMOOTH2)
// dmi_smooth3 = EMA(dmi_smooth2, SMOOTH3)
// osc_cross_up = dmi_smooth3 crosses above 0
// osc_cross_down = dmi_smooth3 crosses below 0
// volume_confirm = volume > avg_vol * VOL_MULT
// LONG: osc_cross_up AND volume_confirm
// SHORT: osc_cross_down AND volume_confirm

pub const DMI_SMOOTH_VOL_ENABLED: bool = true;
pub const DMI_SMOOTH_VOL_DI_PERIOD: usize = 14;
pub const DMI_SMOOTH_VOL_SMOOTH1: usize = 5;
pub const DMI_SMOOTH_VOL_SMOOTH2: usize = 5;
pub const DMI_SMOOTH_VOL_SMOOTH3: usize = 5;
pub const DMI_SMOOTH_VOL_VOL_MULT: f64 = 1.5;
pub const DMI_SMOOTH_VOL_SL: f64 = 0.005;
pub const DMI_SMOOTH_VOL_TP: f64 = 0.004;
pub const DMI_SMOOTH_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run376_1_dmi_smooth_vol_backtest.py)
2. **Walk-forward** (run376_2_dmi_smooth_vol_wf.py)
3. **Combined** (run376_3_combined.py)

## Out-of-Sample Testing

- SMOOTH1 sweep: 3 / 5 / 7
- SMOOTH2 sweep: 3 / 5 / 7
- SMOOTH3 sweep: 3 / 5 / 7
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
