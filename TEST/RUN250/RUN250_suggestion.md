# RUN250 — Percent Rank of ATR: Volatility Regime Position Sizer

## Hypothesis

**Mechanism**: ATR percentile rank = where current ATR falls within its N-bar historical distribution (expressed as %). ATR at 95th percentile = extremely high volatility → reduce position size. ATR at 5th percentile = extremely low volatility → increase position size. The percentile rank is coin-specific and adapts to each coin's natural volatility level.

**Why not duplicate**: No prior RUN uses ATR percentile rank. All prior volatility RUNs use absolute ATR values or Bollinger Bandwidth. Percentile rank is unique because it's a *relative* measure — it tells you where the current value falls in the historical distribution, making it comparable across different volatility regimes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN250: Percent Rank ATR for Position Sizing ─────────────────────────
// atr_percentile = percentile rank of current ATR within ATR history
// position_size = base_size × atr_percentile_factor
// atr_percentile < 20: low vol → increase size × 1.5
// atr_percentile > 80: high vol → decrease size × 0.5
// atr_percentile 20-80: normal size

pub const ATR_PCT_RANK_ENABLED: bool = true;
pub const ATR_PCT_RANK_WINDOW: usize = 100;  // historical window for percentile
pub const ATR_PCT_RANK_LOW: f64 = 20.0;     // low vol threshold (percentile)
pub const ATR_PCT_RANK_HIGH: f64 = 80.0;     // high vol threshold (percentile)
pub const ATR_PCT_RANK_LOW_MULT: f64 = 1.5;  // size multiplier for low vol
pub const ATR_PCT_RANK_HIGH_MULT: f64 = 0.5; // size multiplier for high vol
```

Add to indicators.rs:

```rust
pub fn atr_percentile_rank(atr_history: &[f64], current_atr: f64) -> f64 {
    if atr_history.is_empty() {
        return 50.0;
    }
    let count_below = atr_history.iter().filter(|&&x| x < current_atr).count();
    (count_below as f64 / (atr_history.len() as f64)) * 100.0
}
```

---

## Validation Method

1. **Historical backtest** (run250_1_atr_pct_rank_backtest.py)
2. **Walk-forward** (run250_2_atr_pct_rank_wf.py)
3. **Combined** (run250_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 50 / 100 / 200
- LOW_THRESH sweep: 15 / 20 / 25
- HIGH_THRESH sweep: 75 / 80 / 85
- LOW_MULT sweep: 1.2 / 1.5 / 2.0
- HIGH_MULT sweep: 0.3 / 0.5 / 0.7
