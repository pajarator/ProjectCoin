# RUN249 — MACD Histogram Rate of Change: Momentum Acceleration Detector

## Hypothesis

**Mechanism**: MACD Histogram = MACD line - Signal line. The histogram's rate of change (derivative) measures momentum acceleration. When histogram ROC > 0 AND histogram > 0 → accelerating bullish momentum. When histogram ROC < 0 AND histogram < 0 → accelerating bearish momentum. When histogram ROC flips from positive to negative → momentum is fading → exit LONG.

**Why not duplicate**: No prior RUN uses MACD histogram rate of change. All prior MACD RUNs use the MACD line crossover or histogram direction. Histogram ROC is unique because it measures *acceleration* of momentum, not just the momentum itself.

## Proposed Config Changes (config.rs)

```rust
// ── RUN249: MACD Histogram Rate of Change ────────────────────────────────
// macd_hist = macd_line - signal_line
// hist_roc = (hist - hist[N]) / hist[N] × 100
// LONG: hist_roc > 0 AND hist > 0 (accelerating bullish)
// SHORT: hist_roc < 0 AND hist < 0 (accelerating bearish)
// EXIT LONG: hist_roc flips to negative
// EXIT SHORT: hist_roc flips to positive

pub const MACD_HIST_ROC_ENABLED: bool = true;
pub const MACD_HIST_ROC_PERIOD: usize = 3;   // ROC lookback
pub const MACD_HIST_ROC_FAST: usize = 12;
pub const MACD_HIST_ROC_SLOW: usize = 26;
pub const MACD_HIST_ROC_SIGNAL: usize = 9;
pub const MACD_HIST_ROC_SL: f64 = 0.005;
pub const MACD_HIST_ROC_TP: f64 = 0.004;
pub const MACD_HIST_ROC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run249_1_macd_hist_roc_backtest.py)
2. **Walk-forward** (run249_2_macd_hist_roc_wf.py)
3. **Combined** (run249_3_combined.py)

## Out-of-Sample Testing

- ROC_PERIOD sweep: 2 / 3 / 5
- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
