# RUN137 — Bollinger Band Width Percentile Filter: Require BB Width to Be at Historical Extreme for Entries

## Hypothesis

**Named:** `bbw_pct_filter`

**Mechanism:** COINCLAW's regime trades are mean-reversion strategies that work best when price has deviated significantly from the mean. But deviation alone doesn't tell us if volatility is expanding or contracting. Bollinger Band Width (BBW) measures the distance between upper and lower Bollinger Bands — when BBW is at historical lows, volatility is compressed and expansion is imminent; when BBW is at historical highs, volatility is elevated and mean-reversion setups may fail. The Bollinger Band Width Percentile Filter tracks rolling BBW history and requires BBW to be below a percentile threshold (e.g., bottom 25%) before entries fire — ensuring entries only happen when volatility is compressed and expansion is likely.

**Bollinger Band Width Percentile Filter:**
- Track `bbw = (bb_upper - bb_lower) / bb_middle` for each bar
- Compute rolling percentile rank of BBW: `bbw_pct_rank = % of historical BBW values lower than current`
- For regime entries: require `bbw_pct_rank <= BBW_PCT_MAX` (e.g., ≤ 25% = BBW is at a historically low percentile)
- When BBW is at a low percentile, volatility is compressed and expansion is imminent — ideal for mean-reversion entries

**Why this is not a duplicate:**
- RUN65 (BB squeeze duration) used BB squeeze as a regime filter — this uses BBW percentile, a different measure (squeeze = specific threshold, percentile = rolling distribution)
- RUN110 (BB compression entry) used BB width < avg × threshold — this uses BBW percentile rank, a more adaptive measure
- RUN76 (vol-adaptive SL) used ATR percentile — BBW percentile measures band width volatility, not ATR volatility
- RUN124 (Choppiness Index) used CI — BBW percentile measures volatility compression, different mechanism
- No prior RUN has used Bollinger Band Width percentile as an entry filter

**Mechanistic rationale:** Bollinger Bands expand when volatility rises and contract when volatility falls. When BBW is at a historically low percentile, it means volatility is compressed — the bands are narrow. Markets cannot stay compressed forever; when volatility expands, price moves explosively in one direction. For mean-reversion trades, entering when BBW is at a low percentile means: (1) volatility is compressed, (2) mean-reversion has room to work, and (3) when volatility eventually expands, it will likely be in the direction of the mean-reversion trade. This is more adaptive than RUN110's fixed threshold approach because percentile rank adapts to each coin's own BBW distribution.

---

## Proposed Config Changes

```rust
// RUN137: Bollinger Band Width Percentile Filter
pub const BBW_PCT_FILTER_ENABLE: bool = true;
pub const BBW_PCT_WINDOW: usize = 100;       // rolling window for BBW history
pub const BBW_PCT_MAX: f64 = 25.0;         // BBW must be at or below this percentile for entries
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub bbw_history: Vec<f64>,  // rolling 100-bar BBW history
}
```

**`strategies.rs` — add BBW percentile helpers:**
```rust
/// Compute BBW: (BB_upper - BB_lower) / BB_middle.
fn compute_bbw(ind: &Ind15m) -> f64 {
    if ind.bb_hi.is_nan() || ind.bb_lo.is_nan() || ind.bb_middle.is_nan() {
        return f64::NAN;
    }
    if ind.bb_middle == 0.0 {
        return f64::NAN;
    }
    (ind.bb_hi - ind.bb_lo) / ind.bb_middle
}

/// Compute BBW percentile rank: % of historical BBW values lower than current.
fn bbw_percentile_rank(cs: &CoinState, ind: &Ind15m) -> f64 {
    let current_bbw = compute_bbw(ind);
    let hist = &cs.bbw_history;
    if hist.len() < 20 || current_bbw.is_nan() {
        return 50.0;
    }

    let lower_count = hist.iter().filter(|&&bbw| bbw < current_bbw).count();
    (lower_count as f64 / hist.len() as f64) * 100.0
}

/// Update BBW history each bar.
fn update_bbw_history(cs: &mut CoinState, ind: &Ind15m) {
    let bbw = compute_bbw(ind);
    if !bbw.is_nan() {
        cs.bbw_history.push(bbw);
        if cs.bbw_history.len() > config::BBW_PCT_WINDOW {
            cs.bbw_history.remove(0);
        }
    }
}

/// Check if BBW percentile confirms LOW volatility (compressed — good for mean-reversion).
fn bbw_confirm(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::BBW_PCT_FILTER_ENABLE { return true; }
    let pct_rank = bbw_percentile_rank(cs, ind);
    pct_rank <= config::BBW_PCT_MAX
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN137: Bollinger Band Width Percentile Filter
    if !bbw_confirm(cs, ind) { return false; }

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

    // RUN137: Bollinger Band Width Percentile Filter
    if !bbw_confirm(cs, ind) { return false; }

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

### RUN137.1 — BBW Percentile Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BBW percentile filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BBW_PCT_WINDOW` | [50, 100, 200] |
| `BBW_PCT_MAX` | [15, 25, 35] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `bbw_filter_rate`: % of entries filtered by BBW percentile threshold
- `bbw_pct_rank_at_filtered`: average BBW percentile at filtered entries (should be high = not compressed)
- `bbw_pct_rank_at_allowed`: average BBW percentile at allowed entries (should be low = compressed)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN137.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best BBW_PCT_MAX per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- BBW-filtered entries (blocked) have lower win rate than BBW-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN137.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no BBW pct) | BBW Percentile Filter | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| BBW Pct Rank at Blocked | — | X | — |
| BBW Pct Rank at Allowed | — | X | — |
| BBW at Blocked | — | X | — |
| BBW at Allowed | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **BBW percentile doesn't predict expansion timing:** Knowing BBW is at a low percentile doesn't tell us when volatility will expand — it could stay compressed for many bars before expanding.
2. **Historical distribution changes:** BBW's distribution shifts over time as market characteristics change. A percentile of 25% in 2024 may not mean the same thing as in 2022.
3. **May filter too many entries:** If BBW is rarely at the 25th percentile (most of the time it's higher), the filter will suppress most entries, reducing the opportunity set significantly.

---

## Why It Could Succeed

1. **Volatility compression is a known predictor:** Bollinger Band contraction precedes expansion — this is one of the most reliable patterns in technical analysis. Requiring BBW to be at a low percentile ensures entries fire in the compressed state.
2. **Percentile adapts to each coin:** Fixed BBW thresholds don't work across coins with different volatilities. Percentile rank adapts to each coin's own BBW distribution.
3. **Different from RUN110's fixed approach:** RUN110 required BBW < 0.70 × average. That is a fixed-ratio approach. Percentile rank is adaptive — it finds the historically compressed state regardless of absolute BBW level.
4. **Triggers on compression, not just low BBW:** A low BBW in a high-volatility regime is different from a low BBW in a low-volatility regime. Percentile rank captures the *relative* compression.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN137 BBW Percentile Filter |
|--|--|--|
| BBW awareness | None | Rolling BBW percentile |
| Volatility filter | None | BBW percentile ≤ 25% |
| Compression detection | None | BBW at historically low percentile |
| Volatility adaptation | None | Percentile adapts to BBW history |
| Entry timing | Z-score extreme | BBW compressed + z-score extreme |
| Measurement type | Fixed ratio (RUN110) | Adaptive percentile |
| Historical context | None | Rolling window of BBW history |
