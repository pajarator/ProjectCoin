# RUN452 — CCI with RSI Confluence

## Hypothesis

**Mechanism**: CCI (Commodity Channel Index) measures how far price deviates from its statistical mean. When CCI reaches extreme readings (above +100 or below -100), it indicates overbought/oversold conditions. RSI Confluence adds a second oscillator's confirmation: when CCI fires AND RSI also confirms in the same direction (RSI crossing its own signal line or reaching extreme), the reversal has dual-oscillator confirmation.

**Why not duplicate**: RUN349 uses CCI Percentile Rank. RUN397 uses CCI with Volume Divergence. RUN414 uses CCI with KST Confluence. This RUN specifically uses RSI as the confirming oscillator — the distinct mechanism is dual-oscillator confirmation using CCI and RSI together.

## Proposed Config Changes (config.rs)

```rust
// ── RUN452: CCI with RSI Confluence ─────────────────────────────────
// cci = (typical_price - SMA(typical_price, period)) / (0.015 * mean_deviation)
// cci_extreme: cci < CCI_OVERSOLD or cci > CCI_OVERBOUGHT
// rsi = relative_strength_index(close, period)
// rsi_signal: rsi crosses above/below RSI_THRESH
// LONG: cci < CCI_OVERSOLD AND rsi_signal bullish
// SHORT: cci > CCI_OVERBOUGHT AND rsi_signal bearish

pub const CCI_RSI_ENABLED: bool = true;
pub const CCI_RSI_CCI_PERIOD: usize = 14;
pub const CCI_RSI_CCI_OVERSOLD: f64 = -100.0;
pub const CCI_RSI_CCI_OVERBOUGHT: f64 = 100.0;
pub const CCI_RSI_RSI_PERIOD: usize = 14;
pub const CCI_RSI_RSI_THRESH: f64 = 50.0;
pub const CCI_RSI_SL: f64 = 0.005;
pub const CCI_RSI_TP: f64 = 0.004;
pub const CCI_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run452_1_cci_rsi_backtest.py)
2. **Walk-forward** (run452_2_cci_rsi_wf.py)
3. **Combined** (run452_3_combined.py)

## Out-of-Sample Testing

- CCI_PERIOD sweep: 10 / 14 / 21
- CCI_OVERSOLD sweep: -80 / -100 / -120
- CCI_OVERBOUGHT sweep: 80 / 100 / 120
- RSI_THRESH sweep: 45 / 50 / 55
