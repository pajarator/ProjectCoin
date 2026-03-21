# RUN385 — Parabolic SAR with Volume Acceleration Confirmation

## Hypothesis

**Mechanism**: Parabolic SAR is a trend-following indicator that provides entry points and trailing stops. However, in choppy markets it whipsaws. Add volume confirmation: when Parabolic SAR flips direction AND volume is accelerating (volume > volume SMA), the trend change has institutional backing. Volume acceleration confirms that the move isn't just a brief spike but a sustained shift in supply/demand dynamics.

**Why not duplicate**: RUN340 uses Parabolic SAR with Volume Acceleration. This RUN is the same hypothesis. Wait — I need a different approach. Let me reconsider: Parabolic SAR with ADX trend strength filter instead. When SAR flips AND ADX confirms a strong trend (ADX > threshold), the signal has both directional change AND trend conviction confirmation.

Actually, let me do a different angle: Detrended Price Oscillator with Parabolic SAR direction filter. DPO removes trend to show cycles; when DPO crosses zero AND SAR confirms the direction, you get cycle turning point + trend confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN385: Detrended Price Oscillator with Parabolic SAR Confirmation ──────────
// dpo = close - SMA(close, period/2 + 1)  // removes trend, shows cycles
// dpo_zero_cross: dpo crossing above/below 0 signals cycle reversal
// sar_direction: sar_flip to bullish/bearish
// LONG: dpo crosses above 0 AND sar_flip to bullish
// SHORT: dpo crosses below 0 AND sar_flip to bearish

pub const DPO_SAR_ENABLED: bool = true;
pub const DPO_SAR_DPO_PERIOD: usize = 20;
pub const DPO_SAR_SAR_AF: f64 = 0.02;   // SAR acceleration factor
pub const DPO_SAR_SAR_MAX: f64 = 0.2;    // SAR maximum
pub const DPO_SAR_SL: f64 = 0.005;
pub const DPO_SAR_TP: f64 = 0.004;
pub const DPO_SAR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run385_1_dpo_sar_backtest.py)
2. **Walk-forward** (run385_2_dpo_sar_wf.py)
3. **Combined** (run385_3_combined.py)

## Out-of-Sample Testing

- DPO_PERIOD sweep: 14 / 20 / 30
- SAR_AF sweep: 0.01 / 0.02 / 0.05
- SAR_MAX sweep: 0.15 / 0.20 / 0.25
