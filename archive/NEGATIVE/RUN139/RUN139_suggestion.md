# RUN139 — Darvas Box Entry Filter: Use Nicolas Darvas's Box Theory for Support/Resistance and Entry Timing

## Hypothesis

**Named:** `darvas_box_filter`

**Mechanism:** Nicolas Darvas was a legendary trader who used a box-based system to identify stocks trading in defined ranges and about to break out. The Darvas Box consists of: (1) a box top = highest high of the last N bars, (2) a box bottom = lowest low since the box top was established, (3) a new box forms when price breaks above the box top (for LONG) or below the box bottom (for SHORT). The Darvas Box Entry Filter uses this as an entry filter: for regime LONG entries, price must be trading within or just below the Darvas Box (consolidating near resistance), not already broken out to the upside. For SHORT entries, price must be within or just above the box (consolidating near support), not already broken out below.

**Darvas Box Entry Filter:**
- Track Darvas Box: box_top = highest high of last DARVAS_BOX_PERIOD bars; box_bottom = lowest low since box_top
- When price crosses above box_top: new box starts (breakout — suppress mean-reversion)
- When price crosses below box_bottom: box is invalid (breakdown — suppress mean-reversion)
- For regime LONG entry: require price is within the box (not broken out above), and ideally near box_bottom
- For regime SHORT entry: require price is within the box (not broken out below), and ideally near box_top
- This ensures entries fire during consolidation, not after breakout

**Why this is not a duplicate:**
- RUN97 (BB width scalp gate) used BB width — Darvas uses absolute high/low boxes, different mechanism
- RUN84 (session-based partial exit scaling) used UTC session — Darvas boxes are self-adapting based on price action, not time-based
- RUN114 (Aroon), RUN124 (Choppiness Index) — Darvas boxes directly measure consolidation/range, different implementation
- No prior RUN has used Darvas Box as an entry filter for regime trades

**Mechanistic rationale:** Darvas's insight was that stocks trade in "boxes" — ranges of consolidation — before breaking out. The box top is resistance, the box bottom is support. A regime LONG entry that fires AFTER the stock has already broken out of its box is fighting the breakout — it's too late. The best mean-reversion entries fire when price is consolidating WITHIN a Darvas Box, near the bottom of the box, with the box still intact. This means: (1) there is a defined resistance above (the box top), (2) price is near support (the box bottom), and (3) the market hasn't yet decided its direction — perfect for mean-reversion.

---

## Proposed Config Changes

```rust
// RUN139: Darvas Box Entry Filter
pub const DARVAS_ENABLE: bool = true;
pub const DARVAS_BOX_PERIOD: usize = 20;     // lookback for highest high / lowest low
pub const DARVAS_BOX_TOLERANCE: f64 = 0.001; // tolerance for box boundary breach
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub darvas_box_top: f64,     // current box top
    pub darvas_box_bottom: f64,  // current box bottom
    pub darvas_in_box: bool,      // whether price is currently within box
    pub darvas_box_top_bar: usize, // bar index when box top was established
}
```

