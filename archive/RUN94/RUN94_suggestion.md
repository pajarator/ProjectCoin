# RUN94 — Partial Reentry After Cooldown: Re-Enter at Improved Price After Early Signal Reappearance

## Hypothesis

**Named:** `reentry_after_cooldown`

**Mechanism:** COINCLAW uses a cooldown period after each exit (typically 2 bars) to prevent overtrading. But sometimes a trade exits early (e.g., via Z0 or SMA) and the signal immediately fires again — the coin is still in the same mean-reversion setup. The current system ignores this because the coin is in cooldown. The Partial Reentry system allows a reduced-size re-entry during cooldown if the signal is stronger (more extreme z-score) than at the original entry.

**Partial Reentry After Cooldown:**
- After a position exits with reason `GOOD_EXIT` (Z0, SMA, BE, PARTIAL_T2/T3), enter cooldown as normal
- If during cooldown the signal fires again AND `z_at_reentry < z_at_original * REENTRY_Z_MULT` (e.g., z is more extreme):
  - Allow a PARTIAL re-entry at `REENTRY_SIZE_PCT` (e.g., 50%) of normal position size
  - This is a "second chance" at a better entry price with reduced risk
- Re-entry exits use normal exit logic
- Track `reentry_count` per coin to limit consecutive re-entries

**Why this is not a duplicate:**
- RUN48 (z_recovery suppress) suppressed entries when z recovered too quickly — this allows re-entry when z gets MORE extreme during cooldown
- RUN59 (same_dir suppress) suppressed same-direction signals after an exit — this specifically allows re-entry under better z conditions
- No prior RUN has allowed partial-size re-entry during cooldown when entry z-score is more extreme

**Mechanistic rationale:** A coin at z = -1.5 exits via Z0 (z crossed back to 0), and immediately drops back to z = -2.0. The original trade was right — the reversion wasn't complete. The cooldown prevented re-entry at -2.0, but the setup is actually better. The reentry_after_cooldown system allows a second bite at the apple at a better price, with half the normal risk.

---

## Proposed Config Changes

```rust
// RUN94: Partial Reentry After Cooldown
pub const REENTRY_ENABLE: bool = true;
pub const REENTRY_Z_MULT: f64 = 1.20;     // z must be this much more extreme vs entry z (e.g., 1.2× = -1.8 vs -1.5)
pub const REENTRY_SIZE_PCT: f64 = 0.50;   // re-entry at 50% of normal size
pub const REENTRY_MAX_COUNT: u32 = 2;      // max consecutive re-entries per coin per cycle
pub const REENTRY_ALLOWED_EXITS: &[&str] = &["Z0", "SMA", "BE", "PARTIAL_T2", "PARTIAL_T3"]; // only for good exits
```

**`state.rs` — CoinPersist additions:**
```rust
pub struct CoinPersist {
    // ... existing fields ...
    pub reentry_count: u32,
    pub original_z: Option<f64>,    // z at original entry (for comparison during cooldown)
}
```

**`engine.rs` — reentry_check in check_entry:**
```rust
/// Check if a coin in cooldown is eligible for partial re-entry.
fn check_reentry(state: &SharedState, ci: usize) -> bool {
    if !config::REENTRY_ENABLE { return false; }

    let cs = &state.coins[ci];

    // Must be in cooldown
    if cs.cooldown == 0 { return false; }

    // Must not exceed max re-entry count
    if cs.reentry_count >= config::REENTRY_MAX_COUNT { return false; }

    // Get original z from the previous trade
    let original_z = match cs.original_z {
        Some(z) => z,
        None => return false,
    };

    // Get current z
    let current_z = match &cs.ind_15m {
        Some(ind) => ind.z,
        None => return false,
    };

    // For long: current_z must be more negative than original_z * REENTRY_Z_MULT
    // e.g., original_z = -1.5, REENTRY_Z_MULT = 1.2 → threshold = -1.8
    // current_z = -1.9 < -1.8 → more extreme, allow re-entry
    if current_z >= original_z * config::REENTRY_Z_MULT {
        return false;  // not extreme enough
    }

    true
}

// In check_entry — modify cooldown check:
if cs.cooldown > 0 && !check_reentry(state, ci) {
    // Normal cooldown — don't enter
    state.coins[ci].cooldown -= 1;
    return;
}

// If re-entry allowed:
if check_reentry(state, ci) {
    // Open partial position
    let partial_risk = config::RISK * config::REENTRY_SIZE_PCT;
    let trade_amt = cs.bal * partial_risk;
    // ... open position with partial_risk ...
    state.coins[ci].reentry_count += 1;
    state.coins[ci].original_z = Some(current_z);  // update for potential next re-entry
}

// In close_position — record original z at entry:
if !cs.original_z.is_some() {
    cs.original_z = pos.z_at_entry;  // save for re-entry comparison
}
cs.reentry_count = 0;  // reset on new entry
```

---

## Validation Method

### RUN94.1 — Reentry Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no re-entry during cooldown

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `REENTRY_Z_MULT` | [1.1, 1.2, 1.5] |
| `REENTRY_SIZE_PCT` | [0.3, 0.5, 0.7] |
| `REENTRY_MAX_COUNT` | [1, 2] |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `reentry_rate`: % of cooldown bars where re-entry occurs
- `reentry_win_rate`: win rate of re-entry trades
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN94.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best REENTRY_Z_MULT × SIZE_PCT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Re-entry win rate ≥ 50% (profitable on average)
- Re-entry rate 5–20% of cooldown bars

### RUN94.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no re-entry) | Reentry After Cooldown | Delta |
|--------|----------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Re-entry Rate | 0% | X% | — |
| Re-entry Win Rate | — | X% | — |
| Avg Re-entry Z | — | X | — |
| Re-entry Avg PnL | — | $X | — |

---

## Why This Could Fail

1. **Re-entering during cooldown overrides a safety mechanism:** Cooldowns exist to prevent overtrading in noisy, choppy conditions. Re-entering during cooldown defeats that protection.
2. **A more extreme z-score doesn't guarantee a win:** The coin dropped further after the exit — but it might drop even more (worse entry). The re-entry could be a worse entry than the original.
3. **Size reduction may not compensate:** A 50% position size on a re-entry that still loses may not generate enough profit to justify the complexity.

---

## Why It Could Succeed

1. **Captures "unfinished business":** Some mean reversion trades exit via Z0/SMA but the reversion isn't complete. The coin drops back to an extreme level within a few bars. Re-entry captures this opportunity.
2. **Second entry at better price:** When z drops from -1.5 to -2.0 during cooldown, we're re-entering at a more extreme price — better entry, better risk-reward.
3. **Size control limits downside:** Using 50% position size means the re-entry can't cause catastrophic losses even if the trade fails.
4. **Good-exit only:** Only re-enters after Z0/SMA/BE exits (good exits), not after SL or MAX_HOLD. This ensures we're re-entering after successful trades, not chasing losses.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN94 Reentry After Cooldown |
|--|--|--|
| Cooldown behavior | No entries until cooldown expires | No entries, EXCEPT if z more extreme |
| Re-entry size | N/A | 50% of normal |
| Entry condition | After cooldown | During cooldown if z more extreme |
| Max re-entries | 0 | 2 per cycle |
| Entry quality | Full cooldown | Only when better z available |
