# RUN88 — Trailing Z-Score Exit: Exit When Mean Reversion Has Recovered a Target Fraction

## Hypothesis

**Named:** `trailing_z_exit`

**Mechanism:** COINCLAW regime trades currently exit when price crosses the SMA20 or when z-score crosses 0 (Z0 exit). But these are binary — either the signal is still valid or it's not. There's no concept of "the trade has recovered 70% of the expected reversion, let's take profits here." The Trailing Z-Score Exit adds a fractional exit: if we entered at z = -2.0, exit when z has recovered 65% of the way back to 0 (i.e., when z = -0.7).

**Trailing Z-Score Exit:**
- At entry, record `z_at_entry`
- Compute target z for exit: `z_exit_target = z_at_entry * (1.0 - Z_RECOVERY_PCT)`
  - Example: z_entry = -2.0, Z_RECOVERY_PCT = 0.65 → z_exit_target = -0.7
  - When current z >= -0.7 (i.e., crosses back toward 0), exit with reason `Z_RECOVERY`
- This is a tighter exit than Z0 (z = 0) — it takes profits earlier, capturing a partial reversion rather than waiting for full mean reversion
- Add a minimum hold requirement: `Z_RECVERY_MIN_HOLD` bars (to avoid exiting immediately)

**Why this is not a duplicate:**
- RUN53 (tiered partial exits) exits based on PnL percentage — this exits based on z-score recovery FRACTION
- RUN46 (partial reversion capture) exits at fixed PnL tiers — this exits at z-score levels that represent fractional mean reversion
- RUN77 (recovery rate exit) exits when reversion is too SLOW — this exits when reversion HAS HAPPENED at a target fraction
- No prior RUN has used z-score recovery FRACTION as an exit criterion

**Mechanistic rationale:** Full mean reversion (z = 0) doesn't always happen — sometimes price reverts 65% then reverses. Taking profits at 65% recovery locks in gains without requiring the full reversion. This is a more achievable target than z = 0 and reduces the risk of giving back profits in coins that only partially revert.

---

## Proposed Config Changes

```rust
// RUN88: Trailing Z-Score Exit
pub const TRAILING_Z_EXIT_ENABLE: bool = true;
pub const Z_RECOVERY_PCT: f64 = 0.65;        // exit when 65% of entry z-score has recovered
pub const Z_RECOVERY_MIN_HOLD: u32 = 8;       // minimum bars before Z_RECOVERY exit can fire
```

**`engine.rs` — check_z_recovery_exit in check_exit:**
```rust
/// Check if position should exit due to z-score recovery fraction.
fn check_z_recovery_exit(cs: &CoinState, ind: &Ind15m, pos: &Position) -> bool {
    if !config::TRAILING_Z_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }

    let z_entry = match pos.z_at_entry {
        Some(z) => z,
        None => return false,
    };

    // Minimum hold requirement
    if cs.candles_held < config::Z_RECOVERY_MIN_HOLD { return false; }

    let z_current = ind.z;
    let z_target = z_entry * (1.0 - config::Z_RECOVERY_PCT);

    // For long: z_entry < 0, we want z to increase (become less negative) toward 0
    // z_current >= z_target means we've recovered enough
    if pos.dir == "long" {
        if z_current >= z_target {
            return true;
        }
    } else {
        // For short: z_entry > 0, we want z to decrease (become less positive)
        // z_current <= z_target means we've recovered enough
        if z_current <= z_target {
            return true;
        }
    }

    false
}

// In check_exit — add after recovery rate check but before SL check:
if check_z_recovery_exit(cs, &ind, &pos) {
    close_position(state, ci, price, "Z_RECOVERY", TradeType::Regime);
    return true;
}
```

---

## Validation Method

### RUN88.1 — Trailing Z-Score Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — exits at SMA or Z0, no partial z-score recovery

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_RECOVERY_PCT` | [0.50, 0.60, 0.65, 0.70, 0.75] |
| `Z_RECOVERY_MIN_HOLD` | [4, 8, 12] |

**Per coin:** 5 × 3 = 15 configs × 18 coins = 270 backtests

**Key metrics:**
- `z_recovery_exit_rate`: % of regime trades exited by Z_RECOVERY
- `avg_z_at_entry`: average z-score at entry for Z_RECOVERY exits
- `avg_z_at_exit`: average z-score at Z_RECOVERY exit (should be lower magnitude than entry)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_exit_pnl_delta`: change in average exit PnL

### RUN88.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best Z_RECOVERY_PCT × MIN_HOLD per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Z_RECOVERY exit rate 5–30% of regime trades
- Z_RECOVERY exits have higher avg PnL than other exits (confirming they capture partial reversions)

### RUN88.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, SMA/Z0 exit) | Trailing Z-Score Exit | Delta |
|--------|---------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Z_RECOVERY Exit Rate | 0% | X% | — |
| Avg Z at Entry (Z_REC exits) | — | X | — |
| Avg Z at Exit (Z_REC exits) | — | X | — |
| Avg PnL at Z_REC Exit | — | $X | — |
| Other Exit Avg PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **Partial reversions are not always available:** Some coins mean-revert all the way to z = 0. Exiting at 65% recovery leaves money on the table compared to the full reversion.
2. **The "right" recovery fraction varies by coin:** Coins with different volatility characteristics have different typical reversion depths. A one-size-fits-all Z_RECOVERY_PCT may be suboptimal for some coins.
3. **May exit before full momentum:** If a coin is in a strong mean-reversion regime, z may recover to 0 then continue to +1 (overshoot). Z_RECOVERY exit would exit before this overshoot, which may be the most profitable part of the move.

---

## Why It Could Succeed

1. **Locks in partial profits:** Taking 65% of the expected reversion is better than risking a full reversion that may not complete. This is a more achievable target.
2. **Reduces hold time:** Exiting at fractional recovery reduces average hold duration, freeing capital for the next trade.
3. **Better than fixed PnL exits:** Z_RECOVERY is expressed in z-score units, which are normalized and comparable across coins. A fixed PnL% is not — z-score recovery is a more principled measure of mean reversion progress.
4. **Simple and interpretable:** One new exit reason, one new config parameter. Easy to backtest and understand.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN88 Trailing Z-Score Exit |
|--|--|--|
| Z entry = -2.0 | Exits at z = 0 (SMA or Z0) | Exits at z = -0.7 (65% recovered) |
| Exit trigger | Binary (signal present/absent) | Fractional (65% recovered) |
| Avg hold duration | X bars | X − N bars |
| Partial reversion handling | Holds until full reversion | Exits at fractional recovery |
| Exit reasons | SL, SMA, Z0, MAX_HOLD | SL, SMA, Z0, MAX_HOLD, Z_RECOVERY |
