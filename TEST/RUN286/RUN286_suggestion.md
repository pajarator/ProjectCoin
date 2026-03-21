# RUN286 — Three Inside Up Pattern: Multi-Bar Bullish Reversal

## Hypothesis

**Mechanism**: Three Inside Up = a 3-candle bullish reversal pattern: (1) bar N is a large bearish bar, (2) bar N+1 is a small bar inside bar N's range, (3) bar N+2 closes above bar N's high confirming reversal. This is a strong bullish reversal signal.

**Why not duplicate**: No prior RUN uses Three Inside Up. All prior multi-bar patterns use outside bars or engulfing. Three Inside Up is a specific 3-bar reversal pattern distinct from others.

## Proposed Config Changes (config.rs)

```rust
// ── RUN286: Three Inside Up Pattern ─────────────────────────────────────
// Bar1: large bearish (body > avg_body × 2)
// Bar2: small bar inside Bar1 range (higher_low < bar1_low, lower_high < bar1_high)
// Bar3: closes above Bar1 high (confirmation)
// LONG: pattern confirmed

pub const TIU_PATTERN_ENABLED: bool = true;
pub const TIU_PATTERN_BODY_MULT: f64 = 2.0;  // Bar1 body multiplier
pub const TIU_PATTERN_SL: f64 = 0.005;
pub const TIU_PATTERN_TP: f64 = 0.004;
pub const TIU_PATTERN_MAX_HOLD: u32 = 24;
```

Add in `indicators.rs`:

```rust
pub fn three_inside_up(opens: &[f64], highs: &[f64], lows: &[f64], closes: &[f64]) -> bool {
    let n = closes.len();
    if n < 3 { return false; }

    let body1 = (closes[n-3] - opens[n-3]).abs();
    let body2 = (closes[n-2] - opens[n-2]).abs();

    let bar1_bearish = closes[n-3] < opens[n-3];
    let bar2_inside = highs[n-2] < highs[n-3] && lows[n-2] > lows[n-3];
    let bar3_confirms = closes[n-1] > highs[n-3];

    let avg_body = (body1 + body2) / 2.0;
    bar1_bearish && bar2_inside && bar3_confirms && body1 > avg_body * 2.0
}
```

---

## Validation Method

1. **Historical backtest** (run286_1_tiu_pattern_backtest.py)
2. **Walk-forward** (run286_2_tiu_pattern_wf.py)
3. **Combined** (run286_3_combined.py)

## Out-of-Sample Testing

- BODY_MULT sweep: 1.5 / 2.0 / 2.5
