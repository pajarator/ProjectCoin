# RUN73 — Dynamic Max Hold Based on Entry Z-Score: Extended Hold for Extreme Deviations

## Hypothesis

**Named:** `dynamic_max_hold`

**Mechanism:** The current `MOMENTUM_MAX_HOLD = 240` bars is a safety net that forces close after a fixed number of bars. But entry z-score extremity should determine how long we wait:
- Entry at `z = -2.5` (extreme): the deviation is large, mean reversion takes longer to complete → MAX_HOLD should be longer
- Entry at `z = -1.6` (moderate): the deviation is smaller, mean reversion resolves faster → MAX_HOLD can be shorter

This is the inverse of z-confidence sizing (RUN52) — instead of sizing up on extreme entries, we hold longer.

```
z_at_entry = stored
dynamic_max_hold = BASE_HOLD + (|z_at_entry| - Z_BASE) × HOLD_FACTOR
Example:
  BASE_HOLD = 200, HOLD_FACTOR = 40
  z=-1.6: max_hold = 200 + (1.6-1.5)×40 = 204 bars
  z=-2.5: max_hold = 200 + (2.5-1.5)×40 = 240 bars
  z=-3.0: max_hold = 200 + (3.0-1.5)×40 = 260 bars
```

**Why this is not a duplicate:**
- No prior RUN has made MAX_HOLD conditional on entry z-score
- RUN31 set SCALP_MAX_HOLD=480 bars as a safety net — this varies it dynamically
- Different from z-confidence sizing (RUN52) which changes position size, not holding time

---

## Proposed Config Changes

```rust
// RUN73: Dynamic Max Hold
pub const DYNAMIC_MAX_HOLD_ENABLE: bool = true;
pub const DYNAMIC_MAX_HOLD_BASE: u32 = 200;   // base max hold in bars
pub const DYNAMIC_MAX_HOLD_Z_BASE: f64 = 1.5; // reference z for base
pub const DYNAMIC_MAX_HOLD_FACTOR: u32 = 40;   // additional bars per 1σ of z
pub const DYNAMIC_MAX_HOLD_CAP: u32 = 320;    // absolute maximum bars
```

**`state.rs` — add z_at_entry to Position:**
```rust
pub struct Position {
    pub z_at_entry: Option<f64>,  // already added in RUN46
    // ... existing fields ...
}
```

**`engine.rs` — dynamic max hold in check_exit:**
```rust
fn effective_max_hold(cs: &CoinState, z_entry: f64) -> u32 {
    if !config::DYNAMIC_MAX_HOLD_ENABLE { return config::MOMENTUM_MAX_HOLD; }
    let z_abs = z_entry.abs();
    let base = config::DYNAMIC_MAX_HOLD_BASE as f64;
    let extra = ((z_abs - config::DYNAMIC_MAX_HOLD_Z_BASE) * config::DYNAMIC_MAX_HOLD_FACTOR as f64).max(0.0);
    (base + extra).min(config::DYNAMIC_MAX_HOLD_CAP as f64) as u32
}

// In check_exit, replace:
// if held >= config::MOMENTUM_MAX_HOLD
// With:
// if held >= effective_max_hold(cs, z_entry)
```

---

## Validation Method

### RUN73.1 — Dynamic Max Hold Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed MOMENTUM_MAX_HOLD = 240

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `DYNAMIC_MAX_HOLD_BASE` | [150, 200, 250] |
| `DYNAMIC_MAX_HOLD_Z_BASE` | [1.5, 2.0] |
| `DYNAMIC_MAX_HOLD_FACTOR` | [30, 40, 50] |
| `DYNAMIC_MAX_HOLD_CAP` | [280, 320, 360] |

**Per coin:** 3 × 2 × 3 × 3 = 54 configs × 18 coins = 972 backtests

**Key metrics:**
- `avg_held_bars_delta`: average increase in hold duration
- `z_correlation`: does z_at_entry correlate with optimal hold duration?
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline

### RUN73.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `BASE × Z_BASE × FACTOR` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Avg held bars increases for extreme z entries

### RUN73.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Dynamic Max Hold | Delta |
|--------|---------------|----------------|-------|
| Total P&L | $X | $X | +$X |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | -Ypp |
| Avg Held Bars | X | X | +N |
| Z=-1.5–2.0 Avg Hold | 240 | X | — |
| Z=-2.0–2.5 Avg Hold | 240 | X | — |
| Z>2.5 Avg Hold | 240 | X | — |

---

## Why This Could Fail

1. **Holding longer doesn't change outcomes:** If a trade is going to win, it wins regardless of how long we hold. If it's going to lose, it loses. MAX_HOLD is a safety net — extending it for extreme entries may not improve P&L.
2. **Optimal hold is determined by market structure, not z-score:** The right hold duration is determined by when the signal exits (SMA/Z0), not by how extreme the entry was.

---

## Why It Could Succeed

1. **Extreme deviations take longer to resolve:** Mechanically sound: z = -3.0 means price is very far from mean. It takes more bars to revert than z = -1.6.
2. **Prevents premature force-closes:** The fixed 240-bar MAX_HOLD may force-close extreme-deviation trades before they've had time to mean-revert. Dynamic hold lets them work.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN73 Dynamic Max Hold |
|--|--|--|
| Max hold | Fixed 240 bars | Z-score adaptive (200–320 bars) |
| Hold for z=-1.6 | 240 | ~204 |
| Hold for z=-2.5 | 240 | ~240 |
| Hold for z=-3.0 | 240 | ~260 |
| Avg held bars | X | +5–15 bars |
