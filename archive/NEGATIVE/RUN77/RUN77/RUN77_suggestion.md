# RUN77 — Z-Score Recovery Rate Exit: Exit When Mean Reversion Is Stalling

## Hypothesis

**Named:** `recovery_rate_exit`

**Mechanism:** COINCLAW regime trades assume that when a coin's z-score is extreme (e.g., z = -2.0), mean reversion will occur — price will recover toward the mean. The current exit logic checks IF mean reversion happened (SMA crossback, Z0 cross), but doesn't check HOW FAST it's happening. The problem: slow, grinding mean reversion often reverses. If a coin takes too many bars to revert, it signals the reversion signal is weak.

**Z-Score Recovery Rate Exit:**
- At entry, record `z_at_entry` and `bars_held = 0`
- Each bar, compute: `recovery_velocity = (z_at_entry - current_z) / bars_held`
- If `bars_held > N` AND `recovery_velocity < MIN_RECOVERY_VELOCITY` → force close
- `MIN_RECOVERY_VELOCITY` is a threshold: e.g., `0.02` means at least 0.02 z-units per bar of improvement are required
- Example: z_entry = -2.0, min_vel = 0.02 → after 30 bars, we need z ≤ -1.4 to still be in the trade. If z = -1.3, velocity = (-2.0 - (-1.3))/30 = -0.023 < -0.02 → exit

**Why this is not a duplicate:**
- RUN46 (partial reversion exit) exits at fixed PnL tiers — this exits based on RATE of z-score recovery, not absolute PnL
- RUN73 (dynamic max hold) extends hold time based on entry z — this SHORTENS hold when reversion is too slow
- No prior RUN has used a velocity-based exit for mean reversion trades

**Mechanistic rationale:** Crypto mean reversion tends to be fast and sharp (short, violent reversals) or false (grinding then continuing against). Slow grinding with insufficient recovery is a warning sign that the position is wrong. Exiting on slow recovery prevents holding through deteriorating setups and frees capital for better opportunities.

---

## Proposed Config Changes

```rust
// RUN77: Z-Score Recovery Rate Exit
pub const RECOVERY_RATE_EXIT_ENABLE: bool = true;
pub const RECOVERY_MIN_BARS: u32 = 20;         // minimum bars before enforcement kicks in
pub const RECOVERY_MIN_VELOCITY: f64 = 0.015;  // minimum z-units recovered per bar (e.g., 0.015 = 1.5 z-units per 100 bars)
pub const RECOVERY_GRACE_BARS: u32 = 10;        // grace period after entry (allow slow start)
```

**`state.rs` — Position changes:**
```rust
pub struct Position {
    // ... existing fields ...
    pub z_at_entry: Option<f64>,  // already added in RUN46
}
```

**`engine.rs` — recovery_rate_exit in check_exit:**
```rust
/// Check if a regime position should exit due to slow mean reversion.
fn check_recovery_rate_exit(cs: &CoinState, ind: &Ind15m, pos: &Position) -> bool {
    if !config::RECOVERY_RATE_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }

    let z_entry = match pos.z_at_entry {
        Some(z) => z,
        None => return false,  // no z tracked, skip
    };

    let held = cs.candles_held;
    if held < config::RECOVERY_MIN_BARS { return false; }
    if held < config::RECOVERY_GRACE_BARS { return false; }  // grace period

    let effective_held = held - config::RECOVERY_GRACE_BARS;
    let z_current = ind.z;

    // recovery velocity: how many z-units recovered per bar
    // For long: z_entry < 0, we want z to move toward 0 (increase)
    // velocity = (z_entry - z_current) / effective_held  [positive = improving]
    let recovery_velocity = (z_entry - z_current) / effective_held as f64;

    // For short: z_entry > 0, we want z to decrease
    // velocity_short = (z_current - z_entry) / effective_held  [positive = improving]
    let recovery_vel = if pos.dir == "long" {
        recovery_velocity
    } else {
        -(recovery_velocity)  // flip sign for shorts
    };

    if recovery_vel < config::RECOVERY_MIN_VELOCITY {
        return true;  // exit: reversion is too slow
    }

    false
}

// In check_exit — add before MAX_HOLD check:
if check_recovery_rate_exit(cs, &ind, &pos) {
    close_position(state, ci, price, "SLOW_REC", TradeType::Regime);
    return true;
}
```

---

## Validation Method

### RUN77.1 — Recovery Rate Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no recovery rate exit

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `RECOVERY_MIN_BARS` | [15, 20, 30] |
| `RECOVERY_MIN_VELOCITY` | [0.010, 0.015, 0.020, 0.025] |
| `RECOVERY_GRACE_BARS` | [5, 10, 15] |

**Per coin:** 3 × 4 × 3 = 36 configs × 18 coins = 648 backtests

**Key metrics:**
- `recovery_exit_rate`: % of regime trades exited by recovery rate (vs other exits)
- `slow_exit_PF`: profit factor of trades exited by slow recovery
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_held_delta`: change in average hold duration

### RUN77.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `MIN_BARS × MIN_VELOCITY × GRACE_BARS` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Recovery exit rate 5–30% of all regime trades (meaningful but not dominant)
- Slow-exit trades that were winners: check if exiting early captured profits vs holding

### RUN77.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Recovery Rate Exit | Delta |
|--------|---------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Held Bars | X | X | -N |
| Recovery Exits | 0 | X% | — |
| Slow Exit WR% | — | X% | — |
| Slow Exit Avg PnL | — | $X | — |

---

## Why This Could Fail

1. **Slow recovery doesn't mean wrong direction:** A coin can drift sideways for 50 bars then suddenly reverse sharply. Exiting at the first sign of slowness could cut short a valid mean reversion setup that was simply delayed.
2. **Z-score is mean-reverting by definition:** The mathematical nature of z-score (reverting to 0) means that eventually it WILL recover regardless of market conditions. The question is whether waiting is better than exiting and re-entering later.
3. **Optimal velocity threshold is regime-dependent:** In StrongTrend regimes, mean reversion is slower by definition. A fixed velocity threshold may be too aggressive in trending markets.

---

## Why It Could Succeed

1. **Prevents hold-through-reversal:** The worst regime trades are the ones that drift to -2.0 z, slowly grind to -1.5, then reverse and hit SL at -2.3. Slow recovery is often a leading indicator of an invalid setup.
2. **Frees capital for re-entry:** Exiting a slow-moving position frees $100 to re-enter when conditions improve. The re-entry z-score may be more extreme (better setup).
3. **Directly tests the mean reversion hypothesis:** If z-score isn't recovering, the thesis is wrong — exit. This is the cleanest possible signal.
4. **Simple and interpretable:** Just one additional exit reason, easy to backtest and understand.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN77 Recovery Rate Exit |
|--|--|--|
| Exit reasons | SL, SMA, Z0, MAX_HOLD | SL, SMA, Z0, MAX_HOLD, SLOW_REC |
| Hold duration | Determined by signal exits | Signal exits + velocity floor |
| Avg held bars | X | X − N (faster exits) |
| Slow recovery handling | Ignores (holds until other exit) | Explicit exit when velocity < threshold |
| Capital efficiency | Lower (dead capital in slow trades) | Higher (redeploys faster) |
