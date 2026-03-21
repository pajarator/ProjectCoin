# RUN436 — Volume Ratio Spike with KST Momentum Confirmation

## Hypothesis

**Mechanism**: Volume Ratio Spike identifies when volume surges above a threshold relative to the average volume at the same time of day (session-adaptive volume). KST (Know Sure Thing) provides smoothed momentum confirmation. When Volume Ratio Spikes AND KST crosses in the same direction, the signal has both unusual volume (institutional activity) AND smoothed momentum alignment. This catches institutional moves early with momentum confirmation.

**Why not duplicate**: RUN372 uses Volume Ratio Spike with RSI. This RUN uses KST instead of RSI — the distinct mechanism is KST's multi-period smoothed momentum confirmation (vs RSI single-period oscillator), which provides better signal quality through its multiple ROC smoothing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN436: Volume Ratio Spike with KST Momentum Confirmation ─────────────────────────────────────
// volume_ratio = current_volume / SMA(volume at same session hour)
// volume_spike: volume_ratio > VOL_RATIO_THRESH
// kst = know_sure_thing(roc1, roc2, roc3, roc4, signal)
// kst_cross: kst crosses above/below signal line
// LONG: volume_spike AND kst_cross bullish
// SHORT: volume_spike AND kst_cross bearish

pub const VOL_RATIO_KST_ENABLED: bool = true;
pub const VOL_RATIO_KST_RATIO_PERIOD: usize = 20;
pub const VOL_RATIO_KST_RATIO_THRESH: f64 = 2.0;   // 2x average volume
pub const VOL_RATIO_KST_KST_ROC1: usize = 10;
pub const VOL_RATIO_KST_KST_ROC2: usize = 15;
pub const VOL_RATIO_KST_KST_ROC3: usize = 20;
pub const VOL_RATIO_KST_KST_ROC4: usize = 30;
pub const VOL_RATIO_KST_KST_SIGNAL: usize = 9;
pub const VOL_RATIO_KST_SL: f64 = 0.005;
pub const VOL_RATIO_KST_TP: f64 = 0.004;
pub const VOL_RATIO_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run436_1_vol_ratio_kst_backtest.py)
2. **Walk-forward** (run436_2_vol_ratio_kst_wf.py)
3. **Combined** (run436_3_combined.py)

## Out-of-Sample Testing

- RATIO_PERIOD sweep: 14 / 20 / 30
- RATIO_THRESH sweep: 1.5 / 2.0 / 2.5
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
