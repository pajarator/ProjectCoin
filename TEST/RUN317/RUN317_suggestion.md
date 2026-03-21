# RUN317 — Aroon Oscillator Momentum: Trend vs Range Classification

## Hypothesis

**Mechanism**: Aroon measures how many bars since the highest high (Aroon Up) and lowest low (Aroon Down) within a lookback window. Aroon Oscillator = Aroon Up - Aroon Down. When AO > 0 = bullish trend. When AO < 0 = bearish trend. Values near 0 = range-bound market. Use AO as a filter: only enter mean-reversion when AO is near 0 (range-bound), because mean-reversion works poorly in strong trends.

**Why not duplicate**: RUN204 uses Aroon as trend confirmation filter. RUN114 uses Aroon confirmation for entries. This RUN uses Aroon specifically as a range-bound detector to gate mean-reversion entries. When Aroon is strongly positive or negative, suppress mean-reversion signals because the market is trending.

## Proposed Config Changes (config.rs)

```rust
// ── RUN317: Aroon Oscillator Range Filter ─────────────────────────────────────
// aroon_up = (lookback - bars_since_highest_high) / lookback * 100
// aroon_down = (lookback - bars_since_lowest_low) / lookback * 100
// aroon_osc = aroon_up - aroon_down
// LONG (mean-reversion): aroon_osc near 0 (|osc| < RANGE_THRESH) AND RSI extreme
// SHORT (mean-reversion): aroon_osc near 0 AND RSI extreme
// Trend mode: if |aroon_osc| > RANGE_THRESH → suppress mean-reversion

pub const AROON_RANGE_ENABLED: bool = true;
pub const AROON_LOOKBACK: usize = 25;
pub const AROON_RANGE_THRESH: f64 = 20.0;    // |osc| < 20 = range-bound
pub const AROON_RSI_LONG: f64 = 70.0;       // RSI for SHORT mean-reversion
pub const AROON_RSI_SHORT: f64 = 30.0;      // RSI for LONG mean-reversion
pub const AROON_SL: f64 = 0.005;
pub const AROON_TP: f64 = 0.004;
pub const AROON_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run317_1_aroon_range_backtest.py)
2. **Walk-forward** (run317_2_aroon_range_wf.py)
3. **Combined** (run317_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 14 / 25 / 50
- RANGE_THRESH sweep: 10 / 20 / 30
- RSI_LONG sweep: 60 / 70 / 80
- RSI_SHORT sweep: 20 / 30 / 40
