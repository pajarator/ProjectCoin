# RUN62 — Regime Breakeven Stop: Locking In Gains Without Exiting

## Hypothesis

**Named:** `regime_breakeven_stop`

**Mechanism:** Scalp trades have a breakeven stop activation (`SCALP_BE_ACTIVATION = 0.40%`) that moves the SL to entry price once profit reaches that threshold. Regime trades have no equivalent — they either hit the fixed 0.3% SL or exit via signal (SMA/Z0).

The problem: A regime trade might reach +0.8% profit but then reverse before the SMA/Z0 exit fires, hitting the 0.3% SL and giving back most of the unrealized gain. Without a breakeven stop, the trade can't lock in partial gains.

The fix: Add a breakeven stop to regime trades:
```
BE_ACTIVATION = 0.5%  (profit must reach 0.5% before breakeven fires)
When pnl >= BE_ACTIVATION:
    SL = entry_price  (losses now $0, not -0.3%)
    Continue holding until signal exit (SMA/Z0)
```

**Why scalps have it but regimes don't:** Scalps are short-duration, tight-SL trades where the risk/reward tradeoff is acute. Breakeven activation makes sense at 0.4% because the scalp's total range is 0.1% SL to 0.8% TP. For regime trades (wider range, longer hold), the breakeven threshold should be proportionally higher.

**Why this is not a duplicate:**
- Scalp breakeven was RUN35 — this applies the same concept to regime trades
- No prior RUN has tested breakeven stops for non-scalp trades
- This is a new exit mechanism for the regime layer, distinct from SL, SMA, Z0, and partial exits

---

## Proposed Config Changes

```rust
// RUN62: Regime Breakeven Stop
pub const REGIME_BE_ENABLE: bool = true;
pub const REGIME_BE_ACTIVATION: f64 = 0.005;  // 0.5% profit activates breakeven
```

**`state.rs` — add be_active to Position:**
```rust
pub struct Position {
    // ... existing fields ...
    pub be_active: Option<bool>,  // already exists for scalp, needs to work for regime too
}
```

**`engine.rs` — regime breakeven in check_exit:**
```rust
fn check_exit(state: &mut SharedState, ci: usize) -> bool {
    // ... existing regime exit checks ...

    // Regime breakeven activation
    let pnl = /* compute pnl_pct */;
    if config::REGIME_BE_ENABLE && trade_type == TradeType::Regime {
        if pnl >= config::REGIME_BE_ACTIVATION {
            if let Some(ref mut p) = state.coins[ci].pos {
                if p.be_active != Some(true) {
                    p.be_active = Some(true);
                    let name = state.coins[ci].name;
                    state.log(format!("REGIME_BE {} @ {} | pnl:{:+.3}%", name, fmt_price(ind.p), pnl * 100.0));
                }
            }
        }
        let be_active = state.coins[ci].pos.as_ref().map(|p| p.be_active == Some(true)).unwrap_or(false);
        if be_active {
            // When breakeven is active, effective SL = 0 (move to entry)
            // Check if we've reverted to a loss
            if pnl <= 0.0 {
                close_position(state, ci, price, "BE", TradeType::Regime);
                return true;
            }
        }
    }

    // Existing SL check
    if pnl <= -config::STOP_LOSS {
        close_position(state, ci, price, "SL", TradeType::Regime);
        return true;
    }
    // ... rest of exits ...
}
```

---

## Validation Method

### RUN62.1 — Regime Breakeven Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no breakeven for regime trades

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `REGIME_BE_ACTIVATION` | [0.004, 0.005, 0.006, 0.008, 0.010] |

5 configs × 18 coins = 90 backtests (fast grid)

**Also test:** Does the optimal activation threshold vary by strategy type? VwapRev trades may need lower activation (faster reversion), AdrRev may need higher.

**Key metrics:**
- `be_activation_rate`: % of profitable regime trades that reach breakeven activation
- `be_lock_in_rate`: % of activated trades that subsequently revert to loss (would have given back gains)
- `avg_gain_at_be`: average profit % when breakeven activates
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline

### RUN62.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `REGIME_BE_ACTIVATION` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- BE lock-in rate > BE reversion rate (filter is catching real givebacks)
- Portfolio OOS P&L ≥ baseline

### RUN62.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Regime Breakeven | Delta |
|--------|---------------|----------------|-------|
| Total P&L | $X | $X | +$X |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | −Ypp |
| Avg Win | $X | $X | +$X |
| Avg Loss | $X | $X | −$X |
| BE Activation Rate | 0% | X% | — |
| BE Lock-In Rate | — | X% | — |
| Avg PnL at BE | — | X% | — |

---

## Why This Could Fail

1. **Breakeven exits remove upside:** If a trade reaches +0.5% and then goes to +2.0%, the breakeven exit at 0% profit means missing the full upside. Locking in at breakeven trades the remaining upside for guaranteed $0 loss prevention.
2. **Regime trades hold longer than scalp:** Scalps are short-duration (minutes). Regime trades hold longer — by the time pnl reaches 0.5%, the trade may have already been working for 20+ bars and is near its natural exit. The be_stop may fire right before the SMA exit would have fired anyway.
3. **Interaction with partial exits (RUN53):** If RUN53 is also implemented, partial exits and breakeven stops may conflict — partial exits reduce position size while breakeven stops lock in gains on full position.

---

## Why It Could Succeed

1. **Prevents giveback:** The specific failure mode it addresses is a real one: trade reaches +0.8%, then reverses to -0.3% and hits SL. Breakeven converts the potential -0.3% loss to $0.
2. **Simple concept, proven in scalp:** Scalp already uses breakeven successfully. Applying it to regime is straightforward.
3. **Low opportunity cost:** 0.5% activation is modest — trades that reach it and then reverse had limited further upside anyway.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN62 Regime Breakeven |
|--|--|--|
| Breakeven | None (regime) | Activated at +0.5% |
| Effective SL after BE | 0.3% | 0% (at entry) |
| Lock-in mechanism | None | PnL tracking |
| Scalp BE | 0.40% (RUN35) | 0.50% (regime) |
| Expected Max DD | X% | −15–25% |
| Expected Avg Win | $X | +5–15% |
| Expected BE Activation | 0% | 30–50% of profitable trades |
