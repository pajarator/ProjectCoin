# RUN69 — Winning Streak Profit-Taking: Exit Early After Extended Winning Runs

## Hypothesis

**Named:** `winning_streak_profit_take`

**Mechanism:** After a winning streak (N consecutive winning trades), the market may be in a particularly favorable mean-reversion regime — multiple coins are oversold simultaneously and recover quickly. This is exactly the environment COINCLAW excels in. But winning streaks also mean the portfolio has accumulated unrealized gains that could be given back if the regime shifts.

The hypothesis is that **after a winning streak, take profits more aggressively** — lower the TP threshold or exit earlier on the next trade to lock in gains before the favorable regime ends.

```
consecutive_wins = count of consecutive winning trades for this coin
if consecutive_wins >= STREAK_COUNT:
    activate streak_mode:
        LOWER TP threshold by STREAK_DISCOUNT (e.g., 20% lower)
        or set a faster SMA exit (exit after price reaches SMA20, even if z hasn't fully reverted)
```

**Why this is not a duplicate:**
- No prior RUN has triggered on *consecutive wins* as a signal
- No prior RUN has used a "winning streak mode" to alter exit behavior
- This is a pattern-detection exit modifier, distinct from all prior loss-based triggers

---

## Proposed Config Changes

```rust
// RUN69: Winning Streak Profit-Taking
pub const STREAK_PROFIT_ENABLE: bool = true;
pub const STREAK_COUNT: u32 = 3;           // activate after 3 consecutive wins
pub const STREAK_TP_DISCOUNT: f64 = 0.20;  // exit at 20% less profit than normal
```

**`engine.rs` — streak mode modifies effective TP:**
```rust
fn effective_tp_for_streak(cs: &CoinState, base_tp: f64) -> f64 {
    if !config::STREAK_PROFIT_ENABLE { return base_tp; }
    if cs.consecutive_wins >= config::STREAK_COUNT {
        return base_tp * (1.0 - config::STREAK_TP_DISCOUNT);
    }
    base_tp
}

// In check_exit, after partial exits but before signal exits:
if streak_mode_active {
    // Check if price has reached the discounted TP level
    let streak_tp = effective_tp_for_streak(cs, /* base_target */);
    if pnl >= streak_tp {
        close_position(state, ci, price, "STREAK", TradeType::Regime);
        return true;
    }
}
```

---

## Validation Method

### RUN69.1 — Winning Streak Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no streak mode

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `STREAK_COUNT` | [2, 3, 4, 5] |
| `STREAK_TP_DISCOUNT` | [0.10, 0.20, 0.30] |

**Per coin:** 4 × 3 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `streak_activation_rate`: % of trades that occur after a streak is active
- `streak_profit_capture_rate`: % of streak trades that hit the discounted TP
- `streak_PnL_delta`: net P&L change vs baseline
- `streak_preservation_rate`: % of streak trades that would have given back gains without streak mode

### RUN69.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `STREAK_COUNT × STREAK_TP_DISCOUNT` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Streak trades preserve ≥ 70% of gains that would otherwise be given back

### RUN69.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Winning Streak Mode | Delta |
|--------|---------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | −Ypp |
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Streak Activation Rate | 0% | X% | — |
| Avg Streak Length | 0 | X | +N |
| Streak Profit Capture | — | X% | — |
