# RUN457 — VWMA with Bollinger Band Touch Confirmation

## Hypothesis

**Mechanism**: Volume-Weighted Moving Average (VWMA) weights price by volume, making it more responsive to volume-backed price moves than standard MAs. Bollinger Band Touch provides price structure confirmation: when VWMA crosses the Bollinger Band boundary AND the VWMA is touching the band, the move has both volume-weighted trend direction AND price structure confirmation.

**Why not duplicate**: RUN332 uses Volume-Weighted Bollinger Band Touch. This RUN uses VWMA (the same as VWMABT essentially). Let me reconsider. VWMA with RSI? VWMA with ATR volatility filter?

VWMA with KST Momentum Confirmation — when VWMA crosses the BB AND KST confirms momentum in the same direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN457: VWMA with KST Momentum Confirmation ─────────────────────────────────
// vwma = volume-weighted moving average
// bb_touch: price/VWMA crosses bb_upper or bb_lower
// kst = know_sure_thing(roc1, roc2, roc3, roc4, signal)
// kst_cross: kst crosses above/below signal line
// LONG: vwma crosses above bb_upper AND kst_cross bullish
// SHORT: vwma crosses below bb_lower AND kst_cross bearish

pub const VWMA_KST_ENABLED: bool = true;
pub const VWMA_KST_VWMA_PERIOD: usize = 20;
pub const VWMA_KST_BB_PERIOD: usize = 20;
pub const VWMA_KST_BB_STD: f64 = 2.0;
pub const VWMA_KST_KST_ROC1: usize = 10;
pub const VWMA_KST_KST_ROC2: usize = 15;
pub const VWMA_KST_KST_ROC3: usize = 20;
pub const VWMA_KST_KST_ROC4: usize = 30;
pub const VWMA_KST_KST_SIGNAL: usize = 9;
pub const VWMA_KST_SL: f64 = 0.005;
pub const VWMA_KST_TP: f64 = 0.004;
pub const VWMA_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run457_1_vwma_kst_backtest.py)
2. **Walk-forward** (run457_2_vwma_kst_wf.py)
3. **Combined** (run457_3_combined.py)

## Out-of-Sample Testing

- VWMA_PERIOD sweep: 14 / 20 / 30
- BB_PERIOD sweep: 15 / 20 / 30
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
