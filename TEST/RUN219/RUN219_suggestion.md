# RUN219 — Psychological Line: Percentage of Up-Closes as Sentiment Gauge

## Hypothesis

**Mechanism**: Psychological Line = (number of bars closing higher than previous bar) / (total bars) × 100. It measures how consistently the market has been closing higher over a period. Below 30% = extreme bearish sentiment → LONG opportunity. Above 70% = extreme bullish sentiment → SHORT opportunity. The market tends to reverse when sentiment reaches extremes.

**Why not duplicate**: No prior RUN uses Psychological Line. All prior sentiment/momentum RUNs use price-based indicators (RSI, MACD). Psychological Line is unique because it measures the *consistency* of closes, not their magnitude — a fundamentally different sentiment dimension.

## Proposed Config Changes (config.rs)

```rust
// ── RUN219: Psychological Line ───────────────────────────────────────────
// psych_line = count(close > close[1]) / period × 100
// psych_line < 30 → oversold → LONG
// psych_line > 70 → overbought → SHORT
// Zero-line crossover (50) as trend confirmation

pub const PSYCH_ENABLED: bool = true;
pub const PSYCH_PERIOD: usize = 20;          // lookback period
pub const PSYCH_OVERSOLD: f64 = 30.0;        // oversold threshold
pub const PSYCH_OVERBOUGHT: f64 = 70.0;       // overbought threshold
pub const PSYCH_SL: f64 = 0.005;
pub const PSYCH_TP: f64 = 0.004;
pub const PSYCH_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn psychological_line(closes: &[f64], period: usize) -> f64 {
    let n = closes.len();
    if n < period + 1 {
        return 50.0;
    }

    let mut up_count = 0;
    for i in (n-period)..n {
        if closes[i] > closes[i-1] {
            up_count += 1;
        }
    }

    (up_count as f64 / (period as f64)) * 100.0
}
```

---

## Validation Method

1. **Historical backtest** (run219_1_psych_backtest.py)
2. **Walk-forward** (run219_2_psych_wf.py)
3. **Combined** (run219_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 30 / 50
- OVERSOLD sweep: 20 / 30 / 40
- OVERBOUGHT sweep: 60 / 70 / 80
