# RUN105 — Z-Score Persistence: Require Sustained Extreme Z-Score for N Consecutive Bars Before Entry

## Hypothesis

**Named:** `z_persistence_filter`

**Mechanism:** COINCLAW currently enters when z-score crosses a threshold (e.g., z < -1.5) at a single point in time. But z-score can briefly spike to extreme levels due to noise without representing a genuine mean-reversion opportunity. The Z-Score Persistence filter requires that the z-score be at or beyond the entry threshold for N consecutive bars before the entry is confirmed — filtering out momentary spikes and only allowing entries where the extreme reading is sustained.

**Z-Score Persistence:**
- Track `z_persistence_bars` — how many consecutive bars z-score has been at or beyond the entry threshold
- For LONG entry: require `z < -Z_PERSIST_THRESHOLD` for `Z_PERSIST_BARS` consecutive bars
- For SHORT entry: require `z > Z_PERSIST_THRESHOLD` for `Z_PERSIST_BARS` consecutive bars
- The first bar where z crosses the threshold starts the counter; if z ever returns to neutral (|z| < threshold) the counter resets
- Example: z = -1.6, z = -1.7, z = -1.5, z = -1.8 → counter resets to 0 at bar 3

**Why this is not a duplicate:**
- No prior RUN has required consecutive bar confirmation of z-score before entry
- F6 filter (RUN10) checks price momentum in a single bar — this checks z-score stability across multiple bars
- No prior RUN has used temporal persistence of the entry signal itself

**Mechanistic rationale:** A momentary z = -1.8 spike that reverts in one bar is noise — it doesn't represent a genuine oversold opportunity. But z = -1.8 sustained for 3 consecutive bars represents a persistent oversold condition with real mean-reversion probability. Requiring persistence filters out noise trades and only allows entries where the signal is confirmed by time.

---

## Proposed Config Changes

```rust
// RUN105: Z-Score Persistence Filter
pub const Z_PERSIST_ENABLE: bool = true;
pub const Z_PERSIST_THRESHOLD: f64 = 1.5;  // z must be at or beyond this level
pub const Z_PERSIST_BARS: u32 = 2;           // for N consecutive bars
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub z_persist_count: i32,  // consecutive bars z has been at/beyond threshold (-1 = invalid)
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
/// Check if z-score has been persistently at the extreme level.
fn z_persistence_met(cs: &CoinState, ind: &Ind15m, is_long: bool) -> bool {
    if !config::Z_PERSIST_ENABLE { return true; }

    let thresh = config::Z_PERSIST_THRESHOLD;
    let bars = config::Z_PERSIST_BARS as i32;

    // Check current bar
    let current_extreme = if is_long {
        ind.z < -(thresh)
    } else {
        ind.z > thresh
    };
    if !current_extreme { return false; }

    // Check consecutive bar counter
    // If counter was already counting and is now >= bars, we're good
    let prev_count = cs.z_persist_count;
    if prev_count >= bars - 1 {
        return true;
    }
    false
}

// Update z_persistence counter each bar
fn update_z_persistence(cs: &mut CoinState, ind: &Ind15m) {
    let thresh = config::Z_PERSIST_THRESHOLD;
    let is_long_extreme = ind.z < -(thresh);
    let is_short_extreme = ind.z > thresh;

    if is_long_extreme || is_short_extreme {
        if cs.z_persist_count < 0 {
            cs.z_persist_count = 1;  // start counting
        } else {
            cs.z_persist_count += 1;
        }
    } else {
        cs.z_persist_count = -1;  // reset
    }
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // Base entry check
    let base_ok = match strat {
        LongStrat::VwapReversion => ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2,
        LongStrat::BbBounce => ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3,
        LongStrat::DualRsi => ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20,
        LongStrat::AdrReversal => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_lo.is_nan() && range > 0.0
                && ind.p <= ind.adr_lo + range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
        LongStrat::MeanReversion => ind.z < -1.5,
        LongStrat::OuMeanRev => {
            !ind.ou_halflife.is_nan() && !ind.ou_deviation.is_nan()
                && ind.std20 > 0.0
                && ind.ou_halflife >= config::OU_MIN_HALFLIFE
                && ind.ou_halflife <= config::OU_MAX_HALFLIFE
                && (ind.ou_deviation / ind.std20) < -config::OU_DEV_THRESHOLD
        }
    };

    if !base_ok { return false; }

    // Z-Persistence check (for long)
    z_persistence_met(cs, ind, true)
}
```

---

## Validation Method

### RUN105.1 — Z-Persistence Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — single-bar z threshold check

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_PERSIST_THRESHOLD` | [1.3, 1.5, 1.8] |
| `Z_PERSIST_BARS` | [2, 3, 4] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `persistence_filter_rate`: % of single-bar signals filtered out by persistence requirement
- `z_at_entry_delta`: change in average |z| at entry (should increase — more extreme entries)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN105.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best PERSIST_THRESHOLD × BARS per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Entry count decreases (filtering noise) but avg |z| at entry increases
- Win rate improves vs baseline

### RUN105.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, single-bar) | Z-Persistence Filter | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg Z at Entry | X | X | +N |
| Persistence Count at Entry | 1 | X | +N |

---

## Why This Could Fail

1. **Persistence delays entries:** By requiring 2+ consecutive bars, entries are delayed by 15-60 minutes. The optimal entry moment is when z first reaches extreme — requiring persistence means entering later at a less extreme price.
2. **Z-score oscillations:** During volatile markets, z-score can oscillate around the threshold, never sustaining N bars above threshold. This could reduce entry frequency significantly without improving quality.
3. **Optimal persistence is regime-dependent:** In high-volatility regimes, z oscillates more and persistence is harder to achieve. The optimal N may vary with regime type.

---

## Why It Could Succeed

1. **Filters momentary noise:** The biggest source of bad entries is momentary z spikes that revert in 1-2 bars. Persistence cleanly filters these without requiring new indicators.
2. **Better entries = higher quality:** Entries where z has been sustained at extreme are more likely to be genuine mean-reversion setups. The persistence filter ensures the opportunity is real, not just a fleeting spike.
3. **Increases edge:** By only entering when the signal is confirmed by time, the average entry z-score is more extreme and the probability of successful mean reversion is higher.
4. **Simple and intuitive:** "Wait N bars to confirm" is a natural way to filter noise. No new indicators needed.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN105 Z-Persistence Filter |
|--|--|--|
| Entry confirmation | Single bar z-crossing | N consecutive bars at extreme |
| Signal noise | Unfiltered | Filtered |
| Avg Z at entry | -1.5 to -2.0 | -1.8 to -2.5 |
| Entry latency | Immediate | Delayed N bars |
| Persistence check | None | 2-4 bars required |
| Implementation | None | Counter per coin |
