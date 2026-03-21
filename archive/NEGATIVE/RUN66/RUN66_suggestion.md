# RUN66 — Exit Reason Priority Reordering: Optimizing Which Signal Fires First

## Hypothesis

**Named:** `exit_priority_reorder`

**Mechanism:** COINCLAW's `check_exit` function evaluates exit conditions in a fixed order:
1. SL check
2. Breakeven check (if active)
3. Partial exit check (if enabled)
4. SMA20 check
5. Z0 check

When multiple exit conditions are met simultaneously (e.g., price crosses SMA20 AND z-score crosses 0.5 in the same bar), only the first in the priority order fires. The exit reason of the *first* satisfied condition determines the trade outcome.

The hypothesis is that **the current fixed priority may not be optimal**. Different exit reasons have different trade quality implications:
- **Z0 exit** fires when z-score reverts to mean — this is the purest mean-reversion signal
- **SMA exit** fires when price crosses the moving average — broader signal, may fire before reversion completes
- **Breakeven exit** fires when pnl reaches a threshold — locks in $0 loss, may fire too early

**Alternative priority orders to test:**
```
Order A (current):    SL → BE → Partial → SMA → Z0
Order B:              SL → Z0 → SMA → BE → Partial
Order C:              SL → BE → Z0 → SMA → Partial
Order D:              SL → Z0 → BE → SMA → Partial
```

**Why this is not a duplicate:**
- No prior RUN has tested or optimized the order of exit condition evaluation
- All prior RUNs optimized individual exit parameters (SL%, SMA depth, Z0 threshold), not the priority tree
- Exit priority is a structural decision that determines which signal "wins" when multiple fire simultaneously

---

## Proposed Config Changes

```rust
// RUN66: Exit Priority Ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPriority {
    Current,    // SL → BE → Partial → SMA → Z0
    Z0First,   // SL → Z0 → SMA → BE → Partial
    BEFirst,   // SL → BE → Z0 → SMA → Partial
    Z0BeforeSMA, // SL → Z0 → BE → SMA → Partial
}

pub const EXIT_PRIORITY_ORDER: ExitPriority = ExitPriority::Z0First;
```

**`engine.rs` — restructure check_exit as priority-based:**
```rust
fn evaluate_exit_priority(state: &mut SharedState, ci: usize, price: f64) -> Option<(&'static str, TradeType)> {
    let pos = match &state.coins[ci].pos {
        Some(p) => p.clone(),
        None => return None,
    };
    let trade_type = pos.trade_type.unwrap_or(TradeType::Regime);
    let pnl = /* compute pnl_pct */;
    let ind = /* get indicators */;

    match config::EXIT_PRIORITY_ORDER {
        ExitPriority::Current => {
            // SL → BE → Partial → SMA → Z0
        }
        ExitPriority::Z0First => {
            // Check Z0 first (after MIN_HOLD)
            if held >= MIN_HOLD && pnl > 0.0 && z_exit_condition_met(&ind) {
                return Some(("Z0", trade_type));
            }
            // Then SMA
            if held >= MIN_HOLD && pnl > 0.0 && sma_exit_condition_met(&ind) {
                return Some(("SMA", trade_type));
            }
            // Then BE
            if be_active && pnl <= 0.0 {
                return Some(("BE", trade_type));
            }
            // Then Partial
            if partial_exit_due(&pos) {
                return Some(("PARTIAL", trade_type));
            }
            // Then SL
            if pnl <= -config::STOP_LOSS {
                return Some(("SL", trade_type));
            }
        }
        // ... other orderings ...
    }
    None
}
```

---

## Validation Method

### RUN66.1 — Exit Priority Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — ExitPriority::Current

**Grid search:**

| Priority Order | Description |
|---------------|-------------|
| A (current) | SL → BE → Partial → SMA → Z0 |
| B | SL → Z0 → SMA → BE → Partial |
| C | SL → BE → Z0 → SMA → Partial |
| D | SL → Z0 → BE → SMA → Partial |

4 configs × 18 coins = 72 backtests (very fast)

**Also test:** Is the optimal priority different for LONG vs SHORT trades? (LONG: Z0 fires when z > 0.5; SHORT: Z0 fires when z < -0.5)

**Key metrics:**
- `exit_reason_distribution`: % of exits by reason (should shift with priority change)
- `avg_exit_pnl`: average PnL when each exit reason fires
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline

### RUN66.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best exit priority order per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Portfolio OOS P&L ≥ baseline

### RUN66.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Best Priority Order | Delta |
|--------|---------------|-------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Z0 Exit Rate | X% | X% | +Ypp |
| SMA Exit Rate | X% | X% | +Ypp |
| BE Exit Rate | X% | X% | +Ypp |
| Avg Z0 Exit PnL | $X | $X | +$X |
| Avg SMA Exit PnL | $X | $X | +$X |
| Avg BE Exit PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **Priority only matters when conditions fire simultaneously:** If Z0 and SMA rarely fire in the same bar, changing their order has minimal effect.
2. **Exit reasons are already ranked by quality:** The current order (SL → BE → Partial → SMA → Z0) may already be near-optimal by putting the most "permanent" exit (SL = realized loss) first.
3. **Holding longer to let Z0 fire before SMA reduces total trade count:** If Z0 fires before SMA, the trade exits later (more profit) but also holds capital longer.

---

## Why It Could Succeed

1. **Z0 is the purest mean-reversion signal:** Exiting at Z0 captures the full reversion before SMA might exit on a partial cross. Prioritizing Z0 means "let the mean-reversion complete before taking any exit."
2. **SMA can fire prematurely:** In choppy markets, price oscillating around SMA20 fires SMA exits on minor reversals before z-score has actually reverted. Z0-first prevents this.
3. **Trivial to implement:** One enum, a match statement restructure. Zero new data or complex logic.

---

## Comparison to Baseline

| | Current Priority (v16) | RUN66 Optimized Priority |
|--|--|--|
| Exit order | Fixed: SL→BE→Partial→SMA→Z0 | Configurable |
| Z0 priority | Last | Moved up |
| SMA priority | Middle | Moved |
| BE priority | Second | Moved |
| Expected PF | ~0.88 | +0.02–0.08 |
| Expected Max DD | X% | −5–15% |
