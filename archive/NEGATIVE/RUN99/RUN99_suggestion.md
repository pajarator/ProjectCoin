# RUN99 — Z-Score Momentum Divergence: Exit When Z-Score and Price Momentum Diverge

## Hypothesis

**Named:** `z_momentum_divergence`

**Mechanism:** COINCLAW uses z-score for entry and SMA/Z0 for exit. But z-score and price momentum can diverge: price continues in the direction of the trade while z-score reverses (divergence). This is a powerful signal that the trade's thesis is weakening. The Z-Score Momentum Divergence exit detects when price is making new extremes while z-score is failing to confirm — indicating the move lacks conviction and is likely to reverse.

**Z-Score Momentum Divergence:**
- Track `z_momentum = z_current - z_N_bars_ago` (e.g., 5 bars)
- Track `price_momentum = price_current - price_N_bars_ago`
- Compute divergence: `divergence = z_momentum * price_return_sign`
  - For LONG: expect z_momentum > 0 (z recovering toward 0) as price rises
  - If z_momentum < 0 while price_momentum > 0 → negative divergence
  - If `divergence <= DIVERGENCE_THRESHOLD` for `DIVERGENCE_BARS` consecutive bars → exit
- Example: price made +2% from entry, but z-score is less recovered than 5 bars ago → divergence

**Why this is not a duplicate:**
- RUN61 (RSI recovery threshold) used RSI divergence from price — this uses z-score divergence from price momentum
- RUN77 (recovery rate exit) used velocity of z-score recovery — this uses DIVERGENCE between z-score momentum and price momentum
- RUN60 (z_momentum threshold) used z-score direction at entry — this uses divergence between z and price as an EXIT signal
- No prior RUN has used z-score/price momentum divergence as an exit criterion

**Mechanistic rationale:** When price makes a new high but z-score fails to confirm (divergence), the move is weakening. The z-score captures deviation from mean — if price is rising but not as far from mean as before (z less extreme), the move is losing steam. This divergence often precedes reversals. Exiting on divergence avoids holding through losing momentum.

---

## Proposed Config Changes

```rust
// RUN99: Z-Score Momentum Divergence
pub const Z_MOMENTUM_DIV_ENABLE: bool = true;
pub const Z_MOMENTUM_LOOKBACK: u32 = 5;         // bars to compare z-score momentum
pub const DIVERGENCE_THRESHOLD: f64 = -0.50;   // z-momentum below this = divergence (negative)
pub const DIVERGENCE_BARS: u32 = 3;              // consecutive divergence bars before exit
```

**`engine.rs` — check_z_momentum_divergence:**
```rust
/// Check if z-score and price momentum are diverging (for open positions).
fn check_z_momentum_divergence(cs: &CoinState, ind: &Ind15m, pos: &Position) -> bool {
    if !config::Z_MOMENTUM_DIV_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }
    if cs.candles_15m.len() < config::Z_MOMENTUM_LOOKBACK as usize + 1 { return false; }

    let z_current = ind.z;
    let z_prev = cs.candles_15m[cs.candles_15m.len() - config::Z_MOMENTUM_LOOKBACK as usize].c;  // use close as proxy

    // This is a simplification — in practice, we'd need z at each historical bar
    // For this RUN, use the entry_z stored in position and compare to current z recovery
    let z_entry = match pos.z_at_entry {
        Some(z) => z,
        None => return false,
    };

    // z_momentum = how much z has recovered toward 0 since entry
    // positive = recovered (good for long), negative = diverged further
    let z_recovery_momentum = z_entry - z_current;  // positive if z moved toward 0
    let price_recovery_momentum = if pos.dir == "long" {
        (ind.p - pos.e) / pos.e  // positive if price moved in our direction
    } else {
        (pos.e - ind.p) / pos.e
    };

    // Divergence: price moving in our direction but z not recovering
    // For LONG: positive price momentum but z_recovery_momentum is negative or small
    let divergence_score = if price_recovery_momentum > 0.0 {
        // Price moving our way — z should be recovering (z_recovery_momentum should be positive)
        z_recovery_momentum / price_recovery_momentum  // ratio: should be positive
    } else {
        0.0  // price not moving our way — no divergence
    };

    // If divergence_score < threshold for DIVERGENCE_BARS consecutive bars → exit
    if divergence_score < config::DIVERGENCE_THRESHOLD {
        return true;
    }

    false
}

// In check_exit — add after recovery rate check:
if check_z_momentum_divergence(cs, &ind, &pos) {
    close_position(state, ci, price, "Z_DIV", TradeType::Regime);
    return true;
}
```

---

## Validation Method

### RUN99.1 — Z-Momentum Divergence Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no divergence exit

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_MOMENTUM_LOOKBACK` | [3, 5, 8] |
| `DIVERGENCE_THRESHOLD` | [-0.3, -0.5, -0.7] |
| `DIVERGENCE_BARS` | [2, 3, 4] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `divergence_exit_rate`: % of regime trades exited by Z_DIV
- `divergence_correctness`: % of Z_DIV exits that were winners (should be >50% if catching reversals correctly)
- `PF_delta`: profit factor change vs baseline
- `avg_held_delta`: change in average hold duration
- `total_PnL_delta`: P&L change vs baseline

### RUN99.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best divergence params per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Divergence exit rate 5–20% of regime trades
- Z_DIV exits have higher avg pnl than other exits (confirming divergence predicts reversals)

### RUN99.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no divergence) | Z-Momentum Divergence | Delta |
|--------|------------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Z_DIV Exit Rate | 0% | X% | — |
| Z_DIV Exit Win Rate | — | X% | — |
| Z_DIV Avg PnL | — | $X | — |
| Avg Held Bars | X | X | -N |

---

## Why This Could Fail

1. **Divergence can persist:** Classic technical analysis shows divergence can last for many bars before price reverses. Exiting on the first sign of divergence may cut short trades that would have eventually won.
2. **Z-score is mean-reverting by design:** Z-score naturally moves toward 0 as mean reversion occurs. Comparing z-momentum to price-momentum is comparing a mean-reverting series to a trending one — they operate on different mathematical principles.
3. **Threshold is arbitrary:** The divergence threshold (-0.5) is a guess. Optimal threshold may vary by coin and regime, making this difficult to generalize.

---

## Why It Could Succeed

1. **Classic technical analysis principle:** Divergence between indicator and price is one of the most reliable technical trading signals. Applying it to z-score is a natural extension.
2. **Captures thesis failure early:** When price is moving in our direction but z-score isn't confirming, the trade thesis is weakening. Exiting before the move fully reverses preserves capital.
3. **Works across all regime types:** Unlike ADX-based exits that are regime-specific, divergence works in any market condition — price and z can diverge in ranging, trending, and mean-reverting markets.
4. **Complementary to existing exits:** Adds a momentum-based exit alongside the price-based (SMA) and z-based (Z0) exits already in place.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN99 Z-Momentum Divergence |
|--|--|--|
| Exit reasons | SL, SMA, Z0, MAX_HOLD | SL, SMA, Z0, MAX_HOLD, Z_DIV |
| Exit signal | Price/z crossing thresholds | Z-score vs price momentum divergence |
| Thesis failure detection | None | Detects when price moves but z doesn't confirm |
| Market conditions | Any | Any |
| Implementation | None | Z-momentum / price-momentum comparison |
