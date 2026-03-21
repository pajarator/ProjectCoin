# RUN104 — Volume Dry-Up Exit: Exit When Volume Collapses During Profitable Trades

## Hypothesis

**Named:** `volume_dryup_exit`

**Mechanism:** COINCLAW regime trades rely on mean reversion — price oscillating around a mean. But mean reversion requires market participation (volume) to sustain a move. When volume collapses while we're in a profitable position, the move lacks conviction — the price may be floating on low volume without real support. The Volume Dry-Up Exit exits when volume drops below a threshold while we're in a profitable position.

**Volume Dry-Up Exit:**
- Track rolling average volume for each coin (e.g., 20-bar)
- When in an open regime position that is profitable (pnl > 0):
  - If `current_volume < vol_ma * VOL_DRYUP_THRESHOLD` (e.g., 0.40) for `VOL_DRYUP_BARS` consecutive bars
  - AND `held >= VOL_DRYUP_MIN_HOLD` bars → exit with reason `VOL_DRYUP`
- Rationale: if we're in a profitable position and volume has dried up, the remaining profit potential is low and the risk of reversal is high
- Scalp trades exempt (they operate on very short timeframes where volume is inherently noisy)

**Why this is not a duplicate:**
- RUN72 (choppy mode) suppressed scalp entries when market ATR was low — this exits positions when volume dries up during a trade
- RUN80 (volume imbalance) used volume imbalance for entry filtering — this uses absolute volume collapse as an exit signal
- No prior RUN has used volume collapse as an exit criterion for regime trades

**Mechanistic rationale:** Volume is the fuel for price moves. When volume collapses during a profitable position, the price movement lacks market participation — it could easily reverse. Exiting when volume dries up locks in profits before the inevitable reversal, rather than waiting for SMA/Z0 which may come too late.

---

## Proposed Config Changes

```rust
// RUN104: Volume Dry-Up Exit
pub const VOL_DRYUP_EXIT_ENABLE: bool = true;
pub const VOL_DRYUP_THRESHOLD: f64 = 0.40;   // exit when vol < vol_ma * 0.40
pub const VOL_DRYUP_BARS: u32 = 2;           // must be below threshold for 2 consecutive bars
pub const VOL_DRYUP_MIN_HOLD: u32 = 8;      // minimum bars before dryup exit can fire
```

**`engine.rs` — check_volume_dryup_exit:**
```rust
/// Check if volume has dried up for a regime position.
fn check_volume_dryup(state: &mut SharedState, ci: usize, ind: &Ind15m, pos: &Position) -> bool {
    if !config::VOL_DRYUP_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }

    let held = state.coins[ci].candles_held;
    if held < config::VOL_DRYUP_MIN_HOLD { return false; }

    // Must be profitable
    let pnl = if pos.dir == "long" {
        (ind.p - pos.e) / pos.e
    } else {
        (pos.e - ind.p) / pos.e
    };
    if pnl <= 0.0 { return false; }

    // Check volume ratio
    let vol_ratio = if ind.vol_ma > 0.0 {
        ind.vol / ind.vol_ma
    } else {
        return false;
    };

    if vol_ratio < config::VOL_DRYUP_THRESHOLD {
        return true;
    }

    false
}

// In check_exit — add before MAX_HOLD check:
if check_volume_dryup(state, ci, &ind, &pos) {
    close_position(state, ci, ind.p, "VOL_DRYUP", TradeType::Regime);
    return true;
}
```

---

## Validation Method

### RUN104.1 — Volume Dry-Up Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no volume-based exits

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VOL_DRYUP_THRESHOLD` | [0.30, 0.40, 0.50] |
| `VOL_DRYUP_BARS` | [1, 2, 3] |
| `VOL_DRYUP_MIN_HOLD` | [5, 8, 12] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `dryup_exit_rate`: % of regime trades exited by VOL_DRYUP
- `dryup_exit_win_rate`: win rate of dryup exits (should be high — profit-taking)
- `PF_delta`: profit factor change vs baseline
- `avg_pnl_delta`: change in average exit PnL
- `total_PnL_delta`: P&L change vs baseline

### RUN104.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold combinations per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- VOL_DRYUP exit win rate >70% (these are profit-taking exits)
- Dryup exit rate 5–20% of regime trades

### RUN104.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no vol exit) | Volume Dry-Up Exit | Delta |
|--------|-----------------------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Dryup Exit Rate | 0% | X% | — |
| Dryup Exit Win Rate | — | X% | — |
| Dryup Exit Avg PnL | — | $X | — |
| Other Exit Avg PnL | $X | $X | +$X |

---

## Why This Could Fail

1. **Volume is inherently noisy:** On 15m bars, volume can fluctuate significantly. A single low-volume bar doesn't mean the move is exhausted — it could just be a quieter moment.
2. **Mean reversion doesn't need high volume:** Price can mean-revert on low volume if the prior move was also on low volume. Volume collapse doesn't necessarily invalidate the mean reversion thesis.
3. **Early exit cuts winners short:** By requiring volume to be above average to hold a position, we may exit profitable trades prematurely when volume naturally fluctuates lower.

---

## Why It Could Succeed

1. **Volume is the only non-price confirmation:** All other exits (SL, SMA, Z0) are price-based. Volume provides an orthogonal signal — if price is moving but volume isn't confirming, the move is suspect.
2. **Institutional practice:** Volume confirmation is standard in technical analysis. Rising prices on declining volume are a classic warning sign.
3. **Profit-taking mechanism:** By only exiting when profitable, this is purely a mechanism to lock in gains when the market shows signs of exhaustion.
4. **Simple and additive:** One additional comparison per bar. Low complexity, clear logic.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN104 Volume Dry-Up Exit |
|--|--|--|
| Volume awareness | None | Volume collapse triggers exit |
| Exit condition | SMA, Z0, SL, MAX_HOLD | SMA, Z0, SL, MAX_HOLD, VOL_DRYUP |
| Trade requirement | None | Must be profitable |
| Volume threshold | N/A | vol < vol_ma × 0.40 |
| Profit-taking signal | None | Volume exhaustion |
