# RUN418 — VWAP Distance Histogram with Stochastic Confirmation

## Hypothesis

**Mechanism**: VWAP Distance Histogram tracks how far price deviates from VWAP over time, showing the distribution of deviations. When this histogram reaches extreme values (far from mean), price is likely to revert to VWAP. Stochastic Confirmation adds momentum verification: when the histogram shows extreme deviation AND Stochastic crosses in the direction of the reversion, you have both statistical deviation and momentum confirmation for the trade.

**Why not duplicate**: RUN326 uses Z-Score Distance from VWAP. RUN343 uses VWAP Deviation Percentile with Trend Mode. This RUN specifically uses a histogram distribution of VWAP distance (showing the full distribution of deviations, not just percentile rank) with Stochastic crossover confirmation — the distinct mechanism is histogram-based deviation analysis combined with Stochastic momentum confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN418: VWAP Distance Histogram with Stochastic Confirmation ──────────────────────────────
// vwap_distance = close - vwap
// distance_histogram = distribution of vwap_distance over lookback
// hist_extreme: distance is beyond HIST_THRESH standard deviations from mean
// stochastic = stochastic_oscillator(close, period)
// stoch_cross: stochastic crosses above/below signal line in oversold/overbought zone
// LONG: hist_extreme on downside AND stoch crosses bullish in oversold zone
// SHORT: hist_extreme on upside AND stoch crosses bearish in overbought zone

pub const VWAPDIST_STOCH_ENABLED: bool = true;
pub const VWAPDIST_STOCH_VWAP_PERIOD: usize = 20;
pub const VWAPDIST_STOCH_HIST_PERIOD: usize = 20;
pub const VWAPDIST_STOCH_HIST_THRESH: f64 = 2.0;   // standard deviations
pub const VWAPDIST_STOCH_STOCH_PERIOD: usize = 14;
pub const VWAPDIST_STOCH_STOCH_OVERSOLD: f64 = 20.0;
pub const VWAPDIST_STOCH_STOCH_OVERBOUGHT: f64 = 80.0;
pub const VWAPDIST_STOCH_SL: f64 = 0.005;
pub const VWAPDIST_STOCH_TP: f64 = 0.004;
pub const VWAPDIST_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run418_1_vwapdist_stoch_backtest.py)
2. **Walk-forward** (run418_2_vwapdist_stoch_wf.py)
3. **Combined** (run418_3_combined.py)

## Out-of-Sample Testing

- VWAP_PERIOD sweep: 14 / 20 / 30
- HIST_PERIOD sweep: 14 / 20 / 30
- HIST_THRESH sweep: 1.5 / 2.0 / 2.5
- STOCH_PERIOD sweep: 10 / 14 / 21
