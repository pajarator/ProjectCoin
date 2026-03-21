# RUN85 — Momentum Pulse Filter: Short-Term Momentum Alignment for Regime Entries

## Hypothesis

**Named:** `momentum_pulse_filter`

**Mechanism:** COINCLAW currently uses the F6 filter (`dir_roc_3 < -0.195` = counter-momentum) to block entries when price has moved against our direction. But F6 is a negative filter (it blocks bad entries) — it doesn't tell us when there is POSITIVE momentum alignment in our direction. A coin that is oversold AND has started bouncing (positive short-term momentum) is a better setup than one that is oversold but still grinding lower.

**Momentum Pulse Filter:**
- For LONG entries: require `roc_3 >= MOMENTUM_PULSE_MIN` (e.g., +0.05%) — price must be bouncing, not still falling
- For SHORT entries: require `roc_3 <= -MOMENTUM_PULSE_MIN` — price must be pulling back, not still rising
- This complements F6: F6 blocks counter-momentum entries (price moving against us), Momentum Pulse requires some momentum WITH our direction
- Threshold is lower than F6's counter-momentum threshold — we just want "not falling" for longs, not a full reversal

**Why this is not a duplicate:**
- RUN40 (BTC_DOM_SCALE) gates scalp entries using btc_z − avg_z spread — this gates REGIME entries using ROC_3 in the coin's own direction
- RUN61 (RSI recovery threshold) uses RSI divergence — this uses PRICE momentum (ROC_3), not RSI
- F6 filter (RUN10) is a negative gate (blocks entries where price moved aggressively against our direction) — this is a positive requirement (price must be moving WITH our direction, not just not against it)
- No prior RUN has required pro-direction short-term momentum as an entry condition for regime trades

**Mechanistic rationale:** The best mean reversion entries are at extreme oversold/overbought readings where the bounce or pullback has already begun. A coin at z = -2.0 where `roc_3 = +0.2%` has started bouncing — the mean reversion has momentum behind it. A coin at z = -2.0 where `roc_3 = -0.3%` is still falling — the mean reversion hasn't started yet. Momentum Pulse filters for the former.

---

## Proposed Config Changes

```rust
// RUN85: Momentum Pulse Filter
pub const MOMENTUM_PULSE_ENABLE: bool = true;
pub const MOMENTUM_PULSE_LONG_MIN: f64 = 0.0005;   // min 3-bar ROC for LONG entries (0.05%)
pub const MOMENTUM_PULSE_SHORT_MAX: f64 = -0.0005;  // max 3-bar ROC for SHORT entries (-0.05%)
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }
    // F6 already blocks counter-momentum entries
    // RUN85: Momentum Pulse — require positive short-term momentum
    if config::MOMENTUM_PULSE_ENABLE {
        if !ind.roc_3.is_nan() && ind.roc_3 < config::MOMENTUM_PULSE_LONG_MIN {
            return false;  // price still falling — wait for bounce
        }
    }
    match strat {
        // ... rest unchanged ...
    }
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }
    // F6 already blocks counter-momentum entries
    // RUN85: Momentum Pulse — require negative short-term momentum
    if config::MOMENTUM_PULSE_ENABLE {
        if !ind.roc_3.is_nan() && ind.roc_3 > config::MOMENTUM_PULSE_SHORT_MAX {
            return false;  // price still rising — wait for pullback
        }
    }
    match strat {
        // ... rest unchanged ...
    }
}
```

---

## Validation Method

### RUN85.1 — Momentum Pulse Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — F6 filter only, no momentum pulse

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MOMENTUM_PULSE_LONG_MIN` | [0.0003, 0.0005, 0.0008, 0.0010] |
| `MOMENTUM_PULSE_SHORT_MAX` | [-0.0003, -0.0005, -0.0008, -0.0010] |

**Per coin:** 4 × 4 = 16 configs × 18 coins = 288 backtests

**Key metrics:**
- `pulse_filter_rate`: % of regime entries blocked by momentum pulse filter
- `roc3_at_filtered_entries`: average roc_3 of blocked entries (should be negative for LONG, positive for SHORT)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN85.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best momentum pulse threshold per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Filter rate 10–40% (meaningful filtering without over-suppressing)
- Filtered-out trades have lower WR than filtered-in trades

### RUN85.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, F6 only) | Momentum Pulse Filter | Delta |
|--------|----------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg ROC3 at LONG Entry | X | X | +N |
| Avg ROC3 at SHORT Entry | X | X | +N |
| Filtered Entry WR% | — | X% | — |
| Filtered Entry Avg PnL | — | $X | — |

---

## Why This Could Fail

1. **Mean reversion doesn't need momentum to work:** The entire premise of COINCLAW's regime trades is that price reverts to the mean regardless of short-term momentum. Requiring positive momentum before entry may filter out the most extreme (and most profitable) mean reversion setups — exactly when z-score is most extreme but price hasn't bounced yet.
2. **Momentum is a lagging indicator:** By the time 3-bar momentum turns positive, the best entry opportunity (at the extreme) may have passed. This could reduce edge by entering later.
3. **F6 is already doing the heavy lifting:** F6 already blocks entries where counter-momentum is strong. Adding a positive momentum requirement may be redundant for most bad entries that F6 already catches.

---

## Why It Could Succeed

1. **Captures the bounce, not the fall:** The best mean reversion entries are when price has already started reversing. The F6 filter only blocks bad entries — Momentum Pulse captures the additional edge of entering when the bounce has begun.
2. **Filters false reversals:** A coin at z = -1.5 with `roc_3 = -0.3%` is still falling. Entering here is catching a falling knife. Momentum Pulse blocks this.
3. **Complementary to F6:** F6 is a negative gate (don't enter when price moved against us). Momentum Pulse is a positive gate (enter when price is moving with us). Together they form a stronger entry filter than either alone.
4. **Simple and fast:** ROC_3 is already computed. One additional comparison per entry check. No new data.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN85 Momentum Pulse Filter |
|--|--|--|
| Entry filter | F6 (counter-momentum block only) | F6 + Momentum Pulse (positive momentum required) |
| LONG entry | z < -1.5, vol confirmed, not counter-momentum | z < -1.5, vol confirmed, not counter-momentum, roc_3 ≥ +0.05% |
| SHORT entry | z > +1.5, vol confirmed, not counter-momentum | z > +1.5, vol confirmed, not counter-momentum, roc_3 ≤ -0.05% |
| Filter logic | Negative (blocks bad) | Negative + Positive (blocks bad + requires good) |
| Implementation | 1 comparison | 2 comparisons |
