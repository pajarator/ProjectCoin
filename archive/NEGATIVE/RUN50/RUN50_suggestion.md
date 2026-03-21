# RUN50 — Candle Composition Filter: Volume Profile Imbalance as Entry Quality Gate

## Hypothesis

**Named:** `candle_composition_filter`

**Mechanism:** COINCLAW entries are triggered by indicator conditions (z-score, RSI, Bollinger Bands, VWAP) but ignore the internal composition of the entry candle. A bearish engulfing candle (small open, large close below open, large volume) that triggers an RSI < 30 long entry has different predictive power than a doji candle (open ≈ close, small body, large upper wick) that triggers the same RSI < 30 entry.

**Candle composition filters:**
1. **Body-to-Wick Ratio (BWR):** `|close − open| / (high − low)` — measures how much of the candle's range is a directional move vs a wick
   - High BWR (body dominates) = directional conviction — good for entries
   - Low BWR (wick dominates) = rejection/reversal candle — possibly conflicting with regime trade direction

2. **Upper/Lower Shadow Ratio:** For longs: `lower_shadow / (high − low)` — measures how much the candle was rejected downward before closing higher
   - Large lower shadow = good buying pressure, supportive for LONG entry
   - Large upper shadow = selling pressure at top, potentially conflicts with LONG entry

3. **Volume Confirmation:** Entry candle volume should be ≥ 1.2× rolling average volume
   - Low volume entry = lack of conviction, higher false signal rate

**Filter logic:**
```
For LONG entry:
  if lower_shadow_ratio < MIN_LOWER_SHADOW: block entry (no rejection = weak)
  if upper_shadow_ratio > MAX_UPPER_SHADOW: block entry (selling pressure at top)
  if body_ratio < MIN_BODY_RATIO: block entry (doji = indecision)
  if vol < vol_ma * MIN_ENTRY_VOL_MULT: block entry (low conviction)

For SHORT entry: mirror the logic
```

**Why this is not a duplicate:**
- No prior RUN tested candle composition as entry filter
- No prior RUN used body/wick ratios for regime trades
- RUN15 feature matrix includes shadow/body features but they were never tested as entry gates
- RUN20 (momentum crowding) used 3-day volume ratio — this uses intrabar candle structure

---

## Proposed Config Changes

```rust
// RUN50: Candle Composition Filter parameters
pub const CANDLE_FILTER_ENABLE: bool = true;
pub const MIN_BODY_RATIO: f64 = 0.40;       // body must be ≥40% of total range
pub const MIN_LOWER_SHADOW_LONG: f64 = 0.15;  // lower shadow ≥15% for longs
pub const MIN_UPPER_SHADOW_SHORT: f64 = 0.15; // upper shadow ≥15% for shorts
pub const MAX_UPPER_SHADOW_LONG: f64 = 0.30;  // upper shadow ≤30% for longs (too much rejection)
pub const MAX_LOWER_SHADOW_SHORT: f64 = 0.30;  // lower shadow ≤30% for shorts
pub const MIN_ENTRY_VOL_MULT: f64 = 1.2;      // volume must be ≥1.2× rolling avg
```

