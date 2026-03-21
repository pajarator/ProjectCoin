# RUN71 — BB Width Percentile Rank Filter: Historically Compressed Bands Only

## Hypothesis

**Named:** `bb_width_percentile_filter`

**Mechanism:** The current squeeze detection (`bb_width < bb_width_avg × 0.6`) uses a fixed ratio threshold relative to the recent average. This is relative to recent average but doesn't capture how *historically extreme* the current compression is.

**BB Width Percentile Rank:** What if the current BB width is at the 5th percentile of its own 3-month distribution? That's a historically extreme compression — price is more likely to explode out of it. If it's at the 40th percentile, the compression is not unusual.

**Filter logic:**
```
bb_width_percentile = rank of current bb_width within last N bars (0-100%)
For LONG/SHORT entry:
  if bb_width_percentile > BB_WIDTH_PCT_THRESHOLD:
    block entry (BBs are too wide — not compressed enough)
```

**Why this is not a duplicate:**
- No prior RUN has measured BB width percentile rank as an entry filter
- RUN65 (squeeze duration) used consecutive bar count — this uses percentile of width itself
- Squeeze detection uses fixed ratio; percentile rank is adaptive to each coin's own vol profile

---

## Proposed Config Changes

```rust
// RUN71: BB Width Percentile Rank Filter
pub const BB_PCT_FILTER_ENABLE: bool = true;
pub const BB_WIDTH_PCT_WINDOW: u32 = 200;  // ~3 days of 15m bars
pub const BB_WIDTH_PCT_THRESHOLD: f64 = 0.30;  // block if bb_width > 30th percentile
```

**`indicators.rs` — add bb_width_percentile to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub bb_width: f64,
    pub bb_width_avg: f64,
    pub bb_width_pct_rank: f64,  // 0.0 to 1.0 — percentile rank of current width
}
```

**`strategies.rs` — percentile filter in entry:**
```rust
fn passes_bb_pct_filter(ind: &Ind15m) -> bool {
    if !config::BB_PCT_FILTER_ENABLE { return true; }
    if ind.bb_width_pct_rank.is_nan() { return true; }
    if ind.bb_width_pct_rank > config::BB_WIDTH_PCT_THRESHOLD {
        return false;  // BBs are too wide — not compressed enough
    }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    // ... existing checks ...
    if !passes_bb_pct_filter(ind) { return false; }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN71.1 — BB Percentile Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BB width percentile filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BB_WIDTH_PCT_WINDOW` | [100, 200, 300] |
| `BB_WIDTH_PCT_THRESHOLD` | [0.20, 0.30, 0.40, 0.50] |

**Per coin:** 3 × 4 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `bb_pct_block_rate`: % of entries blocked by BB percentile filter
- `WR_delta`: win rate change vs baseline
- `PF_delta`: profit factor change vs baseline
- `false_block_rate`: % of blocked entries that would have been winners

### RUN71.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `BB_WIDTH_PCT_WINDOW × BB_WIDTH_PCT_THRESHOLD` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- False block rate < 25%

### RUN71.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | BB Percentile Filter | Delta |
|--------|---------------|---------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Trade Count | N | M | −K |
| BB Block Rate | 0% | X% | — |
| False Block Rate | — | X% | — |
| Avg BB Width Pct | X% | X% | — |
