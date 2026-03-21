# RUN390 — Money Flow Index with VWAP Mean Reversion Distance

## Hypothesis

**Mechanism**: Money Flow Index (MFI) is a volume-weighted RSI that measures buying and selling pressure. When MFI reaches extreme levels (above 80 = overbought, below 20 = oversold), it often signals reversals. However, not all MFI extremes lead to reversals — the market can stay overbought/oversold for extended periods. Add VWAP distance as a filter: when MFI fires AND price hasdeviated significantly from VWAP (creating a mean reversion opportunity), the signal has both momentum extreme AND price distance confirmation. Strong deviations from VWAP tend to revert, and MFI confirms the turning point.

**Why not duplicate**: RUN320 uses Volume-Weighted RSI Multi-Period. RUN303 uses MFI Percentile Rank. This RUN specifically uses MFI extremes combined with VWAP distance measurement — the distinct mechanism is using VWAP deviation distance to distinguish between "overbought that stays overbought" and "overbought that's ready to revert."

## Proposed Config Changes (config.rs)

```rust
// ── RUN390: Money Flow Index with VWAP Mean Reversion Distance ──────────────────
// mfi = 100 - (100 / (1 + money_flow_ratio))
// vwap_distance = (price - vwap) / vwap  (deviation from fair value)
// mean_reversion_trigger: |vwap_distance| > Vwap_Distance_THRESH
// LONG: mfi < MFI_OVERSOLD AND vwap_distance < -VWAP_DIST_THRESH (price below VWAP)
// SHORT: mfi > MFI_OVERBOUGHT AND vwap_distance > VWAP_DIST_THRESH (price above VWAP)

pub const MFI_VWAP_ENABLED: bool = true;
pub const MFI_VWAP_MFI_PERIOD: usize = 14;
pub const MFI_VWAP_MFI_OVERSOLD: f64 = 20.0;
pub const MFI_VWAP_MFI_OVERBOUGHT: f64 = 80.0;
pub const MFI_VWAP_VWAP_DIST_THRESH: f64 = 0.01;  // 1% deviation from VWAP
pub const MFI_VWAP_SL: f64 = 0.005;
pub const MFI_VWAP_TP: f64 = 0.004;
pub const MFI_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run390_1_mfi_vwap_backtest.py)
2. **Walk-forward** (run390_2_mfi_vwap_wf.py)
3. **Combined** (run390_3_combined.py)

## Out-of-Sample Testing

- MFI_PERIOD sweep: 10 / 14 / 21
- MFI_OVERSOLD sweep: 15 / 20 / 25
- MFI_OVERBOUGHT sweep: 75 / 80 / 85
- VWAP_DIST_THRESH sweep: 0.005 / 0.01 / 0.015
