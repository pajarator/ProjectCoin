# RUN111 — MACD Histogram Slope Exit: Exit When MACD Histogram Confirms Mean Reversion Is Complete

## Hypothesis

**Named:** `macd_histogram_exit`

**Mechanism:** COINCLAW uses z-score extremes and regime indicators to enter mean-reversion trades. But there is no confirmation that the mean-reversion has actually run its course — the system exits on Z0 reversion, SMA20 crossback, or MAX_HOLD. The MACD Histogram Slope Exit adds a momentum confirmation: the MACD histogram (the divergence between fast and slow EMA) measures the rate of change in the mean-reversion momentum. When the MACD histogram crosses zero after an entry, it signals that the short-term momentum that drove the mean-reversion has been exhausted and is now reverting — a natural exit signal.

**MACD Histogram Slope Exit:**
- Track MACD histogram at entry: `hist_entry = macd_line - signal_line` at entry bar
- Exit when: `hist_current * hist_entry < 0` (histogram crossed zero — momentum flipped)
- Or: `hist_current < hist_entry * MACD_HIST_TRAIL` (histogram has tapered by threshold)
- This captures when the initial mean-reversion momentum has been "absorbed" by the market

**Why this is not a duplicate:**
- RUN99 (z-momentum divergence) used z-score momentum divergence — this uses MACD histogram, a different momentum indicator
- RUN103 (stochastic extreme exit) used stochastic %K — this uses MACD histogram slope
- RUN104 (volume dryup exit) used volume — this uses MACD histogram momentum
- RUN77 (recovery rate exit) used z-score recovery velocity — this uses MACD histogram crossover
- No prior RUN has used MACD histogram slope or crossover as an exit condition

**Mechanistic rationale:** Mean-reversion trades work because price diverged from the mean and must return. The MACD histogram captures the velocity of that divergence and subsequent reversion. When the histogram crosses zero after an entry, it means the short-term momentum that was driving price toward the mean has been exhausted and the market is entering a new phase — a natural exit point. This is more responsive than waiting for SMA20 crossback or Z0 reversion, and more signal-dense than MAX_HOLD.

---

## Proposed Config Changes

```rust
// RUN111: MACD Histogram Slope Exit
pub const MACD_HIST_EXIT_ENABLE: bool = true;
pub const MACD_HIST_TRAIL: f64 = 0.50;   // exit when hist drops to 50% of entry hist (momentum faded)
pub const MACD_HIST_FLIP_EXIT: bool = true; // exit on histogram zero-cross (momentum flipped)
```

**`strategies.rs` — add MACD histogram helper:**
```rust
/// Compute MACD histogram (macd_line - signal_line) for 15m.
fn macd_hist(ind: &Ind15m) -> f64 {
    if ind.macd.is_nan() || ind.macd_signal.is_nan() {
        return 0.0;
    }
    ind.macd - ind.macd_signal
}

/// Check if MACD histogram exit condition is met.
fn check_macd_hist_exit(cs: &CoinState, entry_hist: f64) -> bool {
    if !config::MACD_HIST_EXIT_ENABLE { return false; }
    let current_hist = macd_hist_from_ind(&cs.ind_15m);
    if current_hist.is_nan() || current_hist == 0.0 { return false; }

    // Zero-cross exit: histogram flipped sign
    if config::MACD_HIST_FLIP_EXIT && (entry_hist > 0.0 && current_hist < 0.0
        || entry_hist < 0.0 && current_hist > 0.0) {
        return true;
    }

    // Trail exit: histogram faded to threshold of entry value
    if config::MACD_HIST_TRAIL > 0.0 && entry_hist.abs() > 0.0 {
        let fade_ratio = current_hist.abs() / entry_hist.abs();
        if fade_ratio < config::MACD_HIST_TRAIL {
            return true;
        }
    }

    false
}
```

**`engine.rs` — modify check_exit to include MACD histogram exit:**
```rust
/// In check_exit, after Z0 and SMA20 checks, add:
if config::MACD_HIST_EXIT_ENABLE && trade.entry_histogram != 0.0 {
    if check_macd_hist_exit(cs, trade.entry_histogram) {
        return ExitReason::MacdHistExit;
    }
}
```

**`state.rs` — TradeRecord addition:**
```rust
pub struct TradeRecord {
    // ... existing fields ...
    pub entry_histogram: f64,  // MACD histogram at entry for hist-exit tracking
}
```

---

## Validation Method

### RUN111.1 — MACD Histogram Exit Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — exit on Z0/SMA20/MAX_HOLD

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MACD_HIST_TRAIL` | [0.30, 0.50, 0.70, 0.0 (off)] |
| `MACD_HIST_FLIP_EXIT` | [true, false] |

**Per coin:** 4 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `hist_exit_rate`: % of trades exiting via MACD histogram
- `hist_exit_wr`: win rate of trades exiting on MACD histogram
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_hold_bars_delta`: change in average hold duration

### RUN111.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best MACD_HIST_TRAIL × FLIP combo per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- MACD histogram exits have higher win rate than Z0/SMA20 exits
- Average hold time decreases (more responsive exits)

### RUN111.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, Z0/SMA/MAX) | MACD Histogram Exit | Delta |
|--------|--------------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Histogram Exits | 0% | X% | — |
| Z0 Exits | X% | X% | -Y% |
| SMA20 Exits | X% | X% | -Y% |
| Max Hold Exits | X% | X% | -Y% |
| Avg Hold Bars | X | X | -N |
| Histogram Exit WR% | — | X% | — |

---

## Why This Could Fail

1. **MACD is lagging:** MACD is a trend-following indicator. Using it to exit mean-reversion trades may be contradictory — by the time MACD flips, the mean-reversion has already run its course and the trade may be giving back profits.
2. **Histogram noise:** The MACD histogram can oscillate around zero multiple times in a volatile market, causing premature exits if the threshold is too tight.
3. **Duplicate signal:** Z0 reversion and SMA20 crossback already capture mean-reversion completion. Adding MACD histogram may be redundant and not add incremental information.

---

## Why It Could Succeed

1. **Captures momentum exhaustion:** The MACD histogram measures the rate of change in momentum. When the histogram crosses zero, the momentum that was driving the mean-reversion has been absorbed — a natural exit point.
2. **Earlier than SMA20:** MACD histogram can flip before SMA20 crosses, giving a more responsive exit and capturing more profit.
3. **Different information than Z0:** Z0 is based on z-score reversion to the mean. MACD histogram captures the *momentum* of that reversion — a different dimension. A trade can be at Z0 (z-score reverted) but still have bullish MACD momentum.
4. **Simple and interpretable:** One zero-cross check. Clear meaning: "exit when the momentum that drove this trade has flipped."

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN111 MACD Histogram Exit |
|--|--|--|
| Exit triggers | Z0, SMA20, MAX_HOLD, BE | Z0, SMA20, MAX_HOLD, BE + MACD hist |
| Exit sensitivity | Fixed | Adaptive to momentum |
| MACD histogram exit | None | On zero-cross or trail |
| Hold duration | Variable | Shorter (more responsive) |
| Momentum awareness | None | MACD histogram slope |
| Exit signal quality | Z-score based | Z-score + momentum |
