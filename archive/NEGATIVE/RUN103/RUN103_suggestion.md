# RUN103 — Stochastic Extreme Exit: Exit When Stochastic Reaches Extreme Levels During Profitable Trades

## Hypothesis

**Named:** `stochastic_extreme_exit`

**Mechanism:** COINCLAW's scalp trades already use stochastic (stoch_k/stoch_d) for entry signals. But stochastic is not used at all for regime trade exits. When a regime LONG position is profitable and stochastic rises above 80 (overbought), it often signals a local top — the mean reversion may be exhausting. The Stochastic Extreme Exit adds an exit trigger: when a profitable regime position coincides with stochastic reaching extreme levels, exit immediately.

**Stochastic Extreme Exit:**
- For regime LONG positions: if `stoch_k >= STOCH_OB_THRESHOLD` (e.g., 80) AND the trade is profitable (pnl > 0) → exit with reason `STOCH_OB`
- For regime SHORT positions: if `stoch_k <= STOCH_OS_THRESHOLD` (e.g., 20) AND trade is profitable → exit with reason `STOCH_OS`
- Require a minimum hold: `STOCH_MIN_HOLD` bars (e.g., 5) to avoid exiting immediately on noise
- Scalp trades already have stochastic-based exits (STOCH50) — this applies only to regime trades

**Why this is not a duplicate:**
- RUN35 (scalp stochastic exit) used stochastic crossing 50 for scalp — this uses stochastic at EXTREME levels (80/20) for regime trades
- RUN53 (tiered partial exits) exited at PnL thresholds — this exits at indicator (stochastic) extremes
- No prior RUN has used stochastic overbought/oversold as an exit for regime trades

**Mechanistic rationale:** Stochastic at extreme levels (≥80 or ≤20) indicates the short-term momentum has been stretched. For a mean-reversion trade that is already profitable, this is the optimal time to take profits — the stochastic extreme confirms the reversion has reached a short-term climax. This is the opposite of "buy when stochastic is oversold" — it's "sell when you're already profitable and stochastic is overbought."

---

## Proposed Config Changes

```rust
// RUN103: Stochastic Extreme Exit
pub const STOCH_EXTREME_EXIT_ENABLE: bool = true;
pub const STOCH_OB_THRESHOLD: f64 = 80.0;   // overbought threshold for LONG exits
pub const STOCH_OS_THRESHOLD: f64 = 20.0;   // oversold threshold for SHORT exits
pub const STOCH_EXTREME_MIN_HOLD: u32 = 5;   // minimum bars before stochastic exit can fire
```

**`engine.rs` — check_stoch_extreme_exit:**
```rust
/// Check if a regime position should exit due to stochastic extremes.
fn check_stoch_extreme_exit(state: &mut SharedState, ci: usize, ind: &Ind15m, pos: &Position) -> bool {
    if !config::STOCH_EXTREME_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }

    let held = state.coins[ci].candles_held;
    if held < config::STOCH_EXTREME_MIN_HOLD { return false; }

    let stoch_k = ind.stoch_k;
    if stoch_k.is_nan() { return false; }

    // Must be profitable to use this exit (don't exit at a loss on stochastic)
    let pnl = if pos.dir == "long" {
        (ind.p - pos.e) / pos.e
    } else {
        (pos.e - ind.p) / pos.e
    };
    if pnl <= 0.0 { return false; }

    match pos.dir.as_str() {
        "long" => {
            if stoch_k >= config::STOCH_OB_THRESHOLD {
                return true;
            }
        }
        "short" => {
            if stoch_k <= config::STOCH_OS_THRESHOLD {
                return true;
            }
        }
        _ => {}
    }

    false
}

// In check_exit — add before MAX_HOLD check:
if check_stoch_extreme_exit(state, ci, &ind, &pos) {
    close_position(state, ci, ind.p, "STOCH_EXTREME", TradeType::Regime);
    return true;
}
```

---

## Validation Method

### RUN103.1 — Stochastic Extreme Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no stochastic exit for regime trades

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `STOCH_OB_THRESHOLD` | [75.0, 80.0, 85.0] |
| `STOCH_OS_THRESHOLD` | [15.0, 20.0, 25.0] |
| `STOCH_EXTREME_MIN_HOLD` | [3, 5, 8] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `stoch_exit_rate`: % of regime trades exited by stochastic extreme
- `stoch_exit_win_rate`: win rate of stochastic extreme exits (should be high — exiting while profitable)
- `PF_delta`: profit factor change vs baseline
- `avg_pnl_delta`: change in average exit PnL (should increase if capturing peaks)
- `total_PnL_delta`: P&L change vs baseline

### RUN103.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold combinations per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Stochastic extreme exit win rate >70% (these are profit-taking exits, should be winners)
- Stochastic exit rate 5–20% of regime trades

### RUN103.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no stoch exit) | Stochastic Extreme Exit | Delta |
|--------|-------------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Stochastic Exit Rate | 0% | X% | — |
| Stochastic Exit Win Rate | — | X% | — |
| Stochastic Exit Avg PnL | — | $X | — |
| Other Exit Avg PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **Stochastic extremes can persist:** Stochastic can stay at 80+ for many bars during a strong trend. Exiting at the first touch of 80 when the trade is profitable might cut short a winning position that would have continued.
2. **Regime trades should hold for mean reversion:** The thesis for regime trades is that price reverts to the mean. If stochastic is overbought, the mean reversion may not be complete — exiting early sacrifices the remaining potential.
3. **Profitable now doesn't mean won't be more profitable:** A trade at +0.5% with stoch=80 might go to +2.0% if the mean reversion continues. Taking the +0.5% might be suboptimal.

---

## Why It Could Succeed

1. **Catches local tops:** Stochastic at extreme levels often coincides with short-term exhaustion. Exiting when profitable at these points locks in gains before a potential pullback.
2. **Profit-taking signal:** This is fundamentally a profit-taking exit — it captures gains when the market shows short-term exhaustion, rather than waiting for the full mean reversion.
3. **Scalp already uses stochastic:** COINCLAW already uses stochastic for scalp exits (STOCH50). Extending this to regime trades is natural.
4. **Requires profitability:** By only exiting when pnl > 0, we never use this exit to cut losses. This is purely a gains-capture mechanism.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN103 Stochastic Extreme Exit |
|--|--|--|
| Stochastic for regime exits | Not used | Exit when stoch ≥ 80 (long) or ≤ 20 (short) |
| Exit condition | SMA, Z0, SL, MAX_HOLD | SMA, Z0, SL, MAX_HOLD, STOCH_EXTREME |
| Trade requirement | Any | Must be profitable |
| Profit-taking | Via partial exits (RUN53) | Via indicator extremes |
| Short-term exhaustion signal | None | Stochastic extremes |
