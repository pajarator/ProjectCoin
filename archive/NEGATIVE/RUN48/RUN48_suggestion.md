# RUN48 — Z-Score Recovery Suppression: Anti-Chase Re-Entry Gate

## Hypothesis

**Named:** `z_recovery_suppression`

**Mechanism:** After a regime trade exits (via Z0, SMA, or partial reversion exit), the coin's z-score has typically reverted close to the mean. If another entry signal fires immediately after, the trader is effectively "chasing" the same coin — entering after the mean reversion has already largely completed. This is a common failure mode in mean-reversion systems.

The fix: track the z-score at which the last trade exited. For a `z_recent_exit` window (N bars), suppress re-entries on the same coin unless z has drifted back below a re-entry threshold (e.g., z < −1.0 again). This ensures the system waits for a fresh oversold condition rather than chasing a coin that just mean-reverted.

```
z_at_last_exit = stored at close_position time
re_entry_suppress_bars = Z_RECOVERY_SUPPRESS_BARS
re_entry_z_threshold = Z_RECOVERY_ENTRY_THRESHOLD  (e.g., z must be < -1.0 again to re-enter)

During suppress window:
  if z >= Z_RECOVERY_ENTRY_THRESHOLD: block entry
  if z < Z_RECOVERY_ENTRY_THRESHOLD: allow entry (fresh deviation detected)
After suppress window: normal entry logic resumes
```

**Why this is not a duplicate:**
- RUN39 (asymmetric cooldown) treated all exits the same for re-entry timing; this gates re-entry based on z-score recovery, not time
- RUN45 (complement exhaustion) gated scalp after complement; this gates all entries after any regime exit
- RUN46 (partial reversion) stores `z_at_entry`; this RUN stores `z_at_exit` and uses it to gate future entries
- No prior RUN tracked whether a coin has "already mean-reverted" as a gate for subsequent entries

---

## Proposed Config Changes

```rust
// RUN48: Z-Score Recovery Suppression parameters
pub const Z_RECOVERY_SUPPRESS_BARS: u32 = 6;    // suppress re-entry for 6 bars (~90 min) after exit
pub const Z_RECOVERY_ENTRY_THRESHOLD: f64 = -1.0; // z must drift back below this to re-enter during suppress
pub const Z_RECOVERY_SUPPRESS_MODE: u8 = 1;      // 0=disabled, 1=after_win_only, 2=after_any_exit
```

**`state.rs` — add `z_at_exit` and `z_recovery_bars` to CoinState:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub z_at_exit: Option<f64>,           // z-score at last close
    pub z_recovery_bars: u32,            // bars remaining in recovery suppression
}
```

**`engine.rs` — close_position stores z_at_exit:**
```rust
fn close_position(...) {
    // ... existing close logic ...
    cs.z_at_exit = Some(ind.z);  // record z at exit
    cs.z_recovery_bars = config::Z_RECOVERY_SUPPRESS_BARS;
}
```

**`engine.rs` — check_entry gates on recovery suppression:**
```rust
pub fn check_entry(state: &mut SharedState, ci: usize, mode: MarketMode, ctx: &MarketCtx) {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return; }

    // Z-recovery suppression gate
    if config::Z_RECOVERY_SUPPRESS_MODE > 0 {
        if cs.z_recovery_bars > 0 {
            if config::Z_RECOVERY_SUPPRESS_MODE == 1 {
                // After-win-only suppression: check if last trade was a win
                // (requires tracking last_trade_pnl in CoinState)
            }
            // Gate: only allow re-entry if z has drifted back below threshold
            if let Some(ref ind) = cs.ind_15m {
                if !ind.z.is_nan() && ind.z >= config::Z_RECOVERY_ENTRY_THRESHOLD {
                    return;  // coin has recovered too much, suppress
                }
            }
        }
    }
    // ... rest of existing check_entry logic ...
}

// At end of each tick, decrement recovery counter
for cs in &mut state.coins {
    if cs.z_recovery_bars > 0 {
        cs.z_recovery_bars -= 1;
    }
}
```

---

## Validation Method

### RUN48.1 — Z-Recovery Suppression Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no z-recovery suppression

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_RECOVERY_SUPPRESS_BARS` | [0, 4, 6, 8, 12, 16] |
| `Z_RECOVERY_ENTRY_THRESHOLD` | [-0.5, -0.75, -1.0, -1.25] |
| `Z_RECOVERY_SUPPRESS_MODE` | [1=after_win_only, 2=after_any_exit] |

`SUPPRESS_BARS = 0` means disabled (baseline).

**Per coin:** 6 × 4 × 2 = 48 configs × 18 coins = 864 backtests

**Key metrics:**
- `chase_rate`: % of re-entries where z was > -1.0 at re-entry time (these would be blocked by the filter)
- `false_block_rate`: % of blocked re-entries that would have been winners
- `PF_delta`: profit factor change vs baseline
- `re_entry_after_loss`: does the filter block bad re-entries after losses specifically?

**Also measure:** Is the filter more effective after wins vs after losses? If wins and losses both produce chasing behavior equally, MODE=2 (after_any_exit) works better. If chasing is primarily after wins, MODE=1 (after_win_only) is sufficient.

### RUN48.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `SUPPRESS_BARS × ENTRY_THRESHOLD × MODE` per coin
2. Test: evaluate on held-out month with those params

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- False block rate < 20% (filter doesn't block too many good opportunities)
- Portfolio OOS P&L ≥ baseline

### RUN48.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Z-Recovery Suppression | Delta |
|--------|---------------|----------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Chase Rate | X% | X% | −Ypp |
| False Block Rate | — | X% | — |
| Avg Bars After Exit | 0 | X | +N |

---

## Why This Could Fail

1. **Chasing isn't the problem:** Re-entries after z-recovery may actually be fine — the new entry has its own signal (z < -1.5) which independently justifies entry. Blocking it based on a prior trade's exit z doesn't improve outcomes.
2. **Suppression delays good entries:** During the suppress window, a genuinely new oversold condition may develop (z drops below -1.5 again). The filter correctly allows re-entry at z < threshold, but if z is between threshold and 0, it blocks even though a new entry signal (z < -1.5) may exist.
3. **Z-score is mean-reverting so recovery is natural:** The fact that z is near 0 after a winning trade is expected — it doesn't mean the next trade will fail.

---

## Why It Could Succeed

1. **Addresses the "mean-reversion stacking" problem:** When the same coin mean-reverts twice in a short period, the second re-entry is weaker because the first one has already extracted most of the edge. Blocking re-entry until a fresh deviation develops preserves quality.
2. **Reduces consecutive loss streaks:** RUN34 showed that consecutive SLs are a primary P&L leak. Many of these consecutive SLs come from re-entering too quickly after a loss. The z-recovery gate would block these premature re-entries.
3. **Pairs well with RUN46 (partial reversion exit):** After a partial reversion exit (which captures more of the move), z is even closer to the mean. A z-recovery suppression prevents immediately chasing the same coin at full value.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN48 Z-Recovery Suppression |
|--|--|--|
| Re-entry gate | None | z-score based |
| Post-exit tracking | Cooldown only | z_at_exit + recovery_bars |
| Chase prevention | None | Threshold-gated |
| Suppress window | 2 bars (cooldown) | 4–16 bars (configurable) |
| Entry after suppress | Always allowed | Only if z < -1.0 |
| Expected chase rate | Unmeasured | Targeted for reduction |
