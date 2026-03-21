# RUN107 — Percentile Rank Z-Filter: Require Z-Score to Be at Historical Percentile Extreme

## Hypothesis

**Named:** `percentile_z_filter`

**Mechanism:** COINCLAW currently uses fixed z-score thresholds (e.g., z < -1.5) to trigger entries. But a z-score of -1.5 means different things in different volatility regimes: in a high-volatility period, z = -1.5 might be common (not particularly extreme), while in a low-volatility period, z = -1.5 might be rare (very extreme). The Percentile Rank Z-Filter requires that the current z-score be at or beyond a historical percentile threshold (e.g., the top 10% most extreme readings for that coin over the last 100 bars) — normalizing z-score by its own historical distribution.

**Percentile Rank Z-Filter:**
- Track rolling 100-bar z-score history for each coin
- Compute percentile rank: `pct_rank = % of historical z-scores more extreme than current`
- For LONG entry: require `pct_rank <= PERCENTILE_LONG` (e.g., 10) — current z is in the most extreme 10% of historical readings
- For SHORT entry: require `pct_rank >= (100 - PERCENTILE_SHORT)` (e.g., 90) — current z is in the most extreme 10% on the upside
- This normalizes the entry signal across different volatility regimes

**Why this is not a duplicate:**
- RUN52 (z-confidence sizing) sized positions by z-score magnitude — this filters by z-score percentile rank in the coin's own history
- RUN76 (vol-adaptive SL) used ATR percentile rank — this uses z-score percentile rank
- RUN54 (vol entry threshold) used ATR percentile rank — no prior RUN has used z-score percentile rank
- No prior RUN has normalized z-score entry conditions by their historical distribution

**Mechanistic rationale:** A fixed z-threshold of -1.5 is not comparable across time or across coins. A z-score of -1.5 when the historical std dev is 2.0 is a much more extreme reading than z = -1.5 when std dev is 0.5. Percentile rank normalizes this — requiring the top 10% most extreme readings ensures the entry is genuinely extreme relative to the coin's own recent history.

---

## Proposed Config Changes

```rust
// RUN107: Percentile Rank Z-Filter
pub const PERCENTILE_Z_ENABLE: bool = true;
pub const PERCENTILE_Z_WINDOW: usize = 100;   // rolling window for z-score history
pub const PERCENTILE_LONG: f64 = 10.0;         // for LONG: current z must be at or beyond this percentile (most extreme 10%)
pub const PERCENTILE_SHORT: f64 = 10.0;        // for SHORT: current z must be at or beyond this percentile
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub z_history: Vec<f64>,  // rolling 100-bar z-score history
}
```

**`strategies.rs` — percentile_z_filter:**
```rust
/// Compute the percentile rank of current z-score in the rolling window.
fn z_percentile_rank(cs: &CoinState) -> f64 {
    let current_z = match &cs.ind_15m {
        Some(ind) => ind.z,
        None => return 50.0,
    };

    let hist = &cs.z_history;
    if hist.len() < 20 {
        return 50.0;  // not enough history
    }

    // Count how many historical z-scores are more extreme than current
    let more_extreme = hist.iter().filter(|&&z| {
        if current_z < 0.0 {
            z < current_z  // for negative z, more extreme = more negative
        } else {
            z > current_z  // for positive z, more extreme = more positive
        }
    }).count();

    (more_extreme as f64 / hist.len() as f64) * 100.0
}

/// Update z-history each bar.
fn update_z_history(cs: &mut CoinState) {
    if let Some(ref ind) = cs.ind_15m {
        if !ind.z.is_nan() {
            cs.z_history.push(ind.z);
            if cs.z_history.len() > config::PERCENTILE_Z_WINDOW {
                cs.z_history.remove(0);
            }
        }
    }
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // Base entry check (existing logic)
    let base_ok = match strat {
        // ... existing long entry conditions ...
        _ => false,
    };
    if !base_ok { return false; }

    // RUN107: Percentile rank check
    if config::PERCENTILE_Z_ENABLE {
        let pct = z_percentile_rank(cs);
        if pct > config::PERCENTILE_LONG {
            return false;  // not extreme enough relative to history
        }
    }

    true
}
```

---

## Validation Method

### RUN107.1 — Percentile Z Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed z-threshold (e.g., -1.5)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `PERCENTILE_Z_WINDOW` | [50, 100, 200] |
| `PERCENTILE_LONG` | [5.0, 10.0, 15.0] |
| `PERCENTILE_SHORT` | [5.0, 10.0, 15.0] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `percentile_filter_rate`: % of fixed-threshold entries filtered out by percentile requirement
- `avg_z_at_entry_delta`: change in average |z| at entry (should increase if percentile is working)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN107.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best PERCENTILE_LONG × PERCENTILE_SHORT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Entry count decreases (filtering less-extreme entries) but win rate increases

### RUN107.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed -1.5) | Percentile Z-Filter | Delta |
|--------|--------------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg Z at Entry | X | X | +N |
| Percentile at Entry | — | X% | — |

---

## Why This Could Fail

1. **Z-score is mean-reverting by nature:** Z-score spends equal time at all percentile levels by definition. The top 10% most extreme readings are rare by design — requiring them may filter out too many valid opportunities.
2. **History is not predictive:** The fact that z-score was in the top 10% historically doesn't mean it will mean-revert in the next bar. The percentile is descriptive, not predictive.
3. **Implementation complexity:** Tracking z-history per coin adds state management overhead.

---

## Why It Could Succeed

1. **Normalizes across volatility regimes:** In high-volatility periods, z = -1.5 is not extreme. Percentile rank captures this — the top 10% threshold adapts to the current volatility regime.
2. **Better than fixed thresholds:** Fixed thresholds are arbitrary. Percentile rank uses the coin's own history as the reference, making the threshold adaptive and data-driven.
3. **Cross-coin comparability:** z = -1.5 on BTC is different from z = -1.5 on SHIB. Percentile rank makes the "extremeness" comparable across coins.
4. **Simple and principled:** One percentile threshold, clear meaning: "enter only when z is in the most extreme X% of recent readings."

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN107 Percentile Z-Filter |
|--|--|--|
| Z-entry threshold | Fixed -1.5 | Adaptive percentile (top 10% most extreme) |
| Regime adaptation | None (fixed) | Adaptive to recent volatility |
| Cross-coin comparability | Poor (different std devs) | Good (percentile normalizes) |
| Entry filter | z-crossing | z-crossing + historical percentile |
| Volatility awareness | None | Z-history percentile |
