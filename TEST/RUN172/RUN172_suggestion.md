# RUN172 — Candle Body-to-Wick Ratio Filter: Dominant Wick as Rejection/Absorption Signal

## Hypothesis

**Mechanism**: A candle with a long upper wick and small body (shooting star / gravestone doji) signals rejection — price tried to push higher and was absorbed. A candle with long lower wick and small body (hammer / inverted gravestone doji) signals support absorption. Body/wick ratio < 0.3 with wick > 67% of candle range = strong rejection. COINCLAW uses candle composition (RUN50) but not body/wick ratio.

**Why not duplicate**: RUN50 (Candle Composition Filter) uses body-to-range ratio. This RUN uses specifically the wick dominance ratio and direction — a distinct mechanical pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN172: Candle Body-to-Wick Ratio Filter ──────────────────────────
// body_to_wick = body / wick (lower = more rejection)
// Wick dominance = wick / range (higher = more rejection)
// Entry: LONG when lower wick dominant + RSI < 35
// Entry: SHORT when upper wick dominant + RSI > 65

pub const BWICK_ENABLED: bool = true;
pub const BWICK_WICK_PCT: f64 = 0.67;    // wick must be >67% of range
pub const BWICK_BODY_MAX: f64 = 0.33;    // body must be <33% of range
pub const BWICK_RSI_LONG: f64 = 35.0;   // RSI confirmation for LONG
pub const BWICK_RSI_SHORT: f64 = 65.0;  // RSI confirmation for SHORT
```

Add to `Ind15m` in `indicators.rs`:

```rust
pub upper_wick_pct: f64,   // upper_wick / range (0-1)
pub lower_wick_pct: f64,   // lower_wick / range (0-1)
pub body_pct: f64,          // body / range (0-1)
```

Add in `indicators.rs`:

```rust
pub fn compute_wick_ratios(c: &Candle) -> (f64, f64, f64) {
    let range = c.h - c.l;
    if range == 0.0 { return (0.0, 0.0, 1.0); }
    let upper_wick = c.h - c.c.max(c.o);
    let lower_wick = c.c.min(c.o) - c.l;
    let body = (c.c - c.o).abs();
    (upper_wick / range, lower_wick / range, body / range)
}
```

Add in `engine.rs`:

```rust
fn check_wick_entry(cs: &CoinState, ind: &Ind15m) -> Option<(Direction, &'static str)> {
    if !config::BWICK_ENABLED { return None; }
    let (upper, lower, body) = (ind.upper_wick_pct, ind.lower_wick_pct, ind.body_pct);
    if body > config::BWICK_BODY_MAX { return None; }
    // Shooting star: upper wick dominant → SHORT
    if upper > config::BWICK_WICK_PCT && ind.rsi > config::BWICK_RSI_SHORT {
        return Some((Direction::Short, "wick_rejection"));
    }
    // Hammer: lower wick dominant → LONG
    if lower > config::BWICK_WICK_PCT && ind.rsi < config::BWICK_RSI_LONG {
        return Some((Direction::Long, "wick_support"));
    }
    None
}
```

---

## Validation Method

1. **Historical backtest** (run172_1_bwick_backtest.py)
2. **Walk-forward** (run172_2_bwick_wf.py)
3. **Combined** (run172_3_combined.py)

## Out-of-Sample Testing

- WICK_PCT sweep: 0.60 / 0.67 / 0.75
- BODY_MAX sweep: 0.25 / 0.33 / 0.40
