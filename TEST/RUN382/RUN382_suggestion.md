# RUN382 — Chande Momentum Oscillator with VWAP Trend Confirmation

## Hypothesis

**Mechanism**: The Chande Momentum Oscillator (CMO) is a normalized momentum indicator that measures the difference between sum of recent gains and sum of recent losses, divided by the sum of all price movement. Unlike RSI (which uses internal smoothing), CMO is more responsive to genuine momentum shifts. Pair it with VWAP as a trend confirmation filter: VWAP represents the volume-weighted average price, acting as a real-time fair-value line. When CMO fires a signal AND price is on the correct side of VWAP, you get both momentum AND institutional price-alignment confirmation.

**Why not duplicate**: RUN364 uses VWAP Deviation Percentile with Trend Mode. RUN369 uses Ichimoku with Volume Confirmation. This RUN specifically uses CMO (a distinct momentum calculation) with VWAP as a binary trend filter (above/below VWAP), not as a deviation/percentile measure.

## Proposed Config Changes (config.rs)

```rust
// ── RUN382: Chande Momentum Oscillator with VWAP Trend Confirmation ───────────
// cmo = (sum_gains - sum_losses) / (sum_gains + sum_losses) * 100
// vwap = cumulative(price * volume) / cumulative(volume)
// LONG: cmo crosses above CMO_OVERSOLD AND close > vwap
// SHORT: cmo crosses below CMO_OVERBOUGHT AND close < vwap

pub const CMO_VWAP_ENABLED: bool = true;
pub const CMO_VWAP_CMO_PERIOD: usize = 14;
pub const CMO_VWAP_CMO_OVERSOLD: f64 = -50.0;
pub const CMO_VWAP_CMO_OVERBOUGHT: f64 = 50.0;
pub const CMO_VWAP_SL: f64 = 0.005;
pub const CMO_VWAP_TP: f64 = 0.004;
pub const CMO_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run382_1_cmo_vwap_backtest.py)
2. **Walk-forward** (run382_2_cmo_vwap_wf.py)
3. **Combined** (run382_3_combined.py)

## Out-of-Sample Testing

- CMO_PERIOD sweep: 10 / 14 / 21
- CMO_OVERSOLD sweep: -40 / -50 / -60
- CMO_OVERBOUGHT sweep: 40 / 50 / 60
