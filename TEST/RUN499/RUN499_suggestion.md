# RUN499 — RSI Adaptive Period with KST Momentum

## Hypothesis

**Mechanism**: RSI Adaptive Period adjusts the RSI calculation period based on volatility — using shorter periods in volatile markets and longer periods in calm markets. This makes RSI more responsive when needed and smoother when markets are stable. KST Momentum Confirmation provides multi-timeframe momentum verification: when adaptive RSI fires AND KST confirms the same direction across multiple ROC periods, entries have both adaptive oscillator timing and multi-timeframe momentum conviction.

**Why not duplicate**: RUN412 uses Adaptive RSI with Volume Confirmation. This RUN uses KST instead — distinct mechanism is KST's multi-ROC momentum confirmation versus volume-based confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN499: RSI Adaptive Period with KST Momentum ─────────────────────────────────
// rsi_adaptive: rsi period adapts to volatility (shorter in volatile, longer in calm)
// rsi_cross: adaptive rsi crosses above/below signal or extreme levels
// kst: know_sure_thing momentum from multiple smoothed roc periods
// kst_cross: kst crosses above/below signal line
// LONG: rsi_adaptive crosses above 30 (from oversold) AND kst_cross bullish
// SHORT: rsi_adaptive crosses below 70 (from overbought) AND kst_cross bearish

pub const RSIA_KST_ENABLED: bool = true;
pub const RSIA_KST_RSI_BASE: usize = 14;
pub const RSIA_KST_RSI_VOL_PERIOD: usize = 20;
pub const RSIA_KST_RSI_OVERSOLD: f64 = 30.0;
pub const RSIA_KST_RSI_OVERBOUGHT: f64 = 70.0;
pub const RSIA_KST_KST_ROC1: usize = 10;
pub const RSIA_KST_KST_ROC2: usize = 15;
pub const RSIA_KST_KST_ROC3: usize = 20;
pub const RSIA_KST_KST_ROC4: usize = 30;
pub const RSIA_KST_KST_SIGNAL: usize = 9;
pub const RSIA_KST_SL: f64 = 0.005;
pub const RSIA_KST_TP: f64 = 0.004;
pub const RSIA_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run499_1_rs ia_kst_backtest.py)
2. **Walk-forward** (run499_2_rs ia_kst_wf.py)
3. **Combined** (run499_3_combined.py)

## Out-of-Sample Testing

- RSI_BASE sweep: 10 / 14 / 20
- RSI_VOL_PERIOD sweep: 14 / 20 / 30
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
