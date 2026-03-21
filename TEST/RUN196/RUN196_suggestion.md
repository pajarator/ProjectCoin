# RUN196 — Rate of Change (ROC) Threshold: Simple Momentum Magnitude Filter

## Hypothesis

**Mechanism**: ROC = ((close - close[N]) / close[N]) × 100. It measures the percentage price change over N periods — pure momentum without smoothing. High ROC (>5%) = overextended move, likely to mean-revert. Low ROC near zero = consolidation, likely to trend. Use ROC as a filter: only enter when momentum is neither overextended nor dead.

**Why not duplicate**: No prior RUN uses ROC. MACD RUNs exist (RUN13, RUN170) but ROC is fundamentally different — it's a single lookback ratio, not a difference of EMAs. ROC captures raw momentum magnitude.

## Proposed Config Changes (config.rs)

```rust
// ── RUN196: Rate of Change (ROC) Momentum Filter ─────────────────────────
// roc = ((close - close[period]) / close[period]) × 100
// LONG: ROC between -2% and +3% (not overextended, not dead)
// SHORT: ROC between -3% and +2% (inverted)
// Overextended (>5% or <-5%) → skip, mean-reversion likely
// Dead (between -1% and +1%) → skip, no momentum

pub const ROC_ENABLED: bool = true;
pub const ROC_PERIOD: usize = 12;          // lookback period
pub const ROC_LONG_MIN: f64 = -2.0;        // minimum ROC for LONG
pub const ROC_LONG_MAX: f64 = 3.0;         // maximum ROC for LONG (not overextended)
pub const ROC_SHORT_MIN: f64 = -3.0;       // minimum ROC for SHORT (not overextended)
pub const ROC_SHORT_MAX: f64 = 2.0;        // maximum ROC for SHORT
pub const ROC_OVEREXTENDED: f64 = 5.0;     // overextended threshold
pub const ROC_SL: f64 = 0.005;
pub const ROC_TP: f64 = 0.004;
pub const ROC_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn roc(closes: &[f64], period: usize) -> f64 {
    let n = closes.len();
    if n <= period {
        return 0.0;
    }
    let current = closes[n - 1];
    let past = closes[n - 1 - period];
    if past == 0.0 {
        return 0.0;
    }
    ((current - past) / past) * 100.0
}
```

---

## Validation Method

1. **Historical backtest** (run196_1_roc_backtest.py)
2. **Walk-forward** (run196_2_roc_wf.py)
3. **Combined** (run196_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 6 / 12 / 20 / 30
- LONG_MIN sweep: -1 / -2 / -3
- LONG_MAX sweep: 2 / 3 / 4 / 5
- OVEREXTENDED sweep: 4 / 5 / 6
