# RUN269 — Opening 30-Minute Range Momentum: First-Hour Trend Bias

## Hypothesis

**Mechanism**: The first 30 minutes of a trading session (00:00-00:30 UTC) sets the tone for the day. If price ends the first 30 minutes above the opening price → bullish bias for the day. If below → bearish bias. Trade with the opening range direction.

**Why not duplicate**: No prior RUN uses opening 30-minute range. All prior session-based RUNs use longer timeframes (1h, 4h). The 30-minute opening range is a distinct short-term session bias indicator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN269: Opening 30-Minute Range Momentum ────────────────────────────
// open_range_high = highest high of first 2 bars (30 min at 15m)
// open_range_low = lowest low of first 2 bars
// bias_bullish = close > open_range_high
// bias_bearish = close < open_range_low
// LONG: bias_bullish confirmed AND price > EMA20
// SHORT: bias_bearish confirmed AND price < EMA20

pub const OPEN30_ENABLED: bool = true;
pub const OPEN30_BARS: usize = 2;           // 2 bars = 30 min
pub const OPEN30_EMA: usize = 20;           // trend EMA
pub const OPEN30_SL: f64 = 0.005;
pub const OPEN30_TP: f64 = 0.004;
pub const OPEN30_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run269_1_open30_backtest.py)
2. **Walk-forward** (run269_2_open30_wf.py)
3. **Combined** (run269_3_combined.py)

## Out-of-Sample Testing

- BARS sweep: 2 / 4 / 6 (30/60/90 min)
- EMA sweep: 10 / 20 / 50
