# RUN311 — TRIX Triple Smooth Oscillator: Market Structure Cleaner

## Hypothesis

**Mechanism**: TRIX = triple-smoothed rate-of-change (three consecutive EMA layers applied to price). The triple smoothing filters out market noise — TRIX only responds to sustained moves. When TRIX crosses above zero → primary trend turning bullish. When TRIX crosses below zero → primary trend turning bearish. TRIX slope (change in TRIX over N bars) gives momentum. Use TRIX as a trend filter for shorter-term mean-reversion entries.

**Why not duplicate**: No prior RUN uses TRIX. RUN234 uses Schaff Trend Cycle (STC), which is a faster MACD-based cycle indicator. TRIX is distinct because the triple smoothing makes it the slowest, cleanest trend indicator — it eliminates noise at three levels. It's the most filtered of all momentum oscillators.

## Proposed Config Changes (config.rs)

```rust
// ── RUN311: TRIX Triple Smooth Oscillator ───────────────────────────────────
// trix = EMA(EMA(EMA(close, period), period), period)
// trix_roc = (trix - trix[1]) / trix[1] * 100  (rate of change of trix)
// LONG: trix crosses above 0 (trend turning up)
// SHORT: trix crosses below 0 (trend turning down)
// Momentum filter: trix_roc > 0 for longs, trix_roc < 0 for shorts

pub const TRIX_ENABLED: bool = true;
pub const TRIX_PERIOD: usize = 15;
pub const TRIX_ROC_PERIOD: usize = 1;
pub const TRIX_MOMENTUM_BARS: usize = 5;   // lookback for TRIX slope
pub const TRIX_SL: f64 = 0.005;
pub const TRIX_TP: f64 = 0.004;
pub const TRIX_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run311_1_trix_backtest.py)
2. **Walk-forward** (run311_2_trix_wf.py)
3. **Combined** (run311_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 15 / 20
- ROC_PERIOD sweep: 1 / 2 / 3
- MOMENTUM_BARS sweep: 3 / 5 / 10
