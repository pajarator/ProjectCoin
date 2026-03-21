# RUN60 — Z-Score Momentum Filter: Confirming Direction of Indicator Deterioration

## Hypothesis

**Named:** `z_momentum_filter`

**Mechanism:** COINCLAW entries are triggered when z-score crosses a threshold (e.g., `z < -1.5` for longs). However, this tells us only that price is far from mean — not whether the oversold condition is *still worsening* or already reversing.

Consider two scenarios at entry:
1. `z = -2.0`, `z[prev bar] = -1.8` — z is still falling (worsening) — the oversold condition is intensifying, not reversing
2. `z = -2.0`, `z[prev bar] = -2.3` — z is rising (improving) — price has already begun reverting toward the mean

Scenario 2 is a better mean-reversion entry because the reversion has already started. Scenario 1 means you're entering *before* reversion begins, which may lead to more SL hits as the oversold condition continues briefly before reversing.

**Z-momentum filter:**
```
z_delta = z_current - z_previous_bar
For LONG entry: z_delta > Z_MOMENTUM_THRESHOLD  (z is rising toward mean)
For SHORT entry: z_delta < -Z_MOMENTUM_THRESHOLD  (z is falling from mean)
```

**Why this is not a duplicate:**
- All prior entry filters use the *level* of the indicator (z = -1.5, RSI < 30) — not its *direction*
- No prior RUN tested whether the indicator is still moving toward or away from the entry signal at the moment of entry
- This is a second-order filter (rate of change of the signal) applied to entry timing

---

## Proposed Config Changes

```rust
// RUN60: Z-Score Momentum Filter
pub const Z_MOMENTUM_ENABLE: bool = true;
pub const Z_MOMENTUM_THRESHOLD: f64 = 0.05;  // z must be rising/falling by ≥0.05 per bar
pub const Z_MOMENTUM_LOOKBACK: u8 = 1;        // 1 = previous bar, 2 = 2-bar momentum
```

**`indicators.rs` — add z_delta to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub z_delta: f64,        // z_current - z_previous
    pub z_delta_2b: f64,    // z_current - z_2bars_ago (2-bar momentum)
}
```

**`strategies.rs` — add z-momentum check to long_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // Z-momentum filter: only enter if z is recovering (not still falling)
    if config::Z_MOMENTUM_ENABLE {
        let z_delta = match config::Z_MOMENTUM_LOOKBACK {
            1 => ind.z_delta,
            _ => ind.z_delta_2b,
        };
        if z_delta <= config::Z_MOMENTUM_THRESHOLD {
            // z is still falling or flat — block entry
            return false;
        }
    }

    // ... rest of entry logic unchanged ...
}
```

For SHORT entry, block if `z_delta < -Z_MOMENTUM_THRESHOLD` (z is still rising / not yet falling).

---

## Validation Method

### RUN60.1 — Z-Momentum Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no z-momentum filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_MOMENTUM_THRESHOLD` | [0.02, 0.05, 0.10, 0.15] |
| `Z_MOMENTUM_LOOKBACK` | [1, 2] |

**Per coin:** 4 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `momentum_block_rate`: % of entries blocked by z-momentum filter
- `false_block_rate`: % of blocked entries that would have been winners
- `correct_block_rate`: % of blocked entries that would have been losers
- `WR_delta`: win rate change vs baseline among non-blocked entries
- `PF_delta`: profit factor change vs baseline

**Also measure:** Does z-momentum interact with strategy type? VwapReversion (crossing VWAP) might be more momentum-sensitive than AdrReversal (price reaching ADR band).

### RUN60.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `Z_MOMENTUM_THRESHOLD` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- False block rate < 25%
- Portfolio OOS P&L ≥ baseline

### RUN60.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Z-Momentum Filter | Delta |
|--------|---------------|------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Momentum Block Rate | 0% | X% | — |
| False Block Rate | — | X% | — |
| Correct Block Rate | — | X% | — |
| Avg Z-Delta at Entry | X | X | +Y |

---

## Why This Could Fail

1. **Z-score momentum can reverse quickly:** By the time z has started rising (recovering), the best entry point has already passed. The filter may block entries at exactly the right time.
2. **Z-score is noisy:** On 15m bars, z_delta can fluctuate due to noise rather than genuine momentum. A threshold of 0.05 may be too arbitrary.
3. **Holding through z-momentum continuation:** If z is still falling at entry but will recover in the next 2-3 bars, blocking the entry loses the trade opportunity unnecessarily.

---

## Why It Could Succeed

1. **Sound mechanics:** Entering after z has started recovering means the trade has already begun working. The first few bars after entry are more likely to be profitable, reducing SL hits.
2. **Second-order filter is novel:** All other filters are on first-order values (levels). Z-momentum is the first second-order filter — filtering on the rate of change of the signal itself.
3. **Computation is trivial:** z_delta is a simple subtraction. No new data or complex calculations required.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN60 Z-Momentum Filter |
|--|--|--|
| Entry filter | z-level only | z-level + z-momentum |
| Z confirmation | None | z rising ≥ 0.05/bar |
| Direction check | None | z must be moving toward mean |
| Entry timing | At threshold cross | At threshold + momentum confirm |
| Expected block rate | 0% | 15–30% |
| Expected WR% | ~38% | +2–5pp |
| Implementation | strategies.rs | strategies.rs + indicators.rs |
