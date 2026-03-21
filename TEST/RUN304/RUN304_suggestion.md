# RUN304 — True Strength Index Crossover: Double-Smoothed Momentum Signal

## Hypothesis

**Mechanism**: TSI = double smoothed momentum (EMA of EMA of rate-of-change). The signal line = EMA of TSI. TSI crossing above its signal line = momentum shifting bullish. TSI crossing below = momentum shifting bearish. TSI is smoother than MACD because it applies two smoothing layers, reducing false signals. Use TSI histogram (TSI - signal) for early divergence detection.

**Why not duplicate**: No prior RUN uses TSI. MACD variants are RUN203 (volume-weighted), RUN249 (histogram ROC), RUN277 (histogram slope), RUN289 (zero line rejection). TSI is distinct because of the double smoothing — it's less responsive than MACD but more reliable for trend shifts.

## Proposed Config Changes (config.rs)

```rust
// ── RUN304: True Strength Index Crossover ────────────────────────────────────
// tsi = EMA(EMA(momentum(double_smooth), long_period), short_period) / price * 100
// signal_line = EMA(tsi, signal_period)
// LONG: tsi crosses above signal_line
// SHORT: tsi crosses below signal_line
// Optional: TSI histogram divergence for exit

pub const TSI_ENABLED: bool = true;
pub const TSI_LONG_EMA: usize = 25;         // long EMA period
pub const TSI_SHORT_EMA: usize = 13;        // short EMA period (applied twice)
pub const TSI_SIGNAL: usize = 13;           // signal line EMA period
pub const TSI_SL: f64 = 0.005;
pub const TSI_TP: f64 = 0.004;
pub const TSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run304_1_tsi_backtest.py)
2. **Walk-forward** (run304_2_tsi_wf.py)
3. **Combined** (run304_3_combined.py)

## Out-of-Sample Testing

- LONG_EMA sweep: 20 / 25 / 34
- SHORT_EMA sweep: 8 / 13 / 21
- SIGNAL sweep: 8 / 13 / 21
