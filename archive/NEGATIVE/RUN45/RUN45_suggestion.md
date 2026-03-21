# RUN45 — Complement-Scalp Mutual Exclusion + Exhaustion Timer

## Hypothesis

**Named:** `complement_scalp_exclusion`

**Mechanism:** COINCLAW has three independent trade layers:
1. **Regime trades** — primary mean-reversion entries (VwapRev, BbBounce, etc.)
2. **Complement trades** (RUN13) — secondary long entries (Laguerre RSI, Kalman Filter, KST Cross) that fire when the primary doesn't
3. **Scalp trades** — 1m mean-reversion overlays (vol_spike_rev, stoch_cross)

All three are trying to capture the same underlying mean-reversion opportunity. When a complement signal fires (e.g., Laguerre RSI crosses up from <20), the market is already at an oversold extreme. A scalp entry within the next few bars is likely to "double up" on the same trade — capturing a smaller slice of the same move without adding new information.

**The problem:** After a complement fires, the scalp signal may still trigger independently (on 1m stoch cross or vol spike) for the same coin. This wastes the scalp slot (MAX_SCALP_OPENS_PER_CYCLE = 3) on a redundant trade while another coin that could use the scalp slot goes unplayed.

**The fix:** After a complement long entry fires, set a `complement_exhaustion` counter (N bars) during which scalp entries for that coin are suppressed. This ensures scalp slots are used for genuinely independent opportunities.

```rust
// In state.rs CoinState:
pub complement_exhaustion: u32,  // bars remaining before scalp can fire

// In engine.rs after complement entry fires:
cs.complement_exhaustion = config::COMPLEMENT_EXHAUSTION_BARS;

// In check_scalp_entry:
if state.coins[ci].complement_exhaustion > 0 { return; }
```

**Why this is not a duplicate:**
- RUN12 (scalp market mode) fixed scalp direction alignment, not signal exclusivity
- RUN13 (complement signals) discovered the complement logic but never tested interaction with scalp
- No prior RUN tested whether multiple layers firing simultaneously helps or hurts
- This is about signal independence, not signal quality in isolation

---

## Proposed Config Changes

```rust
// RUN45: Complement-Scalp Exhaustion Timer
pub const COMPLEMENT_EXHAUSTION_BARS: u32 = 8;  // suppress scalp for 8 bars (~2h) after complement
pub const COMPLEMENT_EXHAUSTION_SCALP_ONLY: bool = true;  // only suppress scalp, not regime
```

**`state.rs` change:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub complement_exhaustion: u32,  // bars remaining; 0 = not exhausted
}
```

**`engine.rs` changes:**
1. After complement entry fires in `check_entry`:
   ```rust
   if strategies::complement_entry(...) {
       open_position(state, ci, ..., Direction::Long, TradeType::Regime);
       state.coins[ci].complement_exhaustion = config::COMPLEMENT_EXHAUSTION_BARS;
   }
   ```

2. In `check_scalp_entry`:
   ```rust
   pub fn check_scalp_entry(state: &mut SharedState, ci: usize) {
       if state.coins[ci].pos.is_some() { return; }
       if state.coins[ci].complement_exhaustion > 0 { return; }  // NEW
       // ... rest unchanged ...
   }
   ```

3. Decrement exhaustion counter each bar:
   ```rust
   // In the main tick loop (or coordinator), at end of each bar:
   for cs in &mut state.coins {
       if cs.complement_exhaustion > 0 {
           cs.complement_exhaustion -= 1;
       }
   }
   ```

---

## Validation Method

### RUN45.1 — Exhaustion Timer Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — scalp and complement fire independently with no interaction

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `COMPLEMENT_EXHAUSTION_BARS` | [0, 4, 8, 12, 16, 24] |
| `COMPLEMENT_EXHAUSTION_SCALP_ONLY` | [true, false] |

`EXHAUSTION_BARS = 0` means disabled (baseline behavior).

**Per coin:** 6 × 2 = 12 configs × 18 coins = 216 backtests

**Also test:** Is the effect different depending on which complement strategy fired? (Laguerre RSI vs Kalman Filter vs KST Cross have different time profiles — Laguerre is fastest, Kalman is slowest)

**Key metrics:**
- `scalp_redundancy_rate`: % of scalp entries that fire within N bars of a complement entry (same coin)
- `scalp_independence_rate`: % of scalp entries that have no complement entry in the preceding 20 bars
- `delta_total_PnL`: net P&L change vs baseline
- `delta_scalp_PnL`: scalp P&L change vs baseline (may be worse due to fewer scalp trades)

### RUN45.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best exhaustion timer per coin (or universal best)
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Scalp P&L delta ≥ 0 (exhaustion doesn't reduce scalp profits)
- Complement-scalp redundancy rate < 10% (effect is meaningful)

### RUN45.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Complement Exhaustion | Delta |
|--------|---------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Scalp P&L | $X | $X | +$X |
| Regime P&L | $X | $X | +$X |
| Total WR% | X% | X% | +Ypp |
| Scalp WR% | X% | X% | +Ypp |
| Scalp Trade Count | N | M | −K |
| Redundancy Rate | X% | X% | −Ypp |
| Avg Complement→Scalp Bars | X | X | — |

---

## Why This Could Fail

1. **Complement and scalp capture different things:** Complement fires on 15m Laguerre/Kalman/KST; scalp fires on 1m stoch/vol. They may both be valid signals on the same bar but capturing different timeframes of the same move — suppressing scalp doesn't improve outcomes, just reduces total P&L.
2. **Exhaustion timer is arbitrary:** The right number of bars is unknown. Too short = no effect; too long = misses scalp opportunities that are genuinely independent.
3. **Redundancy isn't the problem:** RUN29 showed scalp WR is ~10.9% — scalp is already losing money. The issue isn't redundancy, it's that scalp shouldn't be traded at all under realistic fee models (RUN37).

---

## Why It Could Succeed

1. **Scalp slots are limited:** With `MAX_SCALP_OPENS_PER_CYCLE = 3`, every redundant scalp trade that fires instead of an independent one is a missed opportunity. Using the exhaustion timer to filter redundant scalps could redirect scalp slots to coins that genuinely need them.
2. **Complement signals are strong:** RUN13 showed 62.7% WR on 640 complement trades (+14.2% portfolio P&L). Suppressing scalp after complement fires preserves the quality of the complement trade by preventing a conflicting scalp from interfering.
3. **Low implementation cost:** The change is small and self-contained — one new state field, one new config constant, two lines of logic in `check_scalp_entry`.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN45 Complement Exhaustion |
|--|--|--|
| Scalp after complement | Always allowed | Suppressed for N bars |
| Exhaustion tracking | None | complement_exhaustion counter |
| Scalp slot allocation | First-come-first-served | Complement-aware |
| Redundancy | Unmeasured | Targeted for reduction |
| Regime interaction | None | Scalp suppressed post-complement |
