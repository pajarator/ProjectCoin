# RUN235 — Rainbow Chart Trend Alignment: Multi-MA Direction Consensus

## Hypothesis

**Mechanism**: Rainbow Chart = 10 EMAs at different periods (5, 10, 15, 20, 25, 30, 35, 40, 45, 50). When all 10 EMAs are aligned in ascending order → strong uptrend (don't fade). When all 10 are in descending order → strong downtrend. When EMAs are tangled/crossing → no trend, use mean-reversion. Count how many EMAs are in the correct trend alignment as a confidence score.

**Why not duplicate**: No prior RUN uses Rainbow Chart alignment. All prior EMA cross RUNs use only 2-3 EMAs. Rainbow's 10-EMA consensus approach is fundamentally different — it's a *crowd wisdom* measure where each EMA represents a different time horizon's view.

## Proposed Config Changes (config.rs)

```rust
// ── RUN235: Rainbow Chart Trend Alignment ───────────────────────────────
// ema_periods = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50]
// alignment_score = count of correctly ordered EMA pairs
// 10/10 = full alignment → trend mode
// 5/10 = mixed → mean-reversion mode
// LONG: alignment_score >= 7 AND price > ema_10
// SHORT: alignment_score >= 7 AND price < ema_10

pub const RAINBOW_ENABLED: bool = true;
pub const RAINBOW_MIN_ALIGN: u32 = 7;        // minimum aligned EMAs for trend
pub const RAINBOW_EMA_COUNT: usize = 10;   // number of EMAs
pub const RAINBOW_SL: f64 = 0.005;
pub const RAINBOW_TP: f64 = 0.004;
pub const RAINBOW_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run235_1_rainbow_backtest.py)
2. **Walk-forward** (run235_2_rainbow_wf.py)
3. **Combined** (run235_3_combined.py)

## Out-of-Sample Testing

- MIN_ALIGN sweep: 6 / 7 / 8
- EMA_PERIODS sweep: different period sets
