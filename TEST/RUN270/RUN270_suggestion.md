# RUN270 — Volume-Weighted Stochastic: Volume-Adjusted Overbought/Oversold

## Hypothesis

**Mechanism**: Standard Stochastic compares close to the high-low range. Volume-Weighted Stochastic weights each bar's contribution by its volume — high-volume bars have more influence on the %K and %D. This makes the indicator more responsive to volume-confirmed price moves.

**Why not duplicate**: No prior RUN uses Volume-Weighted Stochastic. All prior Stochastic RUNs use standard Stochastic. VW Stochastic is distinct because it incorporates volume into the stochastic calculation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN270: Volume-Weighted Stochastic ─────────────────────────────────────
// For each bar: weight = volume / avg_volume
// highest_weighted = highest weighted close over period
// lowest_weighted = lowest weighted close over period
// %K = (close - lowest_weighted) / (highest_weighted - lowest_weighted) × 100
// LONG: %K crosses above 20 AND %D < 50
// SHORT: %K crosses below 80 AND %D > 50

pub const VW_STOCH_ENABLED: bool = true;
pub const VW_STOCH_PERIOD: usize = 14;
pub const VW_STOCH_SIGNAL: usize = 3;
pub const VW_STOCH_OVERSOLD: f64 = 20.0;
pub const VW_STOCH_OVERBOUGHT: f64 = 80.0;
pub const VW_STOCH_SL: f64 = 0.005;
pub const VW_STOCH_TP: f64 = 0.004;
pub const VW_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run270_1_vw_stoch_backtest.py)
2. **Walk-forward** (run270_2_vw_stoch_wf.py)
3. **Combined** (run270_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- SIGNAL sweep: 3 / 5 / 7
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
