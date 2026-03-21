# RUN239 — Inside Bar Pattern: Consolidation Breakout Momentum

## Hypothesis

**Mechanism**: An inside bar = today's high < yesterday's high AND today's low > yesterday's low. It represents consolidation — the market is coiling before a directional move. The breakout direction (above yesterday's high or below yesterday's low) determines the trade direction. Volume confirmation: the breakout bar must have above-average volume for higher reliability.

**Why not duplicate**: No prior RUN uses Inside Bar pattern. All prior candlestick pattern RUNs use doji, hammer, or engulfing. Inside bar is a specific N-bar pattern (not a single candle) that measures consolidation — a distinct momentum-building setup.

## Proposed Config Changes (config.rs)

```rust
// ── RUN239: Inside Bar Pattern ───────────────────────────────────────────
// inside_bar = high < prior_high AND low > prior_low
// consolidation = inside_bar confirmed for N consecutive bars
// LONG: price breaks above prior_high AND volume > vol_ma
// SHORT: price breaks below prior_low AND volume > vol_ma

pub const INSIDE_BAR_ENABLED: bool = true;
pub const INSIDE_BAR_CONSECUTIVE: u32 = 1;   // number of consecutive inside bars
pub const INSIDE_BAR_VOL_MA: usize = 20;    // volume MA for confirmation
pub const INSIDE_BAR_VOL_MULT: f64 = 1.2;    // volume must exceed MA × this
pub const INSIDE_BAR_SL: f64 = 0.005;
pub const INSIDE_BAR_TP: f64 = 0.004;
pub const INSIDE_BAR_MAX_HOLD: u32 = 24;
```

Add in `indicators.rs`:

```rust
pub fn inside_bar(highs: &[f64], lows: &[f64]) -> bool {
    let n = highs.len().min(lows.len());
    if n < 2 {
        return false;
    }
    highs[n-1] < highs[n-2] && lows[n-1] > lows[n-2]
}
```

---

## Validation Method

1. **Historical backtest** (run239_1_inside_bar_backtest.py)
2. **Walk-forward** (run239_2_inside_bar_wf.py)
3. **Combined** (run239_3_combined.py)

## Out-of-Sample Testing

- CONSECUTIVE sweep: 1 / 2 / 3
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
