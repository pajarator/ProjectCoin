# RUN90 — Symmetry Exit: Risk-Reward Ratio Scaled by Entry Z-Score Magnitude

## Hypothesis

**Named:** `symmetry_exit`

**Mechanism:** COINCLAW regime trades currently have a fixed 0.30% stop loss but no take profit — exits happen at SMA or Z0, which may be far from entry in price terms. This means the risk-reward ratio varies wildly depending on entry z-score: a coin entered at z = -1.5 has a different expected move than one entered at z = -3.0. The Symmetry Exit enforces a target risk-reward ratio: when entry z-score is extreme, the take profit is closer (symmetric with SL), and when entry z-score is moderate, the take profit is further.

**Symmetry Exit:**
- Compute symmetric TP price: `tp_price = entry_price + (entry_price - sl_price) * SYMMETRY_RATIO`
  - Example: entry = $100, SL = $99.70 (0.30% below), SYMMETRY_RATIO = 1.5 → TP = $100 + ($100 - $99.70) × 1.5 = $100.45
- This gives a fixed R:R ratio (1.5:1 in this example) but expressed in price terms, not z-score terms
- Exit at TP with reason `SYM_TP` before Z0/SMA if TP is reached first
- Allow SYM_TP only when `|z_at_entry| >= SYM_Z_MIN` (e.g., 2.0) — only for extreme entries

**Why this is not a duplicate:**
- RUN53 (tiered partial exits) exits at fixed PnL tiers (0.4%, 0.8%) — this scales TP based on entry z-score magnitude
- RUN88 (trailing z exit) exits at z-score recovery fraction — this uses PRICE-based symmetry (SL distance × ratio)
- No prior RUN has enforced a fixed risk-reward ratio on regime trades based on entry conditions

**Mechanistic rationale:** A coin at z = -3.0 is extremely oversold — the expected mean reversion is larger in absolute terms than a coin at z = -1.5. But the current exit logic (SMA or Z0) gives both coins similar exit distances in price terms. By scaling the TP relative to the SL, we enforce that extreme entries produce larger absolute profits — capturing the full value of the opportunity. This also improves Sharpe by standardizing the reward per unit of risk.

---

## Proposed Config Changes

```rust
// RUN90: Symmetry Exit
pub const SYMMETRY_EXIT_ENABLE: bool = true;
pub const SYMMETRY_RATIO: f64 = 1.5;        // TP distance = SL distance × 1.5 (1.5:1 R:R)
pub const SYM_Z_MIN: f64 = 2.0;              // minimum |z| at entry to activate symmetry exit
pub const SYM_MIN_HOLD: u32 = 4;              // minimum bars before SYM_TP can fire (avoid immediate exit)
```

**`engine.rs` — check_symmetry_exit in check_exit:**
```rust
/// Compute symmetric take profit price for a position.
fn symmetry_tp_price(pos: &Position, entry_price: f64) -> Option<f64> {
    if !config::SYMMETRY_EXIT_ENABLE { return None; }

    let z_entry = match pos.z_at_entry {
        Some(z) => z,
        None => return None,
    };

    if z_entry.abs() < config::SYM_Z_MIN { return None; }

    // SL distance in price terms
    let sl_dist = if pos.dir == "long" {
        entry_price * config::STOP_LOSS
    } else {
        entry_price * config::STOP_LOSS
    };

    // TP = entry + sl_dist × ratio (long) or entry - sl_dist × ratio (short)
    let tp = if pos.dir == "long" {
        entry_price + sl_dist * config::SYMMETRY_RATIO
    } else {
        entry_price - sl_dist * config::SYMMETRY_RATIO
    };

    Some(tp)
}

/// Check if position should exit via symmetry TP.
fn check_symmetry_exit(state: &SharedState, ci: usize, pos: &Position, price: f64) -> bool {
    if !config::SYMMETRY_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }
    if cs.candles_held < config::SYM_MIN_HOLD { return false; }

    let entry_price = pos.e;
    let tp_price = match symmetry_tp_price(pos, entry_price) {
        Some(t) => t,
        None => return false,
    };

    let hit_tp = if pos.dir == "long" {
        price >= tp_price
    } else {
        price <= tp_price
    };

    if hit_tp {
        close_position(state, ci, price, "SYM_TP", TradeType::Regime);
        return true;
    }

    false
}

// In check_exit — add after Z_RECOVERY check:
if check_symmetry_exit(state, ci, &pos, price) {
    return true;
}
```

---

## Validation Method

### RUN90.1 — Symmetry Exit Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no symmetry TP, exits only at SMA/Z0/SL

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SYMMETRY_RATIO` | [1.0, 1.5, 2.0, 2.5] |
| `SYM_Z_MIN` | [1.5, 2.0, 2.5] |
| `SYM_MIN_HOLD` | [2, 4, 6] |

**Per coin:** 4 × 3 × 3 = 36 configs × 18 coins = 648 backtests

**Key metrics:**
- `sym_tp_rate`: % of regime trades exited by SYM_TP
- `avg_z_at_sym_entries`: average |z| at entry for SYM_TP exits (confirm Z_MIN filtering)
- `RRR_achieved`: actual average R:R ratio of SYM_TP exits (should be close to SYMMETRY_RATIO)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline

### RUN90.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best SYMMETRY_RATIO × Z_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- SYM_TP exit rate 5–25% of regime trades
- SYM_TP exits have avg R:R ≥ 1.0 (profitable on average)

### RUN90.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no SYM_TP) | Symmetry Exit | Delta |
|--------|--------------------------|--------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| SYM_TP Exit Rate | 0% | X% | — |
| Avg R:R Achieved | — | X.X:1 | — |
| Avg Z at SYM_TP Entry | — | X | — |
| Other Exit Avg PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **SMA/Z0 exits may be better:** If the SMA crossback or Z0 crossing happens before the symmetry TP is hit, the symmetry exit never fires. The opportunity cost of locking in a fixed R:R may be missing out on larger moves that SMA/Z0 would capture.
2. **Z-score is mean-reverting by design:** The expected mean reversion (distance back to SMA) is already captured by the existing Z0/SMA exit logic. Adding a price-based symmetry TP on top may be redundant.
3. **Forces exits that could have been bigger winners:** A coin at z = -3.0 might revert all the way to z = 0 (SMA). Forcing a symmetry TP at 1.5:1 cuts short a potentially larger winner.

---

## Why It Could Succeed

1. **Standardizes risk-reward:** By enforcing a minimum R:R on all extreme-z entries, we improve the average quality of winning trades. The distribution of win sizes becomes more consistent.
2. **Captures the most extreme opportunities:** Coins at z = -3.0 or lower have the largest expected moves. The symmetry exit captures a predictable fraction of that move without requiring the full reversion.
3. **Prevents hold-through-reversal:** The symmetry TP exits before the price can reverse from a partial reversion. This prevents the common pattern of entering at the extreme and giving back profits before exiting.
4. **Simple and interpretable:** One ratio, one z_min threshold. The R:R is explicit and configurable.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN90 Symmetry Exit |
|--|--|--|
| Exit reasons | SL, SMA, Z0, MAX_HOLD | SL, SMA, Z0, MAX_HOLD, SYM_TP |
| R:R ratio | Undefined (no TP) | Fixed 1.5:1 for extreme z entries |
| TP for z=-3.0, entry=$100 | SMA/Z0 (unknown distance) | $100 + 0.30%×1.5 = $100.45 |
| Entry requirement | None | |z| ≥ 2.0 |
| Win standardization | None | Consistent R:R on extreme entries |
