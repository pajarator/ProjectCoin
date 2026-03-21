# RUN59 — Same-Direction Consecutive Signal Suppression: Anti-Doubling Down Filter

## Hypothesis

**Named:** `same_dir_suppression`

**Mechanism:** After a regime trade hits SL (loss), if the market conditions that triggered the entry haven't changed, the next signal in the same direction is likely to also fail. The current system re-enters after cooldown=2 bars regardless of whether the prior trade won or lost, and regardless of whether the next signal is in the same direction.

The hypothesis is that **after a loss, suppress re-entries in the same direction until either:**
1. A trade in the *opposite* direction resolves (confirming the market has reversed)
2. The signal reaches a *stronger* threshold than the initial entry

**The specific failure mode:**
- Entry fires at `z < -1.5` → price doesn't revert → hits SL
- 2 bars later, same conditions: `z < -1.5` again → re-entry fires → hits SL again
- This can repeat 3-4 times before the market finally reverses

**Fix:**
```rust
// After a loss (SL exit):
if reason == "SL":
    cs.last_loss_dir = Some(current_direction)  // "long" or "short"
    cs.same_dir_suppress = config::SAME_DIR_SUPPRESS_BARS  // e.g., 6 bars

// During suppression:
if cs.same_dir_suppress > 0 AND entry_direction == cs.last_loss_dir:
    return false  // block — don't double down

// Opposite direction always allowed (market may have reversed)
if opposite_dir_entry_fires:
    cs.same_dir_suppress = 0  // reset suppression
```

**Why this is not a duplicate:**
- RUN39 (asymmetric cooldown) gated by win/loss outcome but not by direction sequence
- RUN48 (z-recovery suppression) gated by z-level, not by direction of prior loss
- RUN34 (ISO escalation) suppressed after consecutive SLs but didn't distinguish direction
- This is specifically about avoiding re-entering in the *same direction* after a loss

---

## Proposed Config Changes

```rust
// RUN59: Same-Direction Consecutive Signal Suppression
pub const SAME_DIR_SUPPRESS_ENABLE: bool = true;
pub const SAME_DIR_SUPPRESS_BARS: u32 = 6;  // suppress same-dir re-entry for 6 bars (~90 min) after loss
pub const SAME_DIR_SUPPRESS_MODE: u8 = 1;   // 1=after_SL_only, 2=after_any_loss (including Z0/SMA at negative PnL)
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub last_loss_dir: Option<String>,       // "long" or "short" — direction that hit SL
    pub same_dir_suppress: u32,             // bars remaining in same-direction suppression
}
```

**`engine.rs` — close_position tracks direction loss:**
```rust
fn close_position(...) {
    // ... existing logic ...
    if reason == "SL" && trade_type == TradeType::Regime {
        state.coins[ci].last_loss_dir = Some(pos.dir.clone());
        state.coins[ci].same_dir_suppress = config::SAME_DIR_SUPPRESS_BARS;
    }
}
```

**`engine.rs` — check_entry gates on same-direction suppression:**
```rust
pub fn check_entry(...) {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return; }

    // Same-direction suppression gate
    if config::SAME_DIR_SUPPRESS_ENABLE && cs.same_dir_suppress > 0 {
        if let Some(ref last_dir) = cs.last_loss_dir {
            // Determine current entry direction
            let entry_dir = /* infer from mode and signal */;
            if entry_dir == *last_dir {
                return;  // blocked — don't re-enter same direction after a loss
            }
        }
    }
    // ... rest of check_entry ...
}

// Decrement suppression counter each bar
for cs in &mut state.coins {
    if cs.same_dir_suppress > 0 {
        cs.same_dir_suppress -= 1;
    }
}
```

---

## Validation Method

### RUN59.1 — Same-Direction Suppression Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no same-direction suppression

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SAME_DIR_SUPPRESS_BARS` | [4, 6, 8, 12] |
| `SAME_DIR_SUPPRESS_MODE` | [1=after_SL_only, 2=after_any_loss] |

**Per coin:** 4 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `same_dir_reentry_rate`: % of SL exits followed by a same-direction re-entry within N bars (baseline)
- `suppression_effectiveness`: % of suppressed re-entries that would have been losing trades
- `false_suppress_rate`: % of suppressed re-entries that would have been winners
- `consecutive_SL_rate_delta`: change in rate of 2+ consecutive SLs in same direction
- `PF_delta`: profit factor change vs baseline

### RUN59.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `SUPPRESS_BARS × MODE` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Consecutive SL rate reduces by > 20% vs baseline
- Portfolio OOS P&L ≥ baseline

### RUN59.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Same-Dir Suppression | Delta |
|--------|---------------|---------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Same-Dir Re-Entry Rate | X% | X% | −Ypp |
| Consecutive SL Rate | X% | X% | −Ypp |
| False Suppress Rate | — | X% | — |
| Suppression Effectiveness | — | X% | — |

---

## Why This Could Fail

1. **Suppressing after SL is too late:** The damage is done when the SL hits. Suppressing the next entry doesn't recover the loss and may prevent a genuinely good re-entry opportunity.
2. **Opposite direction may also fail:** If the market is choppy, both directions can fail. Suppressing LONG after a LONG loss and entering SHORT may hit the same SL in the other direction.
3. **The threshold matters:** If the re-entry signal is the same (z < -1.5), suppressing it after a loss and waiting 6 bars is arbitrary. The right fix would be to require a stronger signal after a loss, not just wait.

---

## Why It Could Succeed

1. **Targets the specific failure mode:** Consecutive SLs in the same direction are a clear pattern that destroys P&L. Blocking same-direction re-entries directly addresses this.
2. **Simple and low-risk:** Suppressing one direction for 6 bars after a loss only costs opportunity if the suppressed re-entry would have won — and those are hard to distinguish from the ones that would have lost.
3. **Complements RUN39 cooldown:** Where RUN39 differentiates win vs loss cooldowns, this differentiates loss direction sequences. Together they form a more nuanced re-entry policy.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN59 Same-Direction Suppression |
|--|--|--|
| Re-entry after loss | Always allowed (2-bar cooldown) | Blocked if same direction for N bars |
| Direction tracking | None | last_loss_dir tracked |
| Consecutive SL prevention | None | Targeted |
| After SL | cooldown = 2 | cooldown = 2 + same_dir suppress = 6+ |
| Opposite direction | Allowed after 2 bars | Always allowed immediately |
