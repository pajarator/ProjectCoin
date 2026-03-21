# RUN129 — VWAP Deviation Percentile Filter: Require Price to Be at Historical Extreme Distance from VWAP

## Hypothesis

**Named:** `vwap_dev_pct_filter`

**Mechanism:** COINCLAW uses z-score as a measure of price deviation from the mean, and uses VWAP as a key reference point. But z-score measures deviation from the SMA-based mean, not from VWAP specifically. The VWAP Deviation Percentile Filter tracks the rolling distribution of how far price has deviated from VWAP (as a percentage) and requires the current VWAP deviation to be at an extreme percentile before allowing entries — normalizing VWAP deviation across different volatility regimes and market conditions.

**VWAP Deviation Percentile Filter:**
- Track `vwap_dev_pct = (price - vwap) / vwap` for each bar
- Compute rolling percentile rank of current VWAP deviation: `vwap_dev_pct_rank = % of historical readings more extreme than current`
- For regime LONG entry: require `vwap_dev_pct_rank <= VWAP_DEV_LONG_PCT` (e.g., price is at most extreme 10% below VWAP)
- For regime SHORT entry: require `vwap_dev_pct_rank >= (100 - VWAP_DEV_SHORT_PCT)` (most extreme 10% above VWAP)
- This normalizes VWAP deviation across volatility regimes, like RUN107 did for z-score

**Why this is not a duplicate:**
- RUN107 (percentile z-filter) normalized z-score (deviation from SMA mean) — this normalizes VWAP deviation (deviation from volume-weighted mean)
- RUN52 (z-confidence sizing) used z-score magnitude — VWAP deviation is a different reference point
- RUN78 (cross-coin z-confirm) used BTC z-score — this uses single-coin VWAP deviation percentile
- VWAP is a distinct reference: it weights recent prices by volume, making it more responsive to volume-driven price levels than SMA
- No prior RUN has tracked rolling VWAP deviation percentile as an entry filter

**Mechanistic rationale:** VWAP is the volume-weighted average price — it represents the "fair" price based on where most volume was traded. When price is far below VWAP, it means the market has been trading below the volume-weighted fair value — an extreme that may mean-revert. The percentile rank makes this comparable across time: a VWAP deviation of -0.5% in a quiet market is more extreme than -0.5% in a volatile market. By requiring the percentile rank to be in the top 10% most extreme, we ensure entries only fire when the VWAP deviation is genuinely unusual relative to recent history.

---

## Proposed Config Changes

```rust
// RUN129: VWAP Deviation Percentile Filter
pub const VWAP_DEV_PCT_ENABLE: bool = true;
pub const VWAP_DEV_WINDOW: usize = 100;      // rolling window for VWAP deviation history
pub const VWAP_DEV_LONG_PCT: f64 = 10.0;    // for LONG: VWAP deviation must be at or beyond this percentile
pub const VWAP_DEV_SHORT_PCT: f64 = 10.0;    // for SHORT: VWAP deviation must be at or beyond this percentile
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub vwap_dev_history: Vec<f64>,  // rolling 100-bar VWAP deviation history
}
```