**`indicators.rs` — add candle composition fields to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub body_ratio: f64,          // |close - open| / (high - low)
    pub upper_shadow: f64,        // high - max(open, close)
    pub lower_shadow: f64,        // min(open, close) - low
    pub upper_shadow_ratio: f64,  // upper_shadow / (high - low)
    pub lower_shadow_ratio: f64, // lower_shadow / (high - low)
}
```

**`strategies.rs` — add candle filter to `long_entry` and `short_entry`:**
```rust
fn passes_candle_filter(ind: &Ind15m, dir: Direction) -> bool {
    if !config::CANDLE_FILTER_ENABLE { return true; }
    let range = ind.high - ind.low;
    if range <= 0.0 { return true; }  // degenerate candle, skip filter

    match dir {
        Direction::Long => {
            if ind.body_ratio < config::MIN_BODY_RATIO { return false; }
            if ind.lower_shadow_ratio < config::MIN_LOWER_SHADOW_LONG { return false; }
            if ind.upper_shadow_ratio > config::MAX_UPPER_SHADOW_LONG { return false; }
            if ind.vol_ma > 0.0 && ind.vol < ind.vol_ma * config::MIN_ENTRY_VOL_MULT { return false; }
        }
        Direction::Short => {
            if ind.body_ratio < config::MIN_BODY_RATIO { return false; }
            if ind.upper_shadow_ratio < config::MIN_UPPER_SHADOW_SHORT { return false; }
            if ind.lower_shadow_ratio > config::MAX_LOWER_SHADOW_SHORT { return false; }
            if ind.vol_ma > 0.0 && ind.vol < ind.vol_ma * config::MIN_ENTRY_VOL_MULT { return false; }
        }
    }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }
    if !passes_candle_filter(ind, Direction::Long) { return false; }  // NEW
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN50.1 — Candle Composition Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no candle composition filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MIN_BODY_RATIO` | [0.30, 0.40, 0.50, 0.60] |
| `MIN_LOWER_SHADOW_LONG` | [0.10, 0.15, 0.20] |
| `MAX_UPPER_SHADOW_LONG` | [0.25, 0.30, 0.35] |
| `MIN_ENTRY_VOL_MULT` | [1.0, 1.2, 1.5] |

**Per coin:** 4 × 3 × 3 × 3 = 108 configs × 18 coins = 1,944 backtests

**Also test:** Is the candle filter more effective for specific strategies? VwapReversion may be most sensitive to candle composition (VWAP cross + doji = unreliable), while AdrReversal may be less sensitive.

**Key metrics:**
- `filter_block_rate`: % of entries blocked by candle filter
- `filter_quality`: % of blocked entries that would have been losing trades (correct blocks)
- `false_block_rate`: % of blocked entries that would have been winning trades
- `WR_delta`: win rate change vs baseline for non-blocked entries
- `PF_delta`: profit factor change vs baseline

### RUN50.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best candle composition params per coin
2. Test: evaluate on held-out month with those params

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline (among non-blocked trades)
- False block rate < 20% (filter doesn't remove too many good trades)
- Block rate 10–40% (filter isn't too lenient or too aggressive)

### RUN50.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Candle Filter | Delta |
|--------|---------------|--------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K (−X%) |
| Filter Block Rate | 0% | X% | — |
| False Block Rate | — | X% | — |
| Correct Block Rate | — | X% | — |
| Avg Body Ratio (entries) | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Candle composition is coin-specific:** Different coins have different typical candle structures. A MIN_BODY_RATIO of 0.40 might be appropriate for BTC (large bodies) but too strict for SHIB (small bodies in USD terms).
2. **Indicators already capture what candle composition adds:** RSI, z-score, and Bollinger Bands may already be filtering for quality entries. Adding candle filters may just reduce trade count without improving WR%.
3. **Small sample within filter:** After blocking low-quality candles, the remaining trade count may be too small to be statistically significant.

---

## Why It Could Succeed

1. **Doji candles at entry are a known failure mode:** A doji (open ≈ close) that triggers an RSI < 30 entry is contradictory — RSI says oversold but the candle shows indecision. Filtering these out should improve win rate.
2. **Lower shadow confirmation for longs:** A large lower wick means buyers stepped in and rejected the dip. This is mechanically supportive of a LONG entry.
3. **Volume confirmation is proven:** High-volume entries are more reliable than low-volume entries across all timeframes.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN50 Candle Composition Filter |
|--|--|--|
| Entry filter | Indicators only | Indicators + candle structure |
| Body ratio | None | ≥ 40% required |
| Wick confirmation | None | Shadow ratios gated |
| Volume filter | vol_ma in strategies | Enhanced vol mult (1.2×) |
| Expected block rate | 0% | 15–30% |
| Expected WR% delta | — | +2–5pp |
| Implementation | strategies.rs only | strategies.rs + indicators.rs |
