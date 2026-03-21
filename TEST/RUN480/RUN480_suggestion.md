# RUN480 — Trend Resonance Factor with Stochastic RSI

## Hypothesis

**Mechanism**: Trend Resonance Factor (TRF) measures how multiple trend indicators align, producing a resonance score when moving averages, ADX, and price action all agree on direction. Stochastic RSI applies Stochastic oscillator to RSI values rather than price, providing more sensitive overbought/oversold readings. When TRF shows strong resonance AND Stochastic RSI confirms momentum direction, entries have both multi-indicator alignment and sensitive oscillator timing.

**Why not duplicate**: RUN416 uses Trend Resonance Factor with Williams %R Extreme. This RUN uses Stochastic RSI instead — distinct mechanism is Stochastic RSI's oscillator sensitivity versus Williams %R's momentum extremes. Stochastic RSI is smoother and less erratic than Williams %R.

## Proposed Config Changes (config.rs)

```rust
// ── RUN480: Trend Resonance Factor with Stochastic RSI ─────────────────────────────────
// trf: trend_resonance_factor combining multiple trend indicators
// trf_cross: trf crosses above/below signal threshold
// stoch_rsi: stochastic applied to rsi values for sensitivity
// stoch_rsi_cross: stoch_rsi crosses above/below signal line
// LONG: trf > 0.6 AND stoch_rsi_cross bullish
// SHORT: trf < -0.6 AND stoch_rsi_cross bearish

pub const TRF_STOCHRSI_ENABLED: bool = true;
pub const TRF_STOCHRSI_TRF_PERIOD: usize = 20;
pub const TRF_STOCHRSI_TRF_THRESH: f64 = 0.6;
pub const TRF_STOCHRSI_RSI_PERIOD: usize = 14;
pub const TRF_STOCHRSI_STOCH_PERIOD: usize = 14;
pub const TRF_STOCHRSI_STOCH_K: usize = 3;
pub const TRF_STOCHRSI_STOCH_D: usize = 3;
pub const TRF_STOCHRSI_SL: f64 = 0.005;
pub const TRF_STOCHRSI_TP: f64 = 0.004;
pub const TRF_STOCHRSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run480_1_trf_stochrsi_backtest.py)
2. **Walk-forward** (run480_2_trf_stochrsi_wf.py)
3. **Combined** (run480_3_combined.py)

## Out-of-Sample Testing

- TRF_PERIOD sweep: 14 / 20 / 30
- TRF_THRESH sweep: 0.5 / 0.6 / 0.7
- RSI_PERIOD sweep: 10 / 14 / 20
- STOCH_PERIOD sweep: 10 / 14 / 20
- STOCH_K sweep: 2 / 3 / 5
