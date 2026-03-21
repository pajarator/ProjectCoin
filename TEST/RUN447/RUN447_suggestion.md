# RUN447 — Stochastic RSI with VWAP Deviation Filter

## Hypothesis

**Mechanism**: Stochastic RSI applies the Stochastic oscillator to the RSI indicator, creating an indicator of an indicator. It identifies when RSI is at extreme levels relative to its own range. VWAP Deviation Filter adds a price context filter: only take Stochastic RSI signals when price is significantly deviated from VWAP. When Stochastic RSI triggers AND price has deviated from VWAP, the signal has both indicator-extreme AND price-distance confirmation.

**Why not duplicate**: RUN324 uses Stochastic RSI Divergence with Volume. This RUN uses VWAP Deviation Filter instead of Volume — the distinct mechanism is using VWAP deviation distance to confirm Stochastic RSI extremes, filtering out extremes that occur near VWAP.

## Proposed Config Changes (config.rs)

```rust
// ── RUN447: Stochastic RSI with VWAP Deviation Filter ─────────────────────────────────────
// stoch_rsi = stochastic_oscillator(RSI_values, period)
// stoch_rsi_extreme: %K < STOCH_RSI_OVERSOLD or > STOCH_RSI_OVERBOUGHT
// vwap_deviation = |close - vwap| / vwap
// high_deviation: vwap_deviation > DEV_THRESH
// LONG: stoch_rsi < STOCH_RSI_OVERSOLD AND high_deviation
// SHORT: stoch_rsi > STOCH_RSI_OVERBOUGHT AND high_deviation

pub const STOCH_RSI_VWAP_ENABLED: bool = true;
pub const STOCH_RSI_VWAP_RSI_PERIOD: usize = 14;
pub const STOCH_RSI_VWAP_STOCH_PERIOD: usize = 14;
pub const STOCH_RSI_VWAP_STOCH_RSI_OVERSOLD: f64 = 20.0;
pub const STOCH_RSI_VWAP_STOCH_RSI_OVERBOUGHT: f64 = 80.0;
pub const STOCH_RSI_VWAP_VWAP_PERIOD: usize = 20;
pub const STOCH_RSI_VWAP_DEV_THRESH: f64 = 0.01;
pub const STOCH_RSI_VWAP_SL: f64 = 0.005;
pub const STOCH_RSI_VWAP_TP: f64 = 0.004;
pub const STOCH_RSI_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run447_1_stoch_rsi_vwap_backtest.py)
2. **Walk-forward** (run447_2_stoch_rsi_vwap_wf.py)
3. **Combined** (run447_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- STOCH_PERIOD sweep: 10 / 14 / 21
- STOCH_RSI_OVERSOLD sweep: 15 / 20 / 25
- STOCH_RSI_OVERBOUGHT sweep: 75 / 80 / 85
- DEV_THRESH sweep: 0.005 / 0.01 / 0.015