**`strategies.rs` — add VWAP deviation percentile helpers:**
```rust
/// Compute VWAP deviation as percentage: (price - vwap) / vwap.
fn vwap_deviation(ind: &Ind15m) -> f64 {
    if ind.vwap.is_nan() || ind.vwap == 0.0 {
        return 0.0;
    }
    (ind.p - ind.vwap) / ind.vwap
}

/// Compute percentile rank of current VWAP deviation.
fn vwap_dev_percentile_rank(cs: &CoinState, ind: &Ind15m) -> f64 {
    let current_dev = vwap_deviation(ind);
    let hist = &cs.vwap_dev_history;
    if hist.len() < 20 {
        return 50.0;  // not enough history
    }

    // Count how many historical deviations are more extreme than current
    let more_extreme = hist.iter().filter(|&&dev| {
        if current_dev < 0.0 {
            dev < current_dev  // for negative dev, more extreme = more negative
        } else {
            dev > current_dev  // for positive dev, more extreme = more positive
        }
    }).count();

    (more_extreme as f64 / hist.len() as f64) * 100.0
}

/// Update VWAP deviation history each bar.
fn update_vwap_dev_history(cs: &mut CoinState, ind: &Ind15m) {
    let dev = vwap_deviation(ind);
    cs.vwap_dev_history.push(dev);
    if cs.vwap_dev_history.len() > config::VWAP_DEV_WINDOW {
        cs.vwap_dev_history.remove(0);
    }
}

/// Check if VWAP deviation percentile confirms LONG entry (extreme negative).
fn vwap_dev_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::VWAP_DEV_PCT_ENABLE { return true; }
    let pct = vwap_dev_percentile_rank(cs, ind);
    pct <= config::VWAP_DEV_LONG_PCT
}

/// Check if VWAP deviation percentile confirms SHORT entry (extreme positive).
fn vwap_dev_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::VWAP_DEV_PCT_ENABLE { return true; }
    let pct = vwap_dev_percentile_rank(cs, ind);
    pct >= (100.0 - config::VWAP_DEV_SHORT_PCT)
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN129: VWAP Deviation Percentile Filter
    if !vwap_dev_confirm_long(cs, ind) { return false; }

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

    // RUN129: VWAP Deviation Percentile Filter
    if !vwap_dev_confirm_short(cs, ind) { return false; }

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

### RUN129.1 — VWAP Deviation Percentile Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no VWAP deviation percentile filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VWAP_DEV_WINDOW` | [50, 100, 200] |
| `VWAP_DEV_LONG_PCT` | [5.0, 10.0, 15.0] |
| `VWAP_DEV_SHORT_PCT` | [5.0, 10.0, 15.0] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `vwap_dev_filter_rate`: % of entries filtered by VWAP deviation percentile
- `vwap_dev_pct_rank_at_filtered`: average percentile rank of filtered entries
- `vwap_dev_pct_rank_at_allowed`: average percentile rank of allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN129.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best VWAP_DEV_LONG_PCT × VWAP_DEV_SHORT_PCT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- VWAP deviation-filtered entries (blocked) have lower win rate than allowed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN129.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no VWAP pct) | VWAP Deviation Percentile | Delta |
|--------|--------------------------|-----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| VWAP Dev % Rank at Blocked LONG | — | X% | — |
| VWAP Dev % Rank at Blocked SHORT | — | X% | — |
| VWAP Dev % Rank at Allowed LONG | — | X% | — |
| VWAP Dev % Rank at Allowed SHORT | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **VWAP is already used as entry reference:** Many strategies already compare price to VWAP (VWAP Reversion, ShortVwapRev). Adding percentile rank may be redundant for those strategies.
2. **Percentile is descriptive, not predictive:** The fact that VWAP deviation was in the top 10% most extreme historically doesn't mean it will mean-revert. Percentile is descriptive of past conditions, not predictive of future returns.
3. **VWAP deviation can stay extreme:** In a sustained downtrend, price can remain far below VWAP for extended periods. Requiring top 10% percentile before entering LONG means waiting for the most extreme reading — which might already be too late.

---

## Why It Could Succeed

1. **Normalizes across volatility regimes:** A VWAP deviation of -0.5% means different things in different markets. Percentile rank makes the extremeness comparable across time.
2. **VWAP vs SMA mean reversion:** VWAP is volume-weighted, giving more weight to high-volume bars. This is different from SMA (which weights all bars equally). VWAP deviation captures when price has deviated from the volume-weighted fair price — a different signal than z-score deviation from the arithmetic mean.
3. **Complements z-score percentile:** RUN107 normalized z-score (SMA-based mean). VWAP deviation percentile adds a VWAP-based dimension — two different reference points, both normalized.
4. **Simple and interpretable:** One percentile threshold per direction. Clear meaning: "enter when price is at the most extreme X% distance from VWAP."

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN129 VWAP Deviation Percentile |
|--|--|--|
| VWAP deviation filter | None | Rolling percentile of VWAP deviation |
| Reference point | SMA (z-score) | VWAP (volume-weighted) |
| Deviation normalization | None | Percentile rank (0-100) |
| LONG threshold | z < -1.5 | VWAP dev pct rank <= 10% |
| SHORT threshold | z > 1.5 | VWAP dev pct rank >= 90% |
| Regime adaptation | None (fixed) | Adaptive to recent VWAP deviation history |
| Mean reference | SMA20 | VWAP |
| Percentile normalization | None | Top 10% = extreme |
