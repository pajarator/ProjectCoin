# RUN463 — Volume-Weighted RSI with KST Momentum Confirmation

## Hypothesis

**Mechanism**: Volume-Weighted RSI (VWRSI) combines RSI with volume weighting, making it more responsive to volume-backed price moves than standard RSI. Standard RSI can give signals in either direction regardless of volume conviction. KST Momentum Confirmation adds multi-timeframe momentum verification: when VWRSI fires AND KST confirms the momentum direction across multiple ROC periods, the signal has both volume-weighted and multi-timeframe corroboration.

**Why not duplicate**: RUN445 uses Volume-Weighted RSI with EMA Trend Alignment. This RUN uses KST instead — the distinct mechanism is KST's multi-timeframe momentum confirmation versus EMA's trend direction alignment. KST provides smoother, multi-ROC momentum signals versus single EMA direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN463: Volume-Weighted RSI with KST Momentum Confirmation ─────────────────────────────────
// vwrsi: volume-weighted rsi combining price_change magnitude with volume
// vwrsi_cross: vwrsi crosses above/below signal line
// kst: know_sure_thing momentum from multiple roc smoothed periods
// kst_cross: kst crosses above/below signal line
// LONG: vwrsi_cross bullish AND kst_cross bullish
// SHORT: vwrsi_cross bearish AND kst_cross bearish

pub const VWRSI_KST_ENABLED: bool = true;
pub const VWRSI_KST_RSI_PERIOD: usize = 14;
pub const VWRSI_KST_RSI_VOL_PERIOD: usize = 20;
pub const VWRSI_KST_KST_ROC1: usize = 10;
pub const VWRSI_KST_KST_ROC2: usize = 15;
pub const VWRSI_KST_KST_ROC3: usize = 20;
pub const VWRSI_KST_KST_ROC4: usize = 30;
pub const VWRSI_KST_KST_SIGNAL: usize = 9;
pub const VWRSI_KST_SL: f64 = 0.005;
pub const VWRSI_KST_TP: f64 = 0.004;
pub const VWRSI_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run463_1_vwrsi_kst_backtest.py)
2. **Walk-forward** (run463_2_vwrsi_kst_wf.py)
3. **Combined** (run463_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 20
- RSI_VOL_PERIOD sweep: 14 / 20 / 30
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