**`strategies.rs` — add Darvas Box helper:**
```rust
/// Update Darvas Box each bar.
fn update_darvas_box(cs: &mut CoinState, ind: &Ind15m) {
    let candles = &cs.candles_15m;
    let lookback = config::DARVAS_BOX_PERIOD;

    if candles.len() < lookback {
        return;
    }

    // Find highest high in last lookback bars
    let mut box_top = f64::MIN;
    let mut box_top_idx = 0;
    for i in (candles.len() - lookback)..candles.len() {
        if candles[i].h > box_top {
            box_top = candles[i].h;
            box_top_idx = i;
        }
    }

    // Find lowest low since box top was established
    let mut box_bottom = f64::MAX;
    for i in box_top_idx..candles.len() {
        box_bottom = box_bottom.min(candles[i].l);
    }

    // Check if price is within box (with tolerance)
    let tolerance = (box_top - box_bottom) * config::DARVAS_BOX_TOLERANCE;
    let in_box = ind.p >= box_bottom - tolerance && ind.p <= box_top + tolerance;

    cs.darvas_box_top = box_top;
    cs.darvas_box_bottom = box_bottom;
    cs.darvas_in_box = in_box;
    cs.darvas_box_top_bar = box_top_idx;
}

/// Check if Darvas Box allows LONG entry (price within box, near bottom).
fn darvas_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::DARVAS_ENABLE { return true; }
    if !cs.darvas_in_box { return false; }  // must be within box
    // Price should be closer to bottom than top for mean-reversion LONG
    let box_mid = (cs.darvas_box_top + cs.darvas_box_bottom) / 2.0;
    ind.p <= box_mid  // price below or at midpoint
}

/// Check if Darvas Box allows SHORT entry (price within box, near top).
fn darvas_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::DARVAS_ENABLE { return true; }
    if !cs.darvas_in_box { return false; }
    let box_mid = (cs.darvas_box_top + cs.darvas_box_bottom) / 2.0;
    ind.p >= box_mid  // price above or at midpoint
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN139: Darvas Box Entry Filter
    if !darvas_confirm_long(cs, ind) { return false; }

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

    // RUN139: Darvas Box Entry Filter
    if !darvas_confirm_short(cs, ind) { return false; }

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

### RUN139.1 — Darvas Box Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Darvas Box filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `DARVAS_BOX_PERIOD` | [14, 20, 30] |
| `DARVAS_BOX_TOLERANCE` | [0.0005, 0.001, 0.002] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `darvas_filter_rate`: % of entries filtered by Darvas Box
- `darvas_box_at_filtered`: average box size at filtered entries
- `darvas_box_at_allowed`: average box size at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN139.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best DARVAS_BOX_PERIOD per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Darvas-filtered entries (blocked) have lower win rate than Darvas-confirmed entries
- Filter rate 10–35%

### RUN139.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Darvas) | Darvas Box Entry Filter | Delta |
|--------|------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Box Top at Filtered | — | X | — |
| Box Bottom at Filtered | — | X | — |
| Price vs Box Mid at Allowed | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Darvas Box is static and slow:** Once a box top is established, it doesn't update until price breaks above it. This means the box can become outdated quickly in volatile markets.
2. **Box breakout is noisy:** Price often tests the box top multiple times before either breaking out or reverting. Distinguishing between a "test" and a "breakout" is subjective.
3. **Crypto is always in breakout mode:** Crypto markets are characterized by frequent breakouts and breakdowns. The "within box" condition may be rare, filtering out most entries.

---

## Why It Could Succeed

1. **Institutional-grade range concept:** Nicolas Darvas used his box system to identify consolidation before breakout — one of the most reliable patterns in price action. Applying it to crypto regime trading is principled.
2. **Clear support/resistance from price action:** The box top (highest high) becomes resistance; the box bottom (lowest low since box top) becomes support. These are dynamic levels established by actual price action, not arbitrary indicators.
3. **Filters out post-breakout entries:** Many bad entries fire AFTER a breakout has already occurred. Darvas Box filter specifically suppresses these, ensuring entries fire during consolidation, not after.
4. **Identifies range-bound state:** When price is within a Darvas Box, the market is range-bound. This is exactly the environment where mean-reversion works best.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN139 Darvas Box Entry Filter |
|--|--|--|
| Range awareness | None | Darvas Box consolidation |
| Support/resistance | SMA20 | Darvas Box top/bottom |
| Entry filter | z-score + RSI + vol | + price within box |
| LONG entry | Any bar | Within box, near bottom |
| SHORT entry | Any bar | Within box, near top |
| Box concept | None | Highest high / lowest low since |
| Breakout detection | None | Price beyond box top/bottom |
| Consolidation detection | None | Price within Darvas Box |
