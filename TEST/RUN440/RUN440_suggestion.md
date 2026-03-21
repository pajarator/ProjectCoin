# RUN440 — DMI Smoothed Oscillator with Bollinger Band Touch Confirmation

## Hypothesis

**Mechanism**: The DMI Smoothed Oscillator (derived from ADX) measures trend direction and strength. When DMI+ crosses above DMI-, it indicates bullish trend, and vice versa. Bollinger Band Touch provides price structure confirmation: when DMI signals a trend AND price touches the Bollinger Band in the direction of the trend, the signal has both directional indicator AND price structure confirmation.

**Why not duplicate**: RUN376 uses DMI Smoothed Oscillator with Volume. This RUN uses Bollinger Band Touch instead of Volume — the distinct mechanism is using Bollinger Band position as the confirmation filter rather than volume relationships.

## Proposed Config Changes (config.rs)

```rust
// ── RUN440: DMI Smoothed Oscillator with Bollinger Band Touch Confirmation ─────────────────────────────────────
// dmi_plus = directional movement positive
// dmi_minus = directional movement negative
// dmi_cross: dmi_plus crosses above dmi_minus (bullish) or vice versa (bearish)
// adx = average directional index (trend strength)
// bb_touch: price touches bb_upper or bb_lower
// LONG: dmi_cross bullish AND adx > ADX_THRESH AND price touches bb_upper
// SHORT: dmi_cross bearish AND adx > ADX_THRESH AND price touches bb_lower

pub const DMI_BB_ENABLED: bool = true;
pub const DMI_BB_DMI_PERIOD: usize = 14;
pub const DMI_BB_ADX_PERIOD: usize = 14;
pub const DMI_BB_ADX_THRESH: f64 = 20.0;
pub const DMI_BB_BB_PERIOD: usize = 20;
pub const DMI_BB_BB_STD: f64 = 2.0;
pub const DMI_BB_SL: f64 = 0.005;
pub const DMI_BB_TP: f64 = 0.004;
pub const DMI_BB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run440_1_dmi_bb_backtest.py)
2. **Walk-forward** (run440_2_dmi_bb_wf.py)
3. **Combined** (run440_3_combined.py)

## Out-of-Sample Testing

- DMI_PERIOD sweep: 10 / 14 / 21
- ADX_PERIOD sweep: 10 / 14 / 21
- ADX_THRESH sweep: 15 / 20 / 25
- BB_PERIOD sweep: 15 / 20 / 30
