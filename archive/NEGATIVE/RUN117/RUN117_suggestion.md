# RUN117 — Rate of Change Percentile Filter: Require ROC to Be at Historical Extreme

## Hypothesis

**Named:** `roc_percentile_filter`

**Mechanism:** COINCLAW's regime entries fire on z-score extremes and RSI extremes, but ROC (Rate of Change) is not normalized by its own historical distribution. A ROC of +0.5% means different things in different volatility regimes — in calm markets it's extreme, in volatile markets it's normal. The ROC Percentile Filter tracks the rolling distribution of ROC values and requires the current ROC to be at an extreme percentile (e.g., top 10% most extreme) before allowing entries — normalizing ROC across volatility regimes the same way RUN107 normalized z-score.

**ROC Percentile Filter:**
- Track rolling ROC history for each coin (e.g., last 100 15m ROC readings)
- Compute percentile rank: `roc_pct = % of historical ROCs more extreme than current`
- For LONG entry: require `roc_pct <= ROC_LONG_PCT` (e.g., ROC in most extreme 10% on the downside)
- For SHORT entry: require `roc_pct >= (100 - ROC_SHORT_PCT)` (e.g., most extreme 10% on upside)
- This ensures entries fire when ROC is genuinely extreme relative to its own recent history

**Why this is not a duplicate:**
- RUN107 (percentile z-filter) normalized z-score — this normalizes ROC, a different momentum indicator
- RUN54 (vol entry threshold) used ATR percentile rank — ROC percentile is price-momentum based, not volatility-based
- RUN76 (vol-adaptive SL) used ATR percentile — ROC percentile is a different concept
- RUN85 (momentum pulse filter) used fixed ROC threshold (roc_3 >= +0.05%) — this uses adaptive percentile thresholds
- RUN60 (z_momentum threshold) used z-score direction at entry — ROC percentile is different momentum normalization
- No prior RUN has normalized ROC by its own historical percentile distribution

**Mechanistic rationale:** ROC is a pure momentum measure — it captures the rate of price change without reference to a mean (unlike z-score). But ROC's magnitude is regime-dependent: in a high-volatility market, a 1% ROC is trivial; in a low-volatility market, a 1% ROC is a big move. By normalizing ROC to its own percentile rank, we make the "extremeness" of a ROC reading comparable across different volatility regimes. A ROC at the 5th percentile means price has been falling faster than 95% of recent readings — a strong momentum signal that complements z-score.

---

## Proposed Config Changes

```rust
// RUN117: Rate of Change Percentile Filter
pub const ROC_PCT_FILTER_ENABLE: bool = true;
pub const ROC_PCT_WINDOW: usize = 100;     // rolling window for ROC history
pub const ROC_LONG_PCT: f64 = 10.0;        // for LONG: ROC must be at or beyond this percentile (most extreme 10% downside)
pub const ROC_SHORT_PCT: f64 = 10.0;        // for SHORT: ROC must be at or beyond this percentile (most extreme 10% upside)
pub const ROC_LOOKBACK: usize = 3;         // ROC lookback period (3 = 3-bar ROC)
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub roc_history: Vec<f64>,  // rolling 100-bar ROC history
}
```

