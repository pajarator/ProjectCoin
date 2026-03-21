# RUN262 — RSI Range Percentile: Oscillator Position in Historical Distribution

## Hypothesis

**Mechanism**: RSI often gets stuck in overbought/oversold without crossing. RSI percentile rank = where does the current RSI fall within its N-bar historical range? RSI at its 95th percentile = extremely overbought even if not above 70. RSI at 5th percentile = extremely oversold even if not below 30. Trade extremes of RSI percentile rank.

**Why not duplicate**: No prior RUN uses RSI percentile rank. All prior RSI RUNs use absolute thresholds. RSI percentile rank is unique because it tells you where the *current RSI* falls in the historical distribution, not just whether it's above/below fixed levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN262: RSI Range Percentile ─────────────────────────────────────────
// rsi_percentile = percentile rank of current RSI within RSI history
// rsi_percentile > 90 → extremely overbought → SHORT
// rsi_percentile < 10 → extremely oversold → LONG

pub const RSI_PCT_ENABLED: bool = true;
pub const RSI_PCT_PERIOD: usize = 14;         // base RSI period
pub const RSI_PCT_WINDOW: usize = 100;        // history window for percentile
pub const RSI_PCT_OVERSOLD: f64 = 10.0;      // percentile threshold
pub const RSI_PCT_OVERBOUGHT: f64 = 90.0;    // percentile threshold
pub const RSI_PCT_SL: f64 = 0.005;
pub const RSI_PCT_TP: f64 = 0.004;
pub const RSI_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run262_1_rsi_pct_backtest.py)
2. **Walk-forward** (run262_2_rsi_pct_wf.py)
3. **Combined** (run262_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- WINDOW sweep: 50 / 100 / 200
- OVERSOLD sweep: 5 / 10 / 15
- OVERBOUGHT sweep: 85 / 90 / 95
