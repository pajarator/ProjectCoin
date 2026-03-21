# RUN127 — Force Index Confirmation: Use Alexander Elder's Force Index for Momentum Confirmation

## Hypothesis

**Named:** `force_index_confirm`

**Mechanism:** COINCLAW uses z-score, RSI, and volume as entry filters, but these don't directly measure the *force* behind price movements — the combination of volume, direction, and magnitude. Dr. Alexander Elder's Force Index combines all three: `Force = (close - prior_close) × volume`. A high positive Force Index means price moved up strongly on high volume (strong buying force); a high negative Force Index means price moved down strongly on high volume (strong selling force). The Force Index Confirmation uses the sign and magnitude of Force Index to confirm that entries are backed by genuine market force: for LONG entries, require the Force Index to be less negative than the prior bar (force is recovering), confirming the selling force is weakening.

**Force Index Confirmation:**
- Track Force Index on 15m: `Force = (close - prior_close) × volume`, smoothed with EMA(13)
- For regime LONG entry: require `force > FORCE_LONG_MIN` (force is positive or recovering from negative)
- For regime SHORT entry: require `force < FORCE_SHORT_MAX` (force is negative or recovering from positive)
- This adds the combined signal of price change direction + magnitude + volume as a single force reading

**Why this is not a duplicate:**
- RUN112 (MFI confirmation) used Money Flow Index — Force Index uses (price change × volume), different formula and interpretation
- RUN99 (z-momentum divergence) used z-score momentum divergence — Force Index uses price × volume, completely different calculation
- RUN122 (Elder Ray) used Bull/Bear Power (High/Low - EMA) — Force Index uses (close - prior close) × volume, different components
- RUN108 (momentum hour filter) used 1m ROC — Force Index combines ROC with volume, giving a more complete picture
- RUN85 (momentum pulse filter) used ROC — Force Index = ROC × volume, different because volume is included
- RUN88 (trailing z-exit), RUN104 (volume dryup exit) — completely different mechanism
- No prior RUN has used Force Index as an entry filter for regime trades

**Mechanistic rationale:** Force Index was specifically designed by Dr. Elder to measure the "force" behind price movements. A strong move up on high volume = strong buying force = likely to continue. A weak move down on low volume = weak selling force = likely to reverse. For regime mean-reversion entries, requiring recovering Force Index (less negative than prior for LONG) ensures the trade has buying force backing it, not just statistical extreme. Force Index captures the essence of what makes a move "real" — it's not just the price extreme, it's the force behind it.

---

## Proposed Config Changes

```rust
// RUN127: Force Index Confirmation
pub const FORCE_INDEX_ENABLE: bool = true;
pub const FORCE_EMA_PERIOD: usize = 13;       // Force Index smoothing EMA period (standard is 13)
pub const FORCE_LONG_MIN: f64 = 0.0;        // Force must be >= this for LONG (positive or recovering)
pub const FORCE_SHORT_MAX: f64 = 0.0;       // Force must be <= this for SHORT (negative or recovering)
```

**`indicators.rs` — add Force Index computation:**
```rust
// Force Index = (Close - Prior Close) × Volume
// Force Index (smoothed) = EMA(Force Index, FORCE_EMA_PERIOD)
// Ind15m should have: force_index: f64
```

**`strategies.rs` — add Force Index helper functions:**
```rust
/// Check if Force Index confirms LONG entry (buying force present or recovering).
fn force_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::FORCE_INDEX_ENABLE { return true; }
    if ind.force_index.is_nan() { return true; }

    // For LONG: force must be positive or recovering (less negative than prior)
    if ind.force_index < config::FORCE_LONG_MIN {
        // Check if recovering: current force > prior force (less negative)
        let candles = &cs.candles_15m;
        if candles.len() < 2 { return ind.force_index >= config::FORCE_LONG_MIN; }
        // Need prior force — compute from prior bar
        let prior_close = candles[candles.len() - 2].c;
        let prior_vol = candles[candles.len() - 2].v;
        let current_close = candles[candles.len() - 1].c;
        let prior_force = (prior_close - candles[candles.len() - 3.min(candles.len() - 1)].c) * prior_vol;
        if ind.force_index < prior_force { return false; }
    }
    true
}

/// Check if Force Index confirms SHORT entry (selling force present or recovering).
fn force_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::FORCE_INDEX_ENABLE { return true; }
    if ind.force_index.is_nan() { return true; }

    // For SHORT: force must be negative or recovering (less positive than prior)
    if ind.force_index > config::FORCE_SHORT_MAX {
        let candles = &cs.candles_15m;
        if candles.len() < 2 { return ind.force_index <= config::FORCE_SHORT_MAX; }
        let prior_close = candles[candles.len() - 2].c;
        let prior_vol = candles[candles.len() - 2].v;
        let prior_force = (prior_close - candles[candles.len() - 3.min(candles.len() - 1)].c) * prior_vol;
        if ind.force_index > prior_force { return false; }
    }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN127: Force Index Confirmation
    if !force_confirm_long(cs, ind) { return false; }

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

    // RUN127: Force Index Confirmation
    if !force_confirm_short(cs, ind) { return false; }

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

### RUN127.1 — Force Index Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Force Index filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `FORCE_EMA_PERIOD` | [7, 13, 21] |
| `FORCE_LONG_MIN` | [-1000000, 0.0, 1000000] |
| `FORCE_SHORT_MAX` | [-1000000, 0.0, 1000000] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `force_filter_rate`: % of entries filtered by Force Index confirmation
- `force_at_filtered`: average Force Index at filtered entries
- `force_at_allowed`: average Force Index at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN127.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best FORCE_EMA_PERIOD per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Force Index-filtered entries (blocked) have lower win rate than Force Index-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN127.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Force) | Force Index Confirmation | Delta |
|--------|----------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Force at Blocked LONG | — | X | — |
| Force at Blocked SHORT | — | X | — |
| Force at Allowed LONG | — | X | — |
| Force at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Force Index is raw and noisy:** Force Index = (price change × volume) is a raw number that varies wildly across coins due to different price scales and volume magnitudes. The EMA smoothing helps but the absolute threshold is hard to set universally.
2. **Volume magnitude varies by coin:** A Force of 1,000,000 means very different things for BTC (where volume is huge) vs a low-price altcoin. A single threshold doesn't work across all 18 coins.
3. **Combining price change AND volume may duplicate other filters:** COINCLAW already has z-score (price) and volume filters. Force Index combines both, making it partially redundant with existing filters.

---

## Why It Could Succeed

1. **Combines three forces into one:** Force Index = direction × magnitude × volume. No other indicator in COINCLAW combines all three. This is the purest measure of market "force."
2. **Recovers from weakness:** The key insight is not just current force, but whether force is recovering — for LONG, force being less negative than prior bar means the selling force is weakening. This captures momentum shift.
3. **Institutional-grade tool:** Dr. Alexander Elder is one of the most respected trading educators. Force Index is a core component of his Triple Screen trading system.
4. **Smoothed vs raw:** Using EMA(13) smoothed Force Index removes bar-to-bar noise while preserving the underlying momentum signal. This is more robust than raw Force Index.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN127 Force Index Confirmation |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + Force Index |
| Force measurement | None | (close - prior_close) × volume |
| Momentum source | z-score, ROC | Force Index (direction × magnitude × volume) |
| LONG confirmation | z < -1.5 | + Force >= 0 or recovering |
| SHORT confirmation | z > 1.5 | + Force <= 0 or recovering |
| Smoothing | None | EMA(13) |
| Recovery detection | None | Force recovering vs prior bar |
| Indicator combination | Separate oscillators | Combined price + volume |
