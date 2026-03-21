# RUN240 — Outside Bar Reversal: Volatile Trend Exhaustion Signal

## Hypothesis

**Mechanism**: An outside bar = today's high > yesterday's high AND today's low < yesterday's low. It completely engulfs the prior bar — a volatile reversal signal. When an outside bar occurs AND RSI is overbought/oversold → strong reversal probability. When price is near a support/resistance level AND outside bar forms → reversal entry.

**Why not duplicate**: No prior RUN uses Outside Bar reversal. All prior candlestick pattern RUNs use doji or single-bar patterns. Outside bar is distinct because it measures the *engulfing* of one bar by another — a strong momentum reversal signal that captures volatile turn points.

## Proposed Config Changes (config.rs)

```rust
// ── RUN240: Outside Bar Reversal ─────────────────────────────────────────
// outside_bar = high > prior_high AND low < prior_low
// LONG reversal: outside_bar AND RSI < 30 (at oversold)
// SHORT reversal: outside_bar AND RSI > 70 (at overbought)
// Volume confirmation strengthens signal

pub const OUTSIDE_BAR_ENABLED: bool = true;
pub const OUTSIDE_BAR_RSI_PERIOD: usize = 14;
pub const OUTSIDE_BAR_RSI_OVERSOLD: f64 = 30.0;
pub const OUTSIDE_BAR_RSI_OVERBOUGHT: f64 = 70.0;
pub const OUTSIDE_BAR_VOL_MA: usize = 20;
pub const OUTSIDE_BAR_SL: f64 = 0.005;
pub const OUTSIDE_BAR_TP: f64 = 0.004;
pub const OUTSIDE_BAR_MAX_HOLD: u32 = 24;
```

Add in `indicators.rs`:

```rust
pub fn outside_bar(highs: &[f64], lows: &[f64]) -> bool {
    let n = highs.len().min(lows.len());
    if n < 2 {
        return false;
    }
    highs[n-1] > highs[n-2] && lows[n-1] < lows[n-2]
}
```

---

## Validation Method

1. **Historical backtest** (run240_1_outside_bar_backtest.py)
2. **Walk-forward** (run240_2_outside_bar_wf.py)
3. **Combined** (run240_3_combined.py)

## Out-of-Sample Testing

- RSI_OVERSOLD sweep: 20 / 30 / 40
- RSI_OVERBOUGHT sweep: 60 / 70 / 80
