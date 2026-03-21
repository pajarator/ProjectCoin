# RUN228 — Consecutive Bar Exhaustion: N-Bar Momentum Mean Reversion

## Hypothesis

**Mechanism**: After N consecutive bars closing higher (or lower), the market becomes "exhausted" — the directional momentum has run out of steam. When 4+ consecutive bars close higher → price is likely to see at least one down bar (mean reversion). When 4+ consecutive bars close lower → price is likely to see at least one up bar. This is a pure short-term mean-reversion signal based on momentum exhaustion.

**Why not duplicate**: No prior RUN uses consecutive bar counting. All prior mean-reversion RUNs use RSI, Bollinger Bands, or Z-score. Consecutive bar exhaustion is a completely different signal type — it's based on the *count* of directional bars, not their magnitude.

## Proposed Config Changes (config.rs)

```rust
// ── RUN228: Consecutive Bar Exhaustion ───────────────────────────────────
// count consecutive bars with close > open (up) or close < open (down)
// LONG: 4+ consecutive down bars → expect mean-reversion up
// SHORT: 4+ consecutive up bars → expect mean-reversion down
// Entry on the 4th (or 5th) consecutive bar

pub const CONSEC_ENABLED: bool = true;
pub const CONSEC_THRESH: u32 = 4;            // exhaustion threshold
pub const CONSEC_CONFIRM: u32 = 1;          // entry on bar N+1 (reversal confirmed)
pub const CONSEC_SL: f64 = 0.005;
pub const CONSEC_TP: f64 = 0.004;
pub const CONSEC_MAX_HOLD: u32 = 12;        // very short hold - 3 hours at 15m
```

Add in `indicators.rs`:

```rust
pub fn consecutive_bars(closes: &[f64]) -> i32 {
    let n = closes.len();
    if n < 2 {
        return 0;
    }

    let mut count = 0;
    let mut direction = 0i32; // 1 = up, -1 = down, 0 = neutral

    for i in (1..n).rev() {
        let is_up = closes[i] > closes[i-1];
        let is_down = closes[i] < closes[i-1];

        if count == 0 {
            if is_up { direction = 1; count = 1; }
            else if is_down { direction = -1; count = 1; }
        } else {
            if direction == 1 && is_up { count += 1; }
            else if direction == -1 && is_down { count += 1; }
            else { break; }
        }
    }

    count as i32 * direction
}
```

---

## Validation Method

1. **Historical backtest** (run228_1_consec_backtest.py)
2. **Walk-forward** (run228_2_consec_wf.py)
3. **Combined** (run228_3_combined.py)

## Out-of-Sample Testing

- THRESH sweep: 3 / 4 / 5 / 6
- CONFIRM sweep: 1 / 2
