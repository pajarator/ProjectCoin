# RUN169 — RSI Extreme Percentile Calibration: Rolling Historical RSI Distribution as Dynamic Threshold

## Hypothesis

**Mechanism**: COINCLAW uses fixed RSI thresholds (e.g., RSI < 40 for LONG). But "oversold" is coin-specific and time-varying — BTC's RSI rarely goes below 30 while SHIB's RSI regularly hits 10. A rolling percentile-based threshold (e.g., RSI < 10th percentile of past 100 bars) adapts to each coin's natural RSI distribution, producing better signals than fixed thresholds.

**Why not duplicate**: No prior RUN uses rolling percentile-based thresholds for RSI. All prior RSI thresholds are fixed values.

## Proposed Config Changes (config.rs)

```rust
// ── RUN169: RSI Percentile-Based Thresholds ──────────────────────────────
// For each coin: track RSI distribution over rolling window
// LONG threshold = Nth percentile of past RSI readings (e.g., 15th percentile)
// SHORT threshold = (100 - N)th percentile (e.g., 85th percentile)

pub const RSI_PCT_ENABLED: bool = true;
pub const RSI_PCT_WINDOW: usize = 100;    // rolling window for RSI distribution
pub const RSI_PCT_LONG: f64 = 0.15;      // LONG when RSI < 15th percentile
pub const RSI_PCT_SHORT: f64 = 0.85;    // SHORT when RSI > 85th percentile
```

Add to `CoinState` in `state.rs`:

```rust
pub rsi_history: Vec<f64>,          // rolling RSI history for percentile calculation
```

Add in `indicators.rs`:

```rust
/// Compute Nth percentile of rolling RSI history
pub fn rsi_percentile(rsi_history: &[f64], threshold: f64) -> Option<f64> {
    if rsi_history.len() < 10 { return None; }
    let mut sorted = rsi_history.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = (threshold * sorted.len() as f64).floor() as usize;
    sorted.get(idx.min(sorted.len() - 1)).copied()
}
```

Modify regime LONG entry condition:
```rust
// OLD: RSI < 40.0
// NEW: RSI < rsi_percentile(rsi_history, RSI_PCT_LONG)
```

---

## Validation Method

1. **Historical backtest** (run169_1_rsipct_backtest.py): 18 coins, sweep percentiles
2. **Walk-forward** (run169_2_rsipct_wf.py): 3-window walk-forward
3. **Combined** (run169_3_combined.py): vs baseline fixed thresholds

## Out-of-Sample Testing

- LONG_PCT sweep: 0.10 / 0.15 / 0.20
- SHORT_PCT sweep: 0.80 / 0.85 / 0.90
- WINDOW sweep: 50 / 100 / 200
