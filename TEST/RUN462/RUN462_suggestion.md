# RUN462 — Demand Index with RSI Divergence

## Hypothesis

**Mechanism**: Demand Index (DI) combines price and volume to identify major reversals in price trends. DI peaks before price peaks, making it a leading indicator. RSI Divergence confirms the momentum reversal: when DI makes a divergence with price AND RSI confirms the divergence direction, the signal has both volume-weighted conviction and oscillator confirmation.

**Why not duplicate**: RUN422 uses Demand Index with SuperTrend. This RUN uses RSI Divergence instead — the distinct mechanism is using RSI as the confirmation oscillator rather than SuperTrend, targeting momentum reversal timing with a classic oscillator divergence pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN462: Demand Index with RSI Divergence ─────────────────────────────────
// demand_index: di_combines price_volume_to measure buying_selling_pressure
// di_cross: di crosses above/below signal line
// rsi_divergence: price makes higher_high but rsi makes lower_high (bearish)
// LONG: di_cross bullish AND price_low < prev_price_low AND rsi_low > prev_rsi_low
// SHORT: di_cross bearish AND price_high > prev_price_high AND rsi_high < prev_rsi_high

pub const DI_RSIDIV_ENABLED: bool = true;
pub const DI_RSIDIV_DI_PERIOD: usize = 20;
pub const DI_RSIDIV_DI_SIGNAL: usize = 9;
pub const DI_RSIDIV_RSI_PERIOD: usize = 14;
pub const DI_RSIDIV_RSI_OVERSOLD: f64 = 35.0;
pub const DI_RSIDIV_RSI_OVERBOUGHT: f64 = 65.0;
pub const DI_RSIDIV_SL: f64 = 0.005;
pub const DI_RSIDIV_TP: f64 = 0.004;
pub const DI_RSIDIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run462_1_di_rsidiv_backtest.py)
2. **Walk-forward** (run462_2_di_rsidiv_wf.py)
3. **Combined** (run462_3_combined.py)

## Out-of-Sample Testing

- DI_PERIOD sweep: 14 / 20 / 30
- DI_SIGNAL sweep: 7 / 9 / 12
- RSI_PERIOD sweep: 10 / 14 / 20
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
