# RUN285 — Three Outside Down Pattern: Multi-Bar Bearish Reversal

## Hypothesis

**Mechanism**: Three Outside Down = a 3-candle bearish reversal pattern: (1) bar N is a large bullish bar, (2) bar N+1 opens above bar N's high and closes below bar N's low ( engulfing), (3) bar N+2 continues lower confirming the reversal. This is a strong bearish reversal signal.

**Why not duplicate**: No prior RUN uses Three Outside Down. All prior candlestick RUNs use doji, hammer, or engulfing (single pattern). Three Outside Down is a specific 3-bar reversal pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN285: Three Outside Down Pattern ──────────────────────────────────
// Bar1: large bullish (body > avg_body × 2)
// Bar2: opens above Bar1 high, closes below Bar1 low (engulfing bearish)
// Bar3: closes below Bar2 low (confirmation)
// SHORT: pattern confirmed

pub const TOD_PATTERN_ENABLED: bool = true;
pub const TOD_PATTERN_BODY_MULT: f64 = 2.0;  // Bar1 body multiplier
pub const TOD_PATTERN_SL: f64 = 0.005;
pub const TOD_PATTERN_TP: f64 = 0.004;
pub const TOD_PATTERN_MAX_HOLD: u32 = 24;
```

Add in `indicators.rs`:

```rust
pub fn three_outside_down(opens: &[f64], highs: &[f64], lows: &[f64], closes: &[f64]) -> bool {
    let n = closes.len();
    if n < 3 { return false; }

    let body1 = (closes[n-3] - opens[n-3]).abs();
    let body2 = (closes[n-2] - opens[n-2]).abs();

    let bar1_bullish = closes[n-3] > opens[n-3];
    let bar2_bearish = closes[n-2] < opens[n-2];

    let bar1_high = highs[n-3];
    let bar1_low = lows[n-3];

    let bar2_engulfs = opens[n-2] > bar1_high && closes[n-2] < bar1_low;
    let bar3_continues = closes[n-1] < closes[n-2];

    let avg_body = (body1 + body2) / 2.0;
    bar1_bullish && bar2_bearish && bar2_engulfs && bar3_continues && body1 > avg_body * 2.0
}
```

---

## Validation Method

1. **Historical backtest** (run285_1_tod_pattern_backtest.py)
2. **Walk-forward** (run285_2_tod_pattern_wf.py)
3. **Combined** (run285_3_combined.py)

## Out-of-Sample Testing

- BODY_MULT sweep: 1.5 / 2.0 / 2.5
