# RUN39 — Asymmetric Win/Loss Cooldown: Consecutive-Loss Escalation Beyond ISO

## Hypothesis

**Named:** `win_loss_cooldown_asymmetry`

**Mechanism:** The current cooldown system in COINCLAW has two problems:

1. **Uniform cooldown:** Every non-SL close gets cooldown=2 bars. Every SL close gets cooldown=2 bars (first) or escalating cooldown (consecutive ≥2). The system does NOT distinguish between a win and a loss on non-SL closes.

2. **Win-then-loss vulnerability:** If a position exits profitably (SMA/Z0 signal) and the next bar produces an entry that immediately hits SL, the system treats this as two separate single SLs with cooldown=2 each — never triggering the escalation mechanism. But this is precisely the pattern that destroys P&L: the signal fires too early after a win, before the mean-reversion thesis has fully resolved.

The hypothesis is that **wins and losses should have different default cooldowns**:
- After a **win** (reason ≠ "SL"): allow faster re-entry (1 bar) since the market conditions that produced the win may persist
- After a **loss** (reason = "SL"): require longer cooldown (3 bars) since the position was wrong and market regime may have shifted

Additionally, **loss→loss sequences should escalate faster** than the current consecutive-SL-only trigger. The current ISO escalation (60 bars after 2 consecutive SLs) is slow to activate. A faster escalation (e.g., 4 bars after the second consecutive SL) would reduce the cascade damage.

**Why this is not a duplicate:**
- RUN34 tested ISO-specific SL widening and circuit breakers — a structural change to how ISO shorts operate
- RUN7 tested trailing stops and initial SL values — parameter changes to the stop mechanism
- This RUN changes the cooldown logic based on trade outcome sequence, which has never been tested
- The "win→loss" vulnerability is a specific failure mode not addressed by any prior RUN

---

## Proposed Config Changes

```rust
// RUN39: Asymmetric win/loss cooldown parameters
pub const WIN_COOLDOWN: u32 = 1;        // bars to wait after a winning exit (non-SL)
pub const LOSS_COOLDOWN: u32 = 3;        // bars to wait after a single SL hit
pub const CONSEC_LOSS_2_COOLDOWN: u32 = 6;  // bars after 2nd consecutive SL (replaces ISO escalation 60)
pub const CONSEC_LOSS_3_COOLDOWN: u32 = 12;  // bars after 3rd consecutive SL
```

**State change:** `state.rs` must track per-coin `last_trade_was_win: Option<bool>` and `consecutive_loss_streak: u32`.

```rust
// state.rs changes
pub struct CoinState {
    // ... existing fields ...
    pub last_trade_was_win: Option<bool>,  // None = no trades yet
    pub consecutive_loss_streak: u32,     // resets to 0 after any win
}

// close_position logic change:
if reason == "SL" {
    cs.consecutive_loss_streak += 1;
} else {
    cs.consecutive_loss_streak = 0;
}
// cooldown determination:
cs.cooldown = match cs.consecutive_loss_streak {
    0 if reason != "SL" => config::WIN_COOLDOWN,
    0 => config::LOSS_COOLDOWN,
    1 => config::LOSS_COOLDOWN,
    2 => config::CONSEC_LOSS_2_COOLDOWN,
    _ => config::CONSEC_LOSS_3_COOLDOWN,
};
```

---

## Validation Method

### RUN39.1 — Grid Search: Cooldown Parameters (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset (same as prior RUNs)

**Baseline:** Current COINCLAW v16 cooldown behavior:
- Non-SL close → cooldown = 2
- SL close → cooldown = 2 (first), then ISO_SL_ESCALATE_COOLDOWN=60 after consecutive ≥2

**Grid search parameters:**

| Parameter | Values |
|-----------|--------|
| `WIN_COOLDOWN` | [1, 2, 3] |
| `LOSS_COOLDOWN` | [2, 3, 4, 5] |
| `CONSEC_LOSS_2_COOLDOWN` | [4, 6, 8, 12] |
| `CONSEC_LOSS_3_COOLDOWN` | [8, 12, 16, 20] |

**Per coin:** 3 × 4 × 4 × 4 = 192 configs × 18 coins = 3,456 backtests

**Score on train (67%):** `PF × sqrt(trades) / max(max_dd, 1)`

**Compare:**
1. Best asymmetric cooldown config per coin vs baseline
2. ΔWR%, ΔPF, ΔP&L, trade count ratio
3. Specifically measure: how many win→loss cascades does the new system prevent?

**Key metric:** `cascade_prevention_rate = (baseline_win_then_loss_pairs − new_win_then_loss_pairs) / baseline_win_then_loss_pairs`

### RUN39.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `WIN_COOLDOWN × LOSS_COOLDOWN × CONSEC_LOSS_2 × CONSEC_LOSS_3` per coin
2. Test: evaluate with those params on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Portfolio OOS P&L ≥ baseline on test half
- No coin degrades by >40% in OOS vs baseline (universal params if needed)

### RUN39.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Asymmetric Cooldown | Delta |
|--------|---------------|---------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Cascade Rate | X% | X% | −Ypp |
| Avg Loss Streak | X.X | X.X | −Y.X |

**Cascade rate:** % of trades where the previous trade was a win AND this trade hit SL within 3 bars of entry.

---

## Why This Could Fail

1. **Cooldown doesn't change outcome:** The real problem isn't timing of re-entry — it's that the signal fired when the market wouldn't cooperate. A longer cooldown just delays the same bad trade.
2. **Fewer trades hurts more than quality helps:** If longer loss cooldown reduces trade count significantly, the P&L opportunity cost outweighs the improved win rate.
3. **Regime-specific:** The benefit may only appear in high-volatility periods where signals fire more frequently (and more incorrectly). In calm periods, the cooldown has little effect.

---

## Why It Could Succeed

1. **Win→loss cascade is the primary P&L destroyer** — the system makes money on individual trades but leaks it through rapid re-entry after wins
2. **Simple change, low risk:** It doesn't touch entry logic, stop loss, or take profit — only the timing of re-entry
3. **Scalable:** Same logic applies to scalp layer (where 1-bar vs 2-bar has larger effect due to frequency)

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN39 Asymmetric Cooldown |
|--|--|--|
| Win cooldown | 2 bars | 1 bar (faster re-entry) |
| Single loss cooldown | 2 bars | 3 bars (slower re-entry) |
| 2nd consecutive SL | 60 bars (ISO escalation only) | 4–12 bars (immediate) |
| 3rd+ consecutive SL | 60 bars | 8–20 bars |
| Tracks last trade outcome | No | Yes |
| Escalation trigger | Consecutive SLs only | Loss count after any close |
