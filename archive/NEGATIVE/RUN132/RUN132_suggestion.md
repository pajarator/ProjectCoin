# RUN132 — RSI Divergence Confirmation: Require Hidden or Classical RSI Divergence at Entry

## Hypothesis

**Named:** `rsi_divergence_confirm`

**Mechanism:** COINCLAW uses RSI as a static threshold filter (e.g., RSI < 40 for LONG), but RSI at a single point in time doesn't capture the *direction* of RSI momentum relative to price. RSI Divergence is a classic technical analysis concept: when price makes a new high/low but RSI fails to confirm (RSI makes a lower high/higher low instead), it signals momentum is weakening and a reversal is likely. The RSI Divergence Confirmation requires a detected RSI divergence pattern at entry: for LONG, price made a new local low but RSI formed a higher low (bullish hidden divergence); for SHORT, price made a new local high but RSI formed a lower high (bearish hidden divergence).

**RSI Divergence Confirmation:**
- Track RSI over RSI_DIV_LOOKBACK bars (e.g., 14 bars for RSI, compare to 14 bars prior)
- Detect RSI divergence: `price_new_low = price < price_prior_low` BUT `rsi_higher_low = rsi > rsi_prior_low` (bullish hidden divergence)
- For LONG entry: require RSI divergence confirmed (RSI diverging from price direction)
- For SHORT entry: require RSI divergence confirmed (price making new high, RSI making lower high)
- This adds momentum direction confirmation — RSI extreme alone is insufficient

**Why this is not a duplicate:**
- RUN103 (stochastic extreme exit) used stochastic — RSI divergence is a completely different pattern detection
- RUN99 (z-momentum divergence) used z-score momentum — RSI divergence uses price vs oscillator relationship, different concept
- RUN77 (recovery rate exit) used z-score velocity — RSI divergence is about oscillator non-confirmation, not velocity
- RUN88 (trailing z-exit), RUN104 (volume dryup exit) — completely different mechanism
- RSI is used in COINCLAW but only as a threshold (e.g., RSI < 40), not for divergence detection
- No prior RUN has used RSI divergence as an entry filter

**Mechanistic rationale:** RSI divergence is one of the most established concepts in technical analysis. When price makes a new low but RSI makes a higher low, it means the selling pressure (reflected in price) is not being confirmed by the momentum oscillator — the decline is losing momentum. This is a classic reversal signal. For regime mean-reversion entries, requiring RSI divergence confirmation ensures the entry fires when momentum has demonstrated non-confirmation with price — the mean-reversion has a higher probability because the prior move's momentum has already failed.

---

## Proposed Config Changes

```rust
// RUN132: RSI Divergence Confirmation
pub const RSI_DIV_ENABLE: bool = true;
pub const RSI_DIV_LOOKBACK: usize = 14;      // lookback for detecting divergence (RSI period)
pub const RSI_DIV_TOLERANCE: f64 = 0.02;   // tolerance for price comparison (% difference allowed)
```

**`strategies.rs` — add RSI divergence helper:**
```rust
/// Detect bullish hidden RSI divergence for LONG.
/// Price makes new low, RSI makes higher low (divergence).
fn rsi_bullish_divergence(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::RSI_DIV_ENABLE { return true; }
    let candles = &cs.candles_15m;
    if candles.len() < RSI_DIV_LOOKBACK * 2 + 1 { return true; }

    // Current RSI
    let rsi_current = ind.rsi;
    if rsi_current.is_nan() { return true; }

    // Price: compare current low to low RSI_DIV_LOOKBACK bars ago
    let current_low = candles[candles.len() - 1].l;
    let prior_low = candles[candles.len() - 1 - RSI_DIV_LOOKBACK].l;

    // RSI at prior low
    // We need RSI value at the time of prior low - approximate by computing or using stored
    // For simplicity: use prior bar's RSI as proxy for RSI at prior low
    let prior_rsi = cs.prior_rsi.unwrap_or(rsi_current);

    // Bullish divergence: price lower than prior low, RSI higher than prior RSI
    // Allow tolerance for noise
    let price_lower = current_low < prior_low * (1.0 - config::RSI_DIV_TOLERANCE);
    let rsi_higher = rsi_current > prior_rsi * (1.0 - config::RSI_DIV_TOLERANCE);

    price_lower && rsi_higher
}

/// Detect bearish hidden RSI divergence for SHORT.
/// Price makes new high, RSI makes lower high (divergence).
fn rsi_bearish_divergence(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::RSI_DIV_ENABLE { return true; }
    let candles = &cs.candles_15m;
    if candles.len() < RSI_DIV_LOOKBACK * 2 + 1 { return true; }

    let rsi_current = ind.rsi;
    if rsi_current.is_nan() { return true; }

    let current_high = candles[candles.len() - 1].h;
    let prior_high = candles[candles.len() - 1 - RSI_DIV_LOOKBACK].h;
    let prior_rsi = cs.prior_rsi.unwrap_or(rsi_current);

    // Bearish divergence: price higher than prior high, RSI lower than prior RSI
    let price_higher = current_high > prior_high * (1.0 + config::RSI_DIV_TOLERANCE);
    let rsi_lower = rsi_current < prior_rsi * (1.0 + config::RSI_DIV_TOLERANCE);

    price_higher && rsi_lower
}
```

