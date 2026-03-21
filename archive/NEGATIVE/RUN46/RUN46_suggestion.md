# RUN46 — Partial Reversion Signal Exit: Z-Score Deviation-Adaptive Exit

## Hypothesis

**Named:** `partial_reversion_exit`

**Mechanism:** The current Z-score exit (`Z0`) fires when price has reverted to within `|z| = 0.5` of the mean. This is a fixed threshold regardless of how far the entry deviation was. If you enter at `z = −2.5` (very oversold) and price bounces to `z = −0.5`, the Z0 exit takes profit — but you've only captured 80% of the potential reversion (from −2.5 to 0, with 0.5 buffer). The remaining 20% of the move is left on the table.

The hypothesis is that **exit threshold should scale with entry deviation**:
- Enter at `z = −2.5` (extreme): hold longer, exit at `z = −0.3` or `z = 0` (capture more of the move)
- Enter at `z = −1.6` (moderate): exit earlier at `z = −0.3` or even `z = −0.5`

**Partial reversion formula:**
```
z_at_entry = stored at open_position time
reversion_pct = (z_at_entry - z_current) / z_at_entry
exit when reversion_pct >= THRESHOLD  (e.g., 0.60 = exit after 60% reversion)
```

**Alternative formulation:**
```
exit_z_threshold = z_at_entry × (1 - REVERSION_CAPTURE_PCT)
Enter z=-2.5, REVERSION_CAPTURE_PCT=0.65 → exit when z >= -0.875
Enter z=-1.6, REVERSION_CAPTURE_PCT=0.65 → exit when z >= -0.56
```

**Why this is not a duplicate:**
- RUN8 (TP optimization) tested fixed TP% from entry price — this tests exit triggered by z-score reversion depth
- RUN7 (SL optimization) tested stop loss — this tests signal exit, not SL
- No prior RUN used `z_at_entry` as a state variable for exit decisions
- Partial reversion exits capture different P&L than fixed z-threshold exits

---

## Proposed Config Changes

```rust
// RUN46: Partial Reversion Exit parameters
pub const USE_PARTIAL_REVERSION_EXIT: bool = true;
pub const REVERSION_CAPTURE_PCT: f64 = 0.65;  // exit after 65% reversion
pub const PARTIAL_REVERSION_MIN_HOLD: u32 = 3;  // require at least 3 bars before partial exit fires
pub const PARTIAL_REVERSION_OVERRIDE_Z0: bool = true;  // if true, replaces Z0 exit; if false, Z0 still fires
```

**`state.rs` — add `z_at_entry` to Position:**
```rust
pub struct Position {
    pub e: f64,
    pub s: f64,
    pub high: f64,
    pub low: f64,
    pub margin: f64,
    pub dir: String,
    pub last_price: Option<f64>,
    pub trade_type: Option<TradeType>,
    pub atr_stop: Option<f64>,
    pub trail_distance: Option<f64>,
    pub trail_act_price: Option<f64>,
    pub scalp_bars_held: Option<u32>,
    pub be_active: Option<bool>,
    pub z_at_entry: Option<f64>,  // NEW: z-score at entry for partial reversion exit
}
```

**`engine.rs` — open_position stores z_at_entry:**
```rust
cs.pos = Some(Position {
    e: price,
    s: sz,
    high: price,
    low: price,
    margin: trade_amt,
    dir: dir_str.clone(),
    last_price: None,
    trade_type: Some(trade_type),
    atr_stop: None,
    trail_distance: None,
    trail_act_price: None,
    scalp_bars_held: if trade_type == TradeType::Scalp { Some(0) } else { None },
    be_active: None,
    z_at_entry: Some(ind.z),  // NEW
});
```

