# RUN229 — Opening Range Breakout (ORB): First-N-Bar Range as Momentum Catalyst

## Hypothesis

**Mechanism**: The opening range (highest high and lowest low of the first N bars of a session or lookback window) establishes a "battle zone." When price breaks above the range → bullish momentum → LONG. When price breaks below → bearish momentum → SHORT. ORB captures the first significant directional move of a session and rides it.

**Why not duplicate**: No prior RUN uses Opening Range Breakout. All prior breakout RUNs use N-period highs/lows without the "opening" concept. ORB is distinct because it uses a *fixed reference point* (the open) rather than a rolling lookback — capturing session-start momentum.

## Proposed Config Changes (config.rs)

```rust
// ── RUN229: Opening Range Breakout (ORB) ────────────────────────────────
// or_range_upper = highest high of first OR_BARS bars
// or_range_lower = lowest low of first OR_BARS bars
// LONG: price closes above or_range_upper AND holds for 1 bar
// SHORT: price closes below or_range_lower AND holds for 1 bar
// Use 15m bars: ORB = first 4 bars = first hour

pub const ORB_ENABLED: bool = true;
pub const ORB_BARS: usize = 4;              // number of bars for opening range
pub const ORB_CONFIRM_BARS: u32 = 1;        // must hold breakout for N bars
pub const ORB_SL: f64 = 0.005;
pub const ORB_TP: f64 = 0.004;
pub const ORB_MAX_HOLD: u32 = 24;           // ~6 hours at 15m
```

---

## Validation Method

1. **Historical backtest** (run229_1_orb_backtest.py)
2. **Walk-forward** (run229_2_orb_wf.py)
3. **Combined** (run229_3_combined.py)

## Out-of-Sample Testing

- OR_BARS sweep: 2 / 4 / 6 / 8
- CONFIRM_BARS sweep: 1 / 2
