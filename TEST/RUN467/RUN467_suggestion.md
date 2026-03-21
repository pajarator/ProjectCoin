# RUN467 — Mass Index with KST Momentum Confirmation

## Hypothesis

**Mechanism**: Mass Index analyzes the narrowing/widening of the high-low range to identify trend exhaustion and reversals. A Mass Index reading above 27 then dropping below 26 historically signals trend reversal. KST Momentum Confirmation verifies the momentum direction: when Mass Index reverses AND KST also signals momentum shift in the same direction, the reversal has both range-based and momentum-based conviction.

**Why not duplicate**: RUN383 uses Mass Index with Aroon Oscillator Trend Exhaustion. This RUN uses KST instead — the distinct mechanism is KST's multi-timeframe smoothed momentum confirmation versus Aroon's trend exhaustion timing. KST provides different momentum confirmation at different ROC timescales.

## Proposed Config Changes (config.rs)

```rust
// ── RUN467: Mass Index with KST Momentum Confirmation ─────────────────────────────────
// mass_index: sum of ema_ratios over period, reversal on drop from >27 to <26
// mass_reversal: mass_index crosses below 26 after being > 27
// kst: know_sure_thing momentum from multiple roc periods
// kst_cross: kst crosses above/below signal line
// LONG: mass_reversal bullish (drop from high) AND kst_cross bullish
// SHORT: mass_reversal bearish (drop from high) AND kst_cross bearish

pub const MASS_KST_ENABLED: bool = true;
pub const MASS_KST_MASS_FAST: usize = 9;
pub const MASS_KST_MASS_SLOW: usize = 25;
pub const MASS_KST_MASS_THRESH_HIGH: f64 = 27.0;
pub const MASS_KST_MASS_THRESH_LOW: f64 = 26.5;
pub const MASS_KST_KST_ROC1: usize = 10;
pub const MASS_KST_KST_ROC2: usize = 15;
pub const MASS_KST_KST_ROC3: usize = 20;
pub const MASS_KST_KST_ROC4: usize = 30;
pub const MASS_KST_KST_SIGNAL: usize = 9;
pub const MASS_KST_SL: f64 = 0.005;
pub const MASS_KST_TP: f64 = 0.004;
pub const MASS_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run467_1_mass_kst_backtest.py)
2. **Walk-forward** (run467_2_mass_kst_wf.py)
3. **Combined** (run467_3_combined.py)

## Out-of-Sample Testing

- MASS_FAST sweep: 7 / 9 / 12
- MASS_SLOW sweep: 20 / 25 / 30
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
