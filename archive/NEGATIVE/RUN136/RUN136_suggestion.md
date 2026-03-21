# RUN136 — Trade Zone Index Confirmation: Require Price to Be In or Breaking Out of a Consolidation Zone

## Hypothesis

**Named:** `tzi_confirm`

**Mechanism:** COINCLAW's regime trades are mean-reversion strategies — they work best when price is oscillating within a range (consolidation) rather than trending. The Trade Zone Index (TZI) measures what percentage of the last N bars have been "in a trade zone" — defined as price being between the high and low of the prior N bars. When TZI is high (e.g., > 70%), the market is consolidating — price is ranging, ideal for mean-reversion. When TZI is low (e.g., < 30%), price has been trending — not ideal for mean-reversion entries. The Trade Zone Index Confirmation requires TZI to be above a threshold for entries, ensuring the market is in a consolidation zone where mean-reversion has higher probability.

**Trade Zone Index Confirmation:**
- Track TZI on 15m: `TZI = count of bars where price is within (min of last N lows, max of last N highs) / N × 100`
- For regime entries: require `tzi > TZI_THRESH` (e.g., > 60 = market is consolidating)
- TZI measures consolidation directly — when price has been ranging within a band, TZI is high

**Why this is not a duplicate:**
- RUN124 (Choppiness Index) measured choppiness via ATR/Range — TZI directly measures whether price is within recent High/Low bounds
- RUN114 (Aroon) used time-since-high/low — TZI uses High/Low bounds to define consolidation zone, different calculation
- No prior RUN has used Trade Zone Index as an entry filter for regime trades

**Mechanistic rationale:** Mean-reversion works best in consolidating markets. When TZI is high (> 60%), it means price has been bouncing within a range for the last N bars — classic consolidation. In this environment, mean-reversion trades have room to work because price hasn't trended away. When TZI is low (< 30%), price has been breaking out of its range — trending — and mean-reversion trades are fighting the trend. TZI directly measures consolidation, giving COINCLAW a simple gate: "don't mean-revert when the market is trending; only when it's consolidating."

---

## Proposed Config Changes

```rust
// RUN136: Trade Zone Index Confirmation
pub const TZI_ENABLE: bool = true;
pub const TZI_WINDOW: usize = 20;            // lookback window for TZI (number of bars)
pub const TZI_THRESH: f64 = 60.0;          // TZI must be above this for entries (market consolidating)
```

**`strategies.rs` — add TZI helper:**
```rust
/// Compute Trade Zone Index: % of bars within the recent high/low range.
fn compute_tzi(candles: &[Candle15m], window: usize) -> f64 {
    if candles.len() < window {
        return 50.0;  // not enough data
    }

    // Find min low and max high over the window
    let start = candles.len() - window;
    let mut min_low = f64::MAX;
    let mut max_high = f64::MIN;

    for i in start..candles.len() {
        min_low = min_low.min(candles[i].l);
        max_high = max_high.max(candles[i].h);
    }

    // Count bars where close is within [min_low, max_high]
    let range_size = max_high - min_low;
    if range_size == 0.0 {
        return 50.0;
    }

    let in_zone = candles[start..candles.len()]
        .iter()
        .filter(|c| c.c >= min_low && c.c <= max_high)
        .count();

    (in_zone as f64 / window as f64) * 100.0
}

/// Check if TZI confirms market is consolidating (good for mean-reversion).
fn tzi_confirm(cs: &CoinState) -> bool {
    if !config::TZI_ENABLE { return true; }
    let tzi = compute_tzi(&cs.candles_15m, config::TZI_WINDOW);
    tzi > config::TZI_THRESH
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN136: Trade Zone Index Confirmation
    if !tzi_confirm(cs) { return false; }

    match strat {
        LongStrat::VwapReversion => {
            ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3
        }
        LongStrat::DualRsi => {
            ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20
        }
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
    }
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }

    // RUN136: Trade Zone Index Confirmation
    if !tzi_confirm(cs) { return false; }

    match strat {
        ShortStrat::ShortVwapRev => {
            ind.z > 1.5 && ind.p > ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        ShortStrat::ShortBbBounce => {
            ind.p >= ind.bb_hi * 0.98 && ind.vol > ind.vol_ma * 1.3
        }
        ShortStrat::ShortMeanRev => ind.z > 1.5,
        ShortStrat::ShortAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_hi.is_nan() && range > 0.0
                && ind.p >= ind.adr_hi - range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
    }
}
```

---

## Validation Method

### RUN136.1 — Trade Zone Index Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no TZI filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `TZI_WINDOW` | [10, 20, 30] |
| `TZI_THRESH` | [50.0, 60.0, 70.0] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `tzi_filter_rate`: % of entries filtered by TZI threshold
- `tzi_at_filtered`: average TZI of filtered entries
- `tzi_at_allowed`: average TZI of allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN136.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best TZI_WINDOW × TZI_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- TZI-filtered entries (blocked) have lower win rate than TZI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN136.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no TZI) | Trade Zone Index Confirmation | Delta |
|--------|---------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| TZI at Blocked Entries | — | X | — |
| TZI at Allowed Entries | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **TZI is a lagging consolidation measure:** TZI only tells us if price has been consolidating in the past N bars — it doesn't predict whether consolidation will continue or break. A market can transition from consolidating to trending quickly.
2. **Optimal threshold varies by market state:** In strongly trending markets, TZI may never reach 60%. In volatile ranging markets, TZI may always be above 60%. The threshold may need to be adaptive.
3. **TZI window selection is critical:** A window that's too small (10) is noisy; a window that's too large (30) is slow to respond to regime changes.

---

## Why It Could Succeed

1. **Direct measure of consolidation:** TZI directly measures whether price has been oscillating within a range — exactly the condition where mean-reversion works best.
2. **Simple and intuitive:** TZI = "what % of recent bars have been inside the recent high/low range?" A value of 70% means the market has been consolidating. Immediate intuitive meaning.
3. **Complements existing regime detectors:** ADX/Aroon measure trend direction/strength. TZI measures consolidation within a range — different and complementary information.
4. **Identifies range-bound vs trending:** COINCLAW doesn't have a direct measure of this dichotomy. TZI provides it.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN136 Trade Zone Index Confirmation |
|--|--|--|
| Consolidation awareness | None | TZI direct measurement |
| Market regime | ADX/Aroon | TZI |
| Entry filter | z-score + RSI + vol | + TZI > 60% |
| Consolidation definition | None | Price within recent high/low |
| TZI measurement | None | % of bars in consolidation zone |
| Range detection | None | Direct from High/Low bounds |
| Entry timing | Oscillator extreme | Oscillator extreme + consolidating |
