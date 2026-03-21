# RUN401 — VWAP Standard Deviation Bands with RSI Momentum Crossover

## Hypothesis

**Mechanism**: VWAP Standard Deviation Bands are plotted at multiples of standard deviations around the VWAP line, creating channels that represent statistically significant price deviations. When price touches or crosses the outer bands, it's in extreme territory relative to the volume-weighted average. RSI Momentum Crossover provides confirmation: RSI crosses its own signal line at the same time price hits the outer band. This combination gets VWAP's volume-weighted extreme reading AND RSI's momentum confirmation simultaneously.

**Why not duplicate**: RUN326 uses Z-Score Distance from VWAP. RUN343 uses VWAP Deviation Percentile with Trend Mode. This RUN specifically uses VWAP Standard Deviation Bands (a different visualization method than Z-score distance) with RSI Momentum Crossover at band touches — the distinct mechanism is using outer band touches as extreme signals with RSI crossover confirmation at those extreme points.

## Proposed Config Changes (config.rs)

```rust
// ── RUN401: VWAP Standard Deviation Bands with RSI Momentum Crossover ─────────────────────────
// vwap_stdev_upper = vwap + MULT * stdev(vwap, period)
// vwap_stdev_lower = vwap - MULT * stdev(vwap, period)
// band_touch: price crosses above upper or below lower VWAP band
// rsi_momentum = rsi - EMA(rsi, signal_period)
// rsi_cross: rsi_momentum crosses above/below 0
// LONG: price touches lower_band AND rsi_momentum crosses bullish
// SHORT: price touches upper_band AND rsi_momentum crosses bearish

pub const VWAPSTD_RSI_ENABLED: bool = true;
pub const VWAPSTD_VWAP_PERIOD: usize = 20;
pub const VWAPSTD_STDEV_PERIOD: usize = 20;
pub const VWAPSTD_MULT: f64 = 2.0;        // bands at ±2 standard deviations
pub const VWAPSTD_RSI_PERIOD: usize = 14;
pub const VWAPSTD_RSI_SIGNAL: usize = 5;
pub const VWAPSTD_SL: f64 = 0.005;
pub const VWAPSTD_TP: f64 = 0.004;
pub const VWAPSTD_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run401_1_vwapstd_rsi_backtest.py)
2. **Walk-forward** (run401_2_vwapstd_rsi_wf.py)
3. **Combined** (run401_3_combined.py)

## Out-of-Sample Testing

- VWAP_PERIOD sweep: 14 / 20 / 30
- STDEV_PERIOD sweep: 14 / 20 / 30
- MULT sweep: 1.5 / 2.0 / 2.5
- RSI_SIGNAL sweep: 3 / 5 / 7
