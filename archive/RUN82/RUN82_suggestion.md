# RUN82 — Regime Decay Detection: Early Exit When Ranging Transitions to Trending

## Hypothesis

**Named:** `regime_decay_exit`

**Mechanism:** COINCLAW regime trades are mean-reversion strategies designed for Ranging and WeakTrend regimes. When a position is opened, the regime is detected at entry time. But regimes are not static — a coin can be Ranging when we enter, then slowly shift to WeakTrend or StrongTrend while we're holding the position. The current exit logic handles this only through the Z0 and SMA crossback signals, but these are lagging indicators of regime change.

**Regime Decay Detection Exit:**
- At position open, record the regime (`entry_regime`) and ADX value (`entry_adx`)
- Each bar, compare current regime and ADX to entry values:
  - If current regime is StrongTrend AND position is LONG → regime is decaying against us
  - If current regime is StrongTrend AND position is SHORT → regime is decaying against us (short in StrongTrend is dangerous)
  - If ADX has risen more than `ADX_DECAY_THRESHOLD` above entry ADX (e.g., +15) while regime has worsened → early exit before trend fully establishes
- Exit reason: `REGIME_DECAY` — a pre-emptive exit that avoids riding a developing trend against us

**Why this is not a duplicate:**
- RUN43 (breadth velocity) detects market-wide regime transitions — this detects per-coin regime decay while IN a position
- RUN56 (SMA penetration depth) measures how far price has moved through SMA — this measures ADX regime shift
- RUN65 (BB squeeze duration) uses BB width to detect trend building — this uses ADX + regime classification
- No prior RUN has implemented an early exit that proactively exits positions when the regime deteriorates around them

**Mechanistic rationale:** Mean reversion strategies fail in trending markets. A position opened in a Ranging regime that transitions to StrongTrend will eventually hit SL at 0.30% — but not before the trend has had time to develop. Detecting the transition early and exiting pre-emptively avoids the large drawdown from a trending move against us, freeing capital for the next setup.

---

## Proposed Config Changes

```rust
// RUN82: Regime Decay Detection
pub const REGIME_DECAY_ENABLE: bool = true;
pub const REGIME_DECAY_ADX_RISE: f64 = 15.0;     // if ADX rises by 15+ above entry ADX, consider decay
pub const REGIME_DECAY_REGIME_SHIFT: bool = true;  // if regime shifts to StrongTrend while in position, count as decay
pub const REGIME_DECAY_GRACE_BARS: u32 = 5;       // don't check in first 5 bars (avoid noise)
```

**`state.rs` — Position changes:**
```rust
pub struct Position {
    // ... existing fields ...
    pub entry_regime: Option<Regime>,   // regime at time of entry
    pub entry_adx: Option<f64>,          // ADX at time of entry
}
```

**`engine.rs` — check_regime_decay in check_exit:**
```rust
/// Check if current regime has deteriorated vs entry regime.
fn check_regime_decay(cs: &CoinState, ind: &Ind15m, pos: &Position) -> bool {
    if !config::REGIME_DECAY_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }
    if cs.candles_held < config::REGIME_DECAY_GRACE_BARS { return false; }

    let entry_regime = match pos.entry_regime {
        Some(r) => r,
        None => return false,
    };
    let entry_adx = match pos.entry_adx {
        Some(a) => a,
        None => return false,
    };

    let current_regime = cs.regime;
    let current_adx = ind.adx;

    // Regime shift: entered in Ranging/WeakTrend, now in StrongTrend
    if config::REGIME_DECAY_REGIME_SHIFT {
        if entry_regime != Regime::StrongTrend && current_regime == Regime::StrongTrend {
            // Regime shifted to StrongTrend — mean reversion is invalid
            return true;
        }
    }

    // ADX decay: ADX has risen significantly since entry (trend strengthening)
    if config::REGIME_DECAY_ADX_RISE > 0.0 {
        if current_adx > entry_adx + config::REGIME_DECAY_ADX_RISE {
            // Trend is building — exit before it fully establishes
            return true;
        }
    }

    false
}

// In check_exit — add near the top (before SL check):
if check_regime_decay(cs, &ind, &pos) {
    close_position(state, ci, price, "REGIME_DECAY", TradeType::Regime);
    return true;
}

// In open_position — record entry regime and ADX:
cs.pos = Some(Position {
    // ... existing fields ...
    entry_regime: Some(cs.regime),
    entry_adx: Some(ind.adx),
});
```

**`coordinator.rs` — detect_regime changes not required** (already has Regime enum)

---

## Validation Method

### RUN82.1 — Regime Decay Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no regime decay exit

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `REGIME_DECAY_ADX_RISE` | [10.0, 15.0, 20.0, 25.0] |
| `REGIME_DECAY_REGIME_SHIFT` | [true, false] |
| `REGIME_DECAY_GRACE_BARS` | [3, 5, 10] |

**Per coin:** 4 × 2 × 3 = 24 configs × 18 coins = 432 backtests

**Key metrics:**
- `decay_exit_rate`: % of regime trades exited by regime decay (vs other exits)
- `decay_exit_PF`: profit factor of trades exited by regime decay
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_held_delta`: change in average hold duration

### RUN82.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best ADX_RISE × REGIME_SHIFT × GRACE_BARS per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Decay exit rate 5–25% of regime trades (meaningful but not dominant)
- Trades exited by decay have lower avg_pnl than trades held to other exits (confirming they were deteriorating)

### RUN82.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Regime Decay Exit | Delta |
|--------|---------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Held Bars | X | X | -N |
| Decay Exits | 0 | X% | — |
| Decay Exit Avg PnL | — | $X | — |
| Other Exit Avg PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **Regime detection is noisy:** ADX fluctuates bar-to-bar. A single bar of elevated ADX doesn't mean the regime has changed — the regime could revert in the next bar. The grace period may not be long enough to filter noise.
2. **StrongTrend can mean-revert too:** A coin in StrongTrend can still mean-revert — the trend can provide the momentum for a sharp reversal. Exiting on regime shift could cut short a winning trade.
3. **ADX rise is a lagging indicator:** By the time ADX has risen 15+ points above entry, the trend may already be well-established. The exit is late rather than early.

---

## Why It Could Succeed

1. **Prevents riding failed mean reversion:** The worst regime trades are the ones where the coin ranged, we entered, and then a sustained trend developed against us. Regime decay detection catches this transition early.
2. **ADX is a clean trend strength indicator:** ADX is specifically designed to measure trend intensity. A rising ADX while in a mean-reversion position is exactly the wrong conditions — exiting preserves capital.
3. **Simple and interpretable:** Just two conditions — regime shift or ADX rise above threshold. Easy to backtest and understand.
4. **Complementary to existing exits:** Adds a proactive exit to complement the reactive SL, SMA, and Z0 exits.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN82 Regime Decay Exit |
|--|--|--|
| Exit reasons | SL, SMA, Z0, MAX_HOLD | SL, SMA, Z0, MAX_HOLD, REGIME_DECAY |
| Regime monitoring | None (only at entry) | Per-bar comparison to entry |
| Trending market handling | Holds until SL or SMA | Exits early when trend builds |
| ADX sensitivity | None | Triggers on ADX rise > 15 |
| Grace period | N/A | 5 bars |
| Avg held bars | X | X − N |
