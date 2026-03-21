# RUN423 — Volume-Weighted Bollinger Bands with KST Confluence

## Hypothesis

**Mechanism**: Volume-Weighted Bollinger Bands use volume-weighted standard deviation instead of simple standard deviation, making the bands more responsive to volume-backed price moves. The bands expand when volatility increases and contract when it decreases. KST (Know Sure Thing) provides smoothed momentum confirmation. When price touches the volume-weighted bands AND KST crosses in the same direction, you have both volume-weighted price structure AND smoothed momentum confirmation.

**Why not duplicate**: RUN332 uses Volume-Weighted Bollinger Band Touch. RUN342 uses KST Percentile Rank. This RUN specifically uses VW Bollinger Bands (distinct calculation from standard BB) with KST crossover confirmation — the distinct mechanism is volume-weighted bands that respond to volume-backed volatility rather than standard price-based volatility.

## Proposed Config Changes (config.rs)

```rust
// ── RUN423: Volume-Weighted Bollinger Bands with KST Confluence ───────────────────────────────
// vw_bb_upper = vw_ma + VW_STD * ATR
// vw_bb_middle = vw_ma (volume-weighted moving average)
// vw_bb_lower = vw_ma - VW_STD * ATR
// band_touch: price touches or crosses vw_bb_upper or vw_bb_lower
// kst = know_sure_thing(roc1, roc2, roc3, roc4, signal)
// kst_cross: kst crosses above/below signal line
// LONG: price touches vw_bb_lower AND kst crosses bullish
// SHORT: price touches vw_bb_upper AND kst crosses bearish

pub const VWBB_KST_ENABLED: bool = true;
pub const VWBB_KST_VW_PERIOD: usize = 20;
pub const VWBB_KST_VW_STD: f64 = 2.0;
pub const VWBB_KST_KST_ROC1: usize = 10;
pub const VWBB_KST_KST_ROC2: usize = 15;
pub const VWBB_KST_KST_ROC3: usize = 20;
pub const VWBB_KST_KST_ROC4: usize = 30;
pub const VWBB_KST_KST_SIGNAL: usize = 9;
pub const VWBB_KST_SL: f64 = 0.005;
pub const VWBB_KST_TP: f64 = 0.004;
pub const VWBB_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run423_1_vwbb_kst_backtest.py)
2. **Walk-forward** (run423_2_vwbb_kst_wf.py)
3. **Combined** (run423_3_combined.py)

## Out-of-Sample Testing

- VW_PERIOD sweep: 14 / 20 / 30
- VW_STD sweep: 1.5 / 2.0 / 2.5
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