**`strategies.rs` — add ROC percentile helpers:**
```rust
/// Compute the percentile rank of current ROC in the rolling window.
fn roc_percentile_rank(cs: &CoinState) -> f64 {
    let current_roc = match &cs.ind_15m {
        Some(ind) => ind.roc,
        None => return 50.0,
    };

    let hist = &cs.roc_history;
    if hist.len() < 20 {
        return 50.0;  // not enough history
    }

    // Count how many historical ROCs are more extreme than current
    let more_extreme = hist.iter().filter(|&&roc| {
        if current_roc < 0.0 {
            roc < current_roc  // for negative ROC, more extreme = more negative
        } else {
            roc > current_roc  // for positive ROC, more extreme = more positive
        }
    }).count();

    (more_extreme as f64 / hist.len() as f64) * 100.0
}

/// Update ROC history each bar.
fn update_roc_history(cs: &mut CoinState) {
    if let Some(ref ind) = cs.ind_15m {
        if !ind.roc.is_nan() {
            cs.roc_history.push(ind.roc);
            if cs.roc_history.len() > config::ROC_PCT_WINDOW {
                cs.roc_history.remove(0);
            }
        }
    }
}

/// Check if ROC percentile confirms LONG entry (extreme downside).
fn roc_confirm_long(cs: &CoinState) -> bool {
    if !config::ROC_PCT_FILTER_ENABLE { return true; }
    let pct = roc_percentile_rank(cs);
    pct <= config::ROC_LONG_PCT
}

/// Check if ROC percentile confirms SHORT entry (extreme upside).
fn roc_confirm_short(cs: &CoinState) -> bool {
    if !config::ROC_PCT_FILTER_ENABLE { return true; }
    let pct = roc_percentile_rank(cs);
    pct >= (100.0 - config::ROC_SHORT_PCT)
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN117: ROC Percentile Filter
    if !roc_confirm_long(cs) { return false; }

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

    // RUN117: ROC Percentile Filter
    if !roc_confirm_short(cs) { return false; }

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

### RUN117.1 — ROC Percentile Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed ROC threshold (none)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `ROC_PCT_WINDOW` | [50, 100, 200] |
| `ROC_LONG_PCT` | [5.0, 10.0, 15.0] |
| `ROC_SHORT_PCT` | [5.0, 10.0, 15.0] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `roc_pct_filter_rate`: % of entries filtered by ROC percentile requirement
- `roc_pct_at_filtered`: average ROC percentile of filtered entries
- `roc_pct_at_allowed`: average ROC percentile of allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN117.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best ROC_LONG_PCT × ROC_SHORT_PCT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- ROC-percentile-filtered entries (blocked) have lower win rate than allowed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN117.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no ROC pct) | ROC Percentile Filter | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| ROC Pct at Blocked LONG | — | X% | — |
| ROC Pct at Blocked SHORT | — | X% | — |
| ROC Pct at Allowed LONG | — | X% | — |
| ROC Pct at Allowed SHORT | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **ROC is trend-following:** ROC naturally increases in trending markets and decreases in declining markets. Requiring extreme ROC percentile may filter out mean-reversion entries in markets that are legitimately trending down (for LONG) — the "extreme" ROC is just the trend, not a reversal signal.
2. **History is not predictive:** The fact that ROC was in the top 10% historically doesn't mean it will mean-revert. The percentile is descriptive, not predictive.
3. **ROC percentile can stay extreme:** In a sustained downtrend, ROC stays at extreme percentiles for long periods. Requiring extreme percentile may miss the best entry points (after the trend has been down for a while).

---

## Why It Could Succeed

1. **Normalizes across volatility regimes:** A 1% ROC in calm markets vs 3% ROC in volatile markets are both "extreme" in different ways. ROC percentile makes them comparable.
2. **Adaptive threshold:** Instead of a fixed ROC threshold (which is arbitrary), ROC percentile adapts to each coin's recent ROC distribution — data-driven and coin-specific.
3. **Complements z-score percentile:** RUN107 normalized z-score. Normalizing ROC adds a different dimension (momentum rate) that z-score doesn't capture.
4. **Cross-coin comparability:** ROC = 1% on BTC is different from ROC = 1% on SHIB due to different volatilities. Percentile makes the extremeness comparable across coins.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN117 ROC Percentile Filter |
|--|--|--|
| ROC entry threshold | None | Adaptive percentile (top 10% most extreme) |
| ROC awareness | None | Rolling percentile normalization |
| Regime adaptation | None (fixed) | Adaptive to recent ROC distribution |
| LONG filter | z + RSI + vol | z + RSI + vol + ROC pct |
| SHORT filter | z + RSI + vol | z + RSI + vol + ROC pct |
| Volatility awareness | None | ROC history percentile |
| Cross-coin comparability | Poor | Good (percentile normalizes) |
| Entry filter | Z-crossing | Z-crossing + ROC percentile |
