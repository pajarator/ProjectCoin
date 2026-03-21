# RUN124 — Choppiness Index Confirmation: Use CI to Confirm Market Is Choppy/Ranging Before Mean-Reversion Entries

## Hypothesis

**Named:** `choppiness_confirm`

**Mechanism:** COINCLAW's regime trades are mean-reversion strategies — they work best when the market is choppy or ranging, and struggle when the market is in a strong trend. But COINCLAW doesn't have a direct measure of "choppiness." The Choppiness Index (CI), developed by E.W. Dreiss, is a direct measure of market choppiness — it oscillates between 0 and 100, with high values (typically > 61.8) indicating a choppy, ranging market and low values (< 38.2) indicating a trending market. The Choppiness Index Confirmation requires CI to be above a threshold (e.g., 50) before allowing regime mean-reversion entries, ensuring trades only fire when the market is choppy enough for mean-reversion to work.

**Choppiness Index Confirmation:**
- Track CI on 15m: `CI = 100 × log10(sum(ATR, 14) / (max(14d) - min(14d))) / log10(14)`
- For regime LONG entry: require `ci > CI_CHOPPY_THRESH` (e.g., > 50 = choppy)
- For regime SHORT entry: require `ci > CI_CHOPPY_THRESH` (e.g., > 50 = choppy)
- This directly measures if the market is choppy enough for mean-reversion

**Why this is not a duplicate:**
- RUN82 (regime decay detection) used ADX rise — CI directly measures choppiness, not trend strength
- RUN114 (Aroon) used Aroon direction — CI measures how choppy the market is, not the direction
- RUN89 (market-wide ADX confirmation) used avg ADX — CI is a different calculation and specifically measures choppiness
- RUN25 (BTC 4-regime) used BULL/BEAR × HIGH/LOW vol — CI is a direct choppiness measurement
- No prior RUN has used Choppiness Index as an entry filter for regime trades

**Mechanistic rationale:** Mean-reversion strategies are fundamentally different from trend-following — they expect price to oscillate around a mean. In a strongly trending market, mean-reversion entries are fighting the trend and losing. The CI directly measures whether the market is choppy (high CI = good for mean-reversion) or trending (low CI = bad for mean-reversion). By requiring CI > 50 before entries, COINCLAW only trades when the market environment is favorable for mean-reversion. This is more direct than using ADX or Aroon, which measure trend-related properties rather than choppiness itself.

---

## Proposed Config Changes

```rust
// RUN124: Choppiness Index Confirmation
pub const CHOPPINESS_ENABLE: bool = true;
pub const CI_LOOKBACK: usize = 14;             // Choppiness Index lookback (standard is 14)
pub const CI_CHOPPY_THRESH: f64 = 50.0;       // CI must be above this for mean-reversion entries
```

**`indicators.rs` — add Choppiness Index computation:**
```rust
// CI = 100 × log10(sum(ATR(14), 14) / (max(high, 14) - min(low, 14))) / log10(14)
// Ind15m should have: ci: f64
```

**`strategies.rs` — add CI helper functions:**
```rust
/// Check if Choppiness Index confirms ranging/choppy market for LONG.
fn ci_confirm_long(ind: &Ind15m) -> bool {
    if !config::CHOPPINESS_ENABLE { return true; }
    if ind.ci.is_nan() { return true; }
    ind.ci > config::CI_CHOPPY_THRESH
}

/// Check if Choppiness Index confirms ranging/choppy market for SHORT.
fn ci_confirm_short(ind: &Ind15m) -> bool {
    if !config::CHOPPINESS_ENABLE { return true; }
    if ind.ci.is_nan() { return true; }
    ind.ci > config::CI_CHOPPY_THRESH
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN124: Choppiness Index Confirmation
    if !ci_confirm_long(ind) { return false; }

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

pub fn short_entry(ind: &Ind15m, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }

    // RUN124: Choppiness Index Confirmation
    if !ci_confirm_short(ind) { return false; }

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

### RUN124.1 — Choppiness Index Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no CI filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CI_LOOKBACK` | [10, 14, 20] |
| `CI_CHOPPY_THRESH` | [40.0, 50.0, 60.0] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `ci_filter_rate`: % of entries filtered by CI threshold
- `ci_at_filtered`: average CI of filtered entries (should be trending/low CI)
- `ci_at_allowed`: average CI of allowed entries (should be choppy/high CI)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN124.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best CI_CHOPPY_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- CI-filtered entries (blocked) have lower win rate than CI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN124.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no CI) | Choppiness Index Confirmation | Delta |
|--------|----------------------|---------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| CI at Blocked LONG | — | X | — |
| CI at Blocked SHORT | — | X | — |
| CI at Allowed LONG | — | X | — |
| CI at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **CI can be slow to change:** The Choppiness Index uses a 14-bar lookback and sum of ATR, so it responds slowly to regime changes. By the time CI confirms a choppy market, the choppiness may be ending.
2. **Crypto markets are always somewhat choppy:** BTC and crypto markets in general tend to be volatile and choppy. CI may be above threshold frequently, making the filter ineffective (not filtering anything).
3. **Threshold is arbitrary:** The 61.8/38.2 Fibonacci levels in the CI are theoretical — they may not be optimal for crypto 15m bars. A fixed threshold of 50 may be too low or too high.

---

## Why It Could Succeed

1. **Direct measure of choppiness:** CI was specifically designed to measure whether the market is choppy or trending — exactly what mean-reversion strategies need to know. It's the right tool for the job.
2. **Simple and interpretable:** CI values between 0-100: high = choppy, low = trending. A threshold of 50 is intuitive: require the market to be more choppy than trending before entering.
3. **Complements ADX and Aroon:** ADX measures trend *strength* (high ADX = trending). CI measures trend *choppiness* (high CI = choppy). These are related but different — a market can have high ADX and high CI simultaneously in volatile choppy trends.
4. **Fibonacci-based thresholds:** The traditional CI thresholds (61.8 and 38.2) come from Fibonacci — well-established in technical analysis. These provide principled threshold levels.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN124 Choppiness Index Confirmation |
|--|--|--|
| Choppiness awareness | None | CI direct measurement |
| Market regime | ADX / SMA20 | CI value vs threshold |
| Entry filter | z + RSI + vol | + CI > 50 |
| LONG threshold | None | CI > 50 (choppy required) |
| SHORT threshold | None | CI > 50 (choppy required) |
| Regime measurement | Indirect (ADX) | Direct (CI) |
| Choppiness threshold | None | 50 (intuitive) |
| Filter logic | Oscillator extremes | Market choppiness |
