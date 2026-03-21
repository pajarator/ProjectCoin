# RUN261 — Bollinger Band Width Rank: Historical Volatility Compression Position

## Hypothesis

**Mechanism**: Bollinger Band Width = (upper - lower) / middle. The percentile rank of current bandwidth tells you how compressed the market is relative to history. Width at 5th percentile = extremely compressed → volatility expansion imminent. Width at 95th percentile = extremely wide → volatility likely to contract.

**Why not duplicate**: No prior RUN uses Bollinger Band Width percentile rank. All prior volatility compression RUNs use ATR or Keltner channels. BB Width percentile rank is unique because it tells you where the *current volatility* falls in the historical distribution.

## Proposed Config Changes (config.rs)

```rust
// ── RUN261: Bollinger Band Width Percentile Rank ──────────────────────────
// bb_width = (upper - lower) / middle
// width_percentile = percentile rank of bb_width in its history
// width_percentile < 10 → extremely compressed → breakout imminent
// width_percentile > 90 → extremely wide → contraction imminent

pub const BB_WIDTH_PCT_ENABLED: bool = true;
pub const BB_WIDTH_PERIOD: usize = 20;        // BB period
pub const BB_WIDTH_STD: f64 = 2.0;           // BB std dev multiplier
pub const BB_WIDTH_WINDOW: usize = 100;       // history window for percentile
pub const BB_WIDTH_COMPRESS: f64 = 10.0;      // compression threshold
pub const BB_WIDTH_EXPAND: f64 = 90.0;        // expansion threshold
pub const BB_WIDTH_SL: f64 = 0.005;
pub const BB_WIDTH_TP: f64 = 0.004;
pub const BB_WIDTH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run261_1_bb_width_pct_backtest.py)
2. **Walk-forward** (run261_2_bb_width_pct_wf.py)
3. **Combined** (run261_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 15 / 20 / 30
- WINDOW sweep: 50 / 100 / 200
- COMPRESS sweep: 5 / 10 / 15
- EXPAND sweep: 85 / 90 / 95