**`engine.rs` — check_exit updated Z0 logic:**
```rust
// Partial reversion exit (for longs; shorts mirror the logic)
if held >= config::PARTIAL_REVERSION_MIN_HOLD {
    if let Some(ref pos) = cs.pos {
        if let Some(z_entry) = pos.z_at_entry {
            let reversion_pct = (z_entry - ind.z) / z_entry.abs();
            if reversion_pct >= config::REVERSION_CAPTURE_PCT {
                close_position(state, ci, price, "REV", TradeType::Regime);
                return true;
            }
        }
    }
}

// Z0 exit (if not overridden by partial reversion)
if held >= config::MIN_HOLD_CANDLES {
    let pnl = (price - pos.e) / pos.e;
    if pnl > 0.0 {
        if ind.z > 0.5 {
            close_position(state, ci, price, "Z0", TradeType::Regime);
            return true;
        }
    }
}
```

---

## Validation Method

### RUN46.1 — Partial Reversion Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current Z0 exit fires when `z > 0.5` (long) / `z < -0.5` (short), no partial reversion exit

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `REVERSION_CAPTURE_PCT` | [0.50, 0.60, 0.65, 0.70, 0.75, 0.80] |
| `PARTIAL_REVERSION_MIN_HOLD` | [2, 3, 4, 5] |
| `PARTIAL_REVERSION_OVERRIDE_Z0` | [true, false] |

**Per coin:** 6 × 4 × 2 = 48 configs × 18 coins = 864 backtests

**Also test:** Is the optimal `REVERSION_CAPTURE_PCT` correlated with coin volatility? High-vol coins (SOL, AVAX) may need a lower capture pct due to noisier z-scores; low-vol coins (BTC, ETH) can hold for higher reversion.

**Key metrics:**
- `avg_exit_z`: average z-score at which partial reversion exit fires (lower = captures more of the move)
- `avg_held_bars`: average bars held with partial reversion exit
- `partial_exit_rate`: % of trades that exit via partial reversion vs Z0 vs SMA
- `PF_delta`: profit factor change vs baseline
- `avg_win_delta`: average win amount change (should increase if capturing larger moves)

### RUN46.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `REVERSION_CAPTURE_PCT × PARTIAL_REVERSION_MIN_HOLD` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Portfolio OOS P&L ≥ baseline
- Avg held bars increases (capturing bigger moves takes longer — expected)

### RUN46.3 — Combined Comparison

Side-by-side:

| Metric | Baseline Z0 (v16) | Partial Reversion Exit | Delta |
|--------|-------------------|----------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Avg Win | $X | $X | +$X |
| Avg Held Bars | X | X | +N |
| Partial Exit Rate | 0% | X% | — |
| Z0 Exit Rate | X% | X% | — |
| SMA Exit Rate | X% | X% | — |
| Avg Exit Z (longs) | +0.5 | X | — |

---

## Why This Could Fail

1. **Holding longer means more SL hits:** If you hold for 65% reversion instead of exiting at z=0.5, the price can reverse again before reaching the partial exit, hitting SL. The net effect may be worse avg_win/loss ratio.
2. **Z-score is mean-reverting by definition:** By the time z has reverted 65%, the reversion move may already be exhausted. The "extra profit" from holding may be illusory.
3. **MIN_HOLD interaction:** The existing `MIN_HOLD_CANDLES = 2` already forces holding. Partial reversion on top of MIN_HOLD may not add much incremental holding time.

---

## Why It Could Succeed

1. **Captures the full reversion move:** Mean reversion from extreme (z < −2.0) to the mean typically overshoots slightly. Holding for 65% of the move captures the bulk of it before the inevitable small overshoot reversal.
2. **Higher avg win per trade:** Even if win rate stays the same, capturing larger moves per trade improves total P&L without increasing SL losses.
3. **Simple change:** Only the Z0 exit branch changes; SMA20 exit and SL are unaffected.

---

## Comparison to Baseline

| | Current Z0 Exit (v16) | RUN46 Partial Reversion Exit |
|--|--|--|
| Exit trigger | Fixed z > 0.5 | Z has reverted ≥ 65% of entry deviation |
| Hold duration | MIN_HOLD only | MIN_HOLD + partial reversion target |
| z_at_entry tracking | None | Stored at open_position |
| Avg exit z-score | 0.5 | Variable (0.3–0.0) |
| Expected avg win | $X | +10–20% higher |
| Expected hold time | X bars | +2–4 bars |
