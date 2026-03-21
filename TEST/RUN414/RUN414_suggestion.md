# RUN414 — Commodity Channel Index with KST Momentum Confluence

## Hypothesis

**Mechanism**: CCI measures how far price deviates from its statistical mean, with extreme readings (above +100 or below -100) indicating overbought/oversold conditions. KST (Know Sure Thing) is a smoothed momentum oscillator that combines multiple rate-of-change measurements. When CCI fires an extreme signal AND KST confirms the same directional momentum, you have both statistical deviation and smoothed momentum confirmation working together — reducing false signals from CCI alone.

**Why not duplicate**: RUN349 uses CCI Percentile Rank. RUN397 uses CCI with Volume Divergence. This RUN specifically uses CCI with KST confluence — KST provides a distinctly different smoothing approach (multi-period ROC weighted) compared to volume divergence or percentile ranking.

## Proposed Config Changes (config.rs)

```rust
// ── RUN414: Commodity Channel Index with KST Momentum Confluence ─────────────────────────────
// cci = (typical_price - SMA(typical_price, period)) / (0.015 * mean_deviation)
// cci_extreme: cci < CCI_OVERSOLD or cci > CCI_OVERBOUGHT
// kst = weighted sum of multiple ROC smoothed signals
// kst_cross: kst crosses above/below signal line
// LONG: cci < CCI_OVERSOLD AND kst crosses bullish
// SHORT: cci > CCI_OVERBOUGHT AND kst crosses bearish

pub const CCI_KST_ENABLED: bool = true;
pub const CCI_KST_CCI_PERIOD: usize = 14;
pub const CCI_KST_CCI_OVERSOLD: f64 = -100.0;
pub const CCI_KST_CCI_OVERBOUGHT: f64 = 100.0;
pub const CCI_KST_KST_ROC1: usize = 10;
pub const CCI_KST_KST_ROC2: usize = 15;
pub const CCI_KST_KST_ROC3: usize = 20;
pub const CCI_KST_KST_ROC4: usize = 30;
pub const CCI_KST_KST_SIGNAL: usize = 9;
pub const CCI_KST_SL: f64 = 0.005;
pub const CCI_KST_TP: f64 = 0.004;
pub const CCI_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run414_1_cci_kst_backtest.py)
2. **Walk-forward** (run414_2_cci_kst_wf.py)
3. **Combined** (run414_3_combined.py)

## Out-of-Sample Testing

- CCI_PERIOD sweep: 10 / 14 / 21
- CCI_OVERSOLD sweep: -80 / -100 / -120
- CCI_OVERBOUGHT sweep: 80 / 100 / 120
- KST_ROC1 sweep: 8 / 10 / 12
