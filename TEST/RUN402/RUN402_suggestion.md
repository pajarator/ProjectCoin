# RUN402 — Momentum Acceleration with KST Confluence

## Hypothesis

**Mechanism**: Momentum Acceleration measures the rate of change of momentum itself — whether the rate of price change is increasing or decreasing. KST (Know Sure Thing) is a smoothed momentum oscillator based on multiple ROC smoothing periods. When Momentum Acceleration fires (the acceleration of momentum changes direction) AND KST confirms the same directional move, you have dual-momentum confirmation: both the raw momentum acceleration AND the smoothed KST are aligned. This redundancy filtering reduces false signals.

**Why not duplicate**: RUN331 uses Momentum Acceleration Divergence. RUN342 uses KST Percentile Rank. This RUN specifically combines Momentum Acceleration (second derivative of price) with KST (multi-smoothing ROC) as a confluence pair — the distinct mechanism is using acceleration of momentum as the entry trigger with KST as a smoothed confirmation filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN402: Momentum Acceleration with KST Confluence ─────────────────────────────────
// momentum_accel = ROC(momentum, period) - SMA(ROC(momentum, period), period)
// kst = weighted sum of multiple ROC smoothed signals
// accel_flip: momentum_accel crosses above/below 0
// kst_cross: kst crosses above/below signal line
// LONG: momentum_accel flips bullish AND kst crosses bullish
// SHORT: momentum_accel flips bearish AND kst crosses bearish

pub const MOMACC_KST_ENABLED: bool = true;
pub const MOMACC_KST_MOM_PERIOD: usize = 10;
pub const MOMACC_KST_ACCEL_PERIOD: usize = 5;
pub const MOMACC_KST_KST_ROC1: usize = 10;
pub const MOMACC_KST_KST_ROC2: usize = 15;
pub const MOMACC_KST_KST_ROC3: usize = 20;
pub const MOMACC_KST_KST_ROC4: usize = 30;
pub const MOMACC_KST_SL: f64 = 0.005;
pub const MOMACC_KST_TP: f64 = 0.004;
pub const MOMACC_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run402_1_momacc_kst_backtest.py)
2. **Walk-forward** (run402_2_momacc_kst_wf.py)
3. **Combined** (run402_3_combined.py)

## Out-of-Sample Testing

- MOM_PERIOD sweep: 8 / 10 / 14
- ACCEL_PERIOD sweep: 3 / 5 / 7
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
