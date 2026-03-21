# RUN65 — BB Squeeze Duration Filter: Entry Quality Based on Compression Time

## Hypothesis

**Named:** `squeeze_duration_filter`

**Mechanism:** The Bollinger Band squeeze (BB width < BB width average × 0.6) indicates low volatility and price compression. When a squeeze resolves (BB width crosses back above average), price typically explodes in one direction. For mean-reversion trades, the direction of the explosion matters:

- **Squeeze resolving upward** (price breaks above BB upper): good for LONG, bad for SHORT
- **Squeeze resolving downward** (price breaks below BB lower): good for SHORT, bad for LONG

But more importantly: **how long the squeeze lasted before resolution** is a quality signal:
- **Short squeeze (1-3 bars):** Brief compression = low conviction. Price may not have enough stored energy for a meaningful move.
- **Long squeeze (8+ bars):** Prolonged compression = high energy buildup. When it releases, the move is stronger and more sustained.

**Filter logic:**
```
squeeze_bars = count of consecutive bars with bb_width < bb_width_avg × 0.6

For LONG entry:
  if squeeze_bars < MIN_SQUEEZE_BARS: block entry (squeeze too brief)
  if squeeze_bars >= MIN_SQUEEZE_BARS AND price just broke above BB upper:
    → strong LONG entry (squeeze released upward)
```

**Why this is not a duplicate:**
- RUN12 detected squeeze and blocked ALL entries — this uses squeeze as an ENTRY quality signal
- No prior RUN measured squeeze duration before entry
- BB_bounce strategy fires when price is at lower BB, but doesn't check how long the preceding squeeze lasted

---

## Proposed Config Changes

```rust
// RUN65: BB Squeeze Duration Filter
pub const SQUEEZE_DURATION_ENABLE: bool = true;
pub const MIN_SQUEEZE_BARS: u32 = 4;   // require ≥4 bars of squeeze before entry
pub const SQUEEZE_EXIT_MODE: u8 = 1;   // 1=strict (BB break required), 2=relaxed (any squeeze)
```

**`indicators.rs` — add squeeze tracking to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub bb_width: f64,
    pub bb_width_avg: f64,
    pub squeeze_bars: u32,   // consecutive bars in current squeeze
}
```

**`strategies.rs` — squeeze duration filter:**
```rust
fn passes_squeeze_filter(ind: &Ind15m, dir: Direction) -> bool {
    if !config::SQUEEZE_DURATION_ENABLE { return true; }
    if ind.squeeze_bars.is_nan() { return true; }

    if ind.squeeze_bars < config::MIN_SQUEEZE_BARS {
        return false;  // squeeze too brief — low energy release
    }

    if config::SQUEEZE_EXIT_MODE == 1 {
        // Strict: require price to have recently broken out of BB (within last 2 bars)
        // For LONG: require price > BB upper (just broke out upside)
        match dir {
            Direction::Long => {
                if ind.p <= ind.bb_upper { return false; }
            }
            Direction::Short => {
                if ind.p >= ind.bb_lower { return false; }
            }
        }
    }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    // ... existing checks ...
    if !passes_squeeze_filter(ind, Direction::Long) { return false; }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN65.1 — Squeeze Duration Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no squeeze duration filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MIN_SQUEEZE_BARS` | [3, 4, 6, 8] |
| `SQUEEZE_EXIT_MODE` | [1=strict, 2=relaxed] |

**Per coin:** 4 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `squeeze_detection_rate`: % of bars that are in a squeeze
- `squeeze_entry_rate`: % of LONG/SHORT entries that occur during/after squeeze
- `squeeze_duration_hit_rate`: % of squeeze entries that meet MIN_SQUEEZE_BARS
- `WR_delta`: win rate change vs baseline for non-blocked entries
- `PF_delta`: profit factor change vs baseline

### RUN65.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `MIN_SQUEEZE_BARS` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- Squeeze duration hit rate ≥ 60% (the filter is actually binding)
- Portfolio OOS P&L ≥ baseline

### RUN65.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Squeeze Duration Filter | Delta |
|--------|---------------|----------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Squeeze Block Rate | 0% | X% | — |
| Avg Squeeze Bars | X | X | +N |
| Squeeze Duration Hit Rate | — | X% | — |

---

## Why This Could Fail

1. **Squeeze duration doesn't predict breakout quality:** Long squeezes don't always produce big breakouts. The stored energy theory is intuitive but may not hold in crypto markets.
2. **Strict mode (BB break required) is too limiting:** Requiring price to be above BB upper at entry means the best part of the move has already happened. The entry is late.
3. **BB width calculation is noisy:** On 15m bars, BB width can fluctuate. The squeeze threshold (0.6 × average) may not be stable.

---

## Why It Could Succeed

1. **Intuitively sound:** The "compression → explosion" principle is one of the oldest in technical analysis. The longer the compression, the bigger the eventual move.
2. **Addresses timing:** Current BB_bounce fires when price is at the lower band — it doesn't care whether the compression was building for 1 bar or 10. Squeeze duration adds this dimension.
3. **Orthogonal to other filters:** BB squeeze is independent of z-score, RSI, and volume. It's a different signal dimension.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN65 Squeeze Duration Filter |
|--|--|--|
| Squeeze check | None | Duration ≥ 4 bars |
| BB break requirement | None | Price must be outside BB (strict mode) |
| LONG quality | All equal | Squeeze-duration qualified |
| SHORT quality | All equal | Squeeze-duration qualified |
| Expected block rate | 0% | 15–25% |
| Expected WR% delta | — | +2–5pp |
| Implementation | — | indicators.rs + strategies.rs |
