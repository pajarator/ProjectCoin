# RUN204 — Aroon Indicator: Time-Since-High/Low for Trend Reversal Detection

## Hypothesis

**Mechanism**: Aroon = (period - bars_since_HH_or_LL) / period × 100. Aroon Up measures how recently the highest high occurred; Aroon Down measures how recently the lowest low occurred. Aroon Up > 70 AND rising = strong bullish trend. Aroon Down > 70 AND rising = strong bearish trend. Crossover of Aroon Up above Aroon Down → LONG. Crossover below → SHORT.

**Why not duplicate**: No prior RUN uses Aroon. All prior trend-following RUNs use EMA crosses or ADX. Aroon is unique because it measures *time* since extremes, not price derivatives — making it sensitive to trend momentum changes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN204: Aroon Indicator ─────────────────────────────────────────────
// aroon_up = (period - bars_since_highest_high) / period × 100
// aroon_down = (period - bars_since_lowest_low) / period × 100
// aroon_oscillator = aroon_up - aroon_down
// LONG: aroon_up crosses above aroon_down AND > 50
// SHORT: aroon_down crosses above aroon_up AND > 50
// Neutral: both < 30 (no trend)

pub const AROON_ENABLED: bool = true;
pub const AROON_PERIOD: usize = 25;           // lookback period
pub const AROON_ENTER_THRESH: f64 = 70.0;     // strong trend confirmation
pub const AROON_EXIT_THRESH: f64 = 30.0;      // no-trend threshold
pub const AROON_SL: f64 = 0.005;
pub const AROON_TP: f64 = 0.004;
pub const AROON_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn aroon(highs: &[f64], lows: &[f64], period: usize) -> (f64, f64, f64) {
    let n = highs.len().min(lows.len());
    if n < period {
        return (50.0, 50.0, 0.0);
    }

    let mut bars_since_hh = period;
    let mut bars_since_ll = period;

    let mut hh_val = f64::NEG_INFINITY;
    let mut ll_val = f64::INFINITY;

    for i in 0..period {
        if highs[n-1-i] > hh_val {
            hh_val = highs[n-1-i];
            bars_since_hh = i;
        }
        if lows[n-1-i] < ll_val {
            ll_val = lows[n-1-i];
            bars_since_ll = i;
        }
    }

    let aroon_up = ((period - bars_since_hh) as f64 / (period as f64)) * 100.0;
    let aroon_down = ((period - bars_since_ll) as f64 / (period as f64)) * 100.0;
    let oscillator = aroon_up - aroon_down;

    (aroon_up, aroon_down, oscillator)
}
```

---

## Validation Method

1. **Historical backtest** (run204_1_aroon_backtest.py)
2. **Walk-forward** (run204_2_aroon_wf.py)
3. **Combined** (run204_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 25 / 50
- ENTER_THRESH sweep: 60 / 70 / 80
- EXIT_THRESH sweep: 20 / 30 / 40