**`state.rs` — CoinState addition:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub prior_rsi: Option<f64>,  // prior bar's RSI for divergence detection
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN132: RSI Divergence Confirmation
    if !rsi_bullish_divergence(cs, ind) { return false; }

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

    // RUN132: RSI Divergence Confirmation
    if !rsi_bearish_divergence(cs, ind) { return false; }

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

### RUN132.1 — RSI Divergence Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no RSI divergence filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `RSI_DIV_LOOKBACK` | [10, 14, 20] |
| `RSI_DIV_TOLERANCE` | [0.01, 0.02, 0.05] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `divergence_filter_rate`: % of entries filtered by RSI divergence
- `divergence_detected_at_filtered`: % of filtered entries where divergence was present
- `rsi_delta_at_allowed`: RSI difference at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN132.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best RSI_DIV_LOOKBACK × TOLERANCE per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- RSI divergence-confirmed entries have higher win rate than non-confirmed
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN132.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no RSI div) | RSI Divergence Confirmation | Delta |
|--------|--------------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Price Lower Low at Filtered | — | X% | — |
| RSI Higher Low at Filtered | — | X% | — |
| Divergence Confirmed at Allowed | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Divergence is subjective:** There are multiple types of RSI divergence (classical, hidden, extended) and multiple ways to define the highs/lows to compare. The implementation here is simplified and may not capture all divergence patterns correctly.
2. **Divergence can fail:** RSI divergence is a counter-trend signal — it can fire but price can continue in the original direction. Divergence doesn't guarantee reversal; it only suggests probability.
3. **RSI divergence is rare on 15m:** True RSI divergence may be rare on 15m bars — it often manifests on longer timeframes. Requiring it on 15m may filter out most entries.

---

## Why It Could Succeed

1. **Classic reversal signal:** RSI divergence is one of the most established concepts in technical analysis. When price makes a new low but RSI makes a higher low, it means momentum is diverging — the decline is losing force.
2. **Captures momentum non-confirmation:** COINCLAW uses RSI as a threshold but not as a momentum comparison. RSI divergence adds the crucial dimension of whether RSI *confirms* price moves.
3. **Hidden divergence for mean-reversion:** Hidden divergence (where the oscillator makes a higher/low while price makes a lower/higher) is particularly well-suited for mean-reversion — it signals the prior move's momentum has already failed.
4. **Complements existing RSI threshold:** RSI < 40 tells you RSI is oversold. RSI divergence tells you RSI is *rising while price is falling* — a more dynamic confirmation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN132 RSI Divergence Confirmation |
|--|--|--|
| RSI filter | Static threshold (RSI < 40) | Threshold + divergence pattern |
| Momentum direction | None | RSI vs price direction comparison |
| LONG confirmation | RSI < 40 | RSI < 40 + price low + RSI higher low |
| SHORT confirmation | RSI > 60 | RSI > 60 + price high + RSI lower high |
| Oscillator use | RSI as level | RSI as pattern (divergence) |
| Entry type | Static extreme | Dynamic momentum non-confirmation |
| Pattern detection | None | RSI divergence |
| Signal quality | Level-based | Pattern-based |
