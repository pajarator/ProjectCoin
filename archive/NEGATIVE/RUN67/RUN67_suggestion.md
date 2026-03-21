# RUN67 — Scalp Entry Z-Score Threshold Tightening: More Selective Scalp Entries

## Hypothesis

**Named:** `scalp_tighter_z_threshold`

**Mechanism:** Scalp trades use the same entry indicators as regime trades (z-score, RSI, volume) but with a much tighter SL (0.1% vs 0.3%). A scalp entry at `z = -1.5` (minimum threshold) gives very little room — if the market doesn't immediately revert, the 0.1% SL is hit quickly.

The hypothesis is that **scalp entries should require more extreme oversold/overbought conditions** than regime entries:
```
Scalp LONG entry: z < SCALP_Z_ENTRY  (e.g., -2.0 instead of -1.5)
Scalp SHORT entry: z > SCALP_Z_ENTRY  (e.g., +2.0 instead of +1.5)
```

The tighter threshold:
1. Ensures the scalp enters at a more extreme deviation, increasing the probability of a quick reversion
2. Reduces total scalp trade count (fewer but higher-quality scalp signals)
3. Improves scalp win rate by filtering out marginal setups that hit the tight SL

**Why this is not a duplicate:**
- No prior RUN has tightened scalp-specific entry thresholds
- Scalp uses SCALP_RSI_EXTREME (20) but not a separate SCALP_Z_THRESHOLD
- This is layer-specific threshold optimization — regime vs scalp have different risk profiles

---

## Proposed Config Changes

```rust
// RUN67: Scalp Tighter Z-Score Threshold
pub const SCALP_Z_ENTRY: f64 = 2.0;   // scalp must have |z| >= 2.0 (vs -1.5 for regime)
```

**`strategies.rs` — scalp_entry_with_price uses tighter z threshold:**
```rust
pub fn scalp_entry_with_price(ind: &Ind1m, price: f64) -> Option<(Direction, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }

    // ... existing scalp strategy checks (vol_spike_rev, stoch_cross, bb_squeeze) ...

    // Scalp-specific Z-score gate (more extreme than regime)
    if config::SCALP_TIGHTER_Z_GATE {
        if ind.z.is_nan() { return None; }
        match dir {
            Direction::Long => {
                if ind.z > -config::SCALP_Z_ENTRY { return None; }  // not oversold enough
            }
            Direction::Short => {
                if ind.z < config::SCALP_Z_ENTRY { return None; }   // not overbought enough
            }
        }
    }
    Some((dir, strat_name))
}
```

---

## Validation Method

### RUN67.1 — Scalp Z-Threshold Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 1m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — scalp uses strategy-specific thresholds, no separate scalp z-threshold

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SCALP_Z_ENTRY` | [1.5, 1.75, 2.0, 2.25, 2.5] |

5 configs × 18 coins = 90 backtests

**Key metrics:**
- `scalp_block_rate`: % of scalp entries blocked by tighter z-threshold
- `WR_delta`: scalp win rate change vs baseline among non-blocked entries
- `PF_delta`: scalp profit factor change vs baseline
- `total_scalp_PnL_delta`: net P&L change vs baseline
- `avg_scalp_z_at_entry`: average z-score at entry (should move toward SCALP_Z_ENTRY)

### RUN67.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `SCALP_Z_ENTRY` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS scalp P&L delta vs baseline
- Scalp WR% improves ≥ 3pp vs baseline
- Scalp P&L ≥ baseline despite fewer trades

### RUN67.3 — Combined Comparison

Side-by-side scalp trades:

| Metric | Baseline Scalp (v16) | Tighter Z-Threshold | Delta |
|--------|---------------------|---------------------|-------|
| Scalp WR% | X% | X% | +Ypp |
| Scalp PF | X.XX | X.XX | +0.XX |
| Scalp Total P&L | $X | $X | +$X |
| Scalp Trade Count | N | M | −K |
| Scalp Block Rate | 0% | X% | — |
| Avg Z at Entry | X | X | +Y |
| Scalp Avg Win | $X | $X | +$X |
| Scalp Avg Loss | $X | $X | $X |

---

## Why This Could Fail

1. **Fewer scalp trades = less P&L:** Scalp trades are already small winners (+0.8% TP vs -0.1% SL). Even with higher WR%, fewer trades may mean lower total P&L.
2. **Scalp strategies don't use z-score primarily:** vol_spike_rev uses RSI and volume; stoch_cross uses stoch. A tighter z-threshold may not significantly filter these strategies.
3. **Optimal z-threshold may be coin-specific:** BTC and ETH have different z-score distributions than smaller altcoins. A single threshold for all coins may not work.

---

## Why It Could Succeed

1. **Sound risk management:** Tight SL (0.1%) should require more extreme entry conditions to compensate for the reduced margin of error.
2. **Addresses scalp's main weakness:** Scalp WR is ~10.9% (RUN29) — way below breakeven. Filtering to only extreme entries should raise WR substantially.
3. **Complementary to RUN35 (breakeven):** Breakeven locks in gains; tighter z-entry prevents bad entries. Together they address both sides of scalp's problem.

---

## Comparison to Baseline

| | Current Scalp (v16) | RUN67 Tighter Z-Threshold |
|--|--|--|
| Scalp z-entry | Strategy-specific | z ≥ 2.0 (extreme) |
| Scalp WR% | ~11% | +5–10pp |
| Scalp PF | ~0.5 | +0.10–0.20 |
| Scalp Trade Count | N | N × (1 − block_rate) |
| Block Rate | 0% | 30–50% |
| Avg Z at Entry | ~-1.7 | ~-2.1 |
