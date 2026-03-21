# RUN113 — CCI Regime Confirmation: Require Commodity Channel Index Extreme at Entry

## Hypothesis

**Named:** `cci_regime_confirm`

**Mechanism:** COINCLAW uses z-score, RSI, and Bollinger Bands to identify mean-reversion extremes. The Commodity Channel Index (CCI) is another established oscillator that measures how far price has deviated from its mean relative to the mean absolute deviation — similar to z-score but with different sensitivity characteristics. When CCI is at an extreme (e.g., < -100 for LONG, > +100 for SHORT), it confirms that price has deviated significantly from its typical range. The CCI Regime Confirmation requires CCI to be at an extreme before allowing regime entries, adding a different oscillator perspective that filters out weaker signals.

**CCI Regime Confirmation:**
- Track CCI on 15m: `CCI = (Typical Price - SMA20) / (0.015 × Mean Absolute Deviation)`
- For regime LONG entry: require `cci < CCI_LONG_MAX` (e.g., CCI < -100) — confirms price is deeply oversold
- For regime SHORT entry: require `cci > CCI_SHORT_MIN` (e.g., CCI > +100) — confirms price is deeply overbought
- This is an additional oscillator confirmation on top of z-score and RSI

**Why this is not a duplicate:**
- RUN107 (percentile z-filter) used z-score percentile — CCI is a different oscillator with different math (MAD vs standard deviation)
- RUN52 (z-confidence sizing) used z-score magnitude — CCI uses a similar concept but different normalization
- RUN54 (vol entry threshold) used ATR percentile — CCI is price-deviation based, not volatility based
- RUN86 (correlation cluster suppress) — completely different mechanism
- No prior RUN has used CCI as an entry confirmation for regime trades

**Mechanistic rationale:** CCI was designed by Donald Lambert specifically to identify cyclical extremes in commodities. It is similar in spirit to z-score (measuring deviation from mean) but uses mean absolute deviation instead of standard deviation, making it more responsive to sudden price moves. A CCI reading below -100 means price is more than 1% of the mean absolute deviation below the average — a strong oversold signal. Requiring CCI extreme as a second confirmation after z-score ensures that entries fire only when multiple independent oscillators agree that the price is at an extreme.

---

## Proposed Config Changes

```rust
// RUN113: CCI Regime Confirmation
pub const CCI_CONFIRM_ENABLE: bool = true;
pub const CCI_LONG_MAX: f64 = -100.0;   // CCI must be below this for LONG entry (deeply oversold)
pub const CCI_SHORT_MIN: f64 = 100.0;   // CCI must be above this for SHORT entry (deeply overbought)
pub const CCI_LOOKBACK: usize = 20;      // CCI lookback period (standard is 20)
```

**`indicators.rs` — add CCI computation to Ind15m:**
```rust
// Ind15m already has cci: f64 field — just needs to be computed
// CCI = (typical_price - sma20) / (0.015 * mean_absolute_deviation)
// where typical_price = (high + low + close) / 3
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
/// Check if CCI confirms LONG entry (deeply oversold).
fn cci_confirm_long(ind: &Ind15m) -> bool {
    if !config::CCI_CONFIRM_ENABLE { return true; }
    if ind.cci.is_nan() { return true; }  // no CCI data available
    ind.cci < config::CCI_LONG_MAX
}

/// Check if CCI confirms SHORT entry (deeply overbought).
fn cci_confirm_short(ind: &Ind15m) -> bool {
    if !config::CCI_CONFIRM_ENABLE { return true; }
    if ind.cci.is_nan() { return true; }
    ind.cci > config::CCI_SHORT_MIN
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN113: CCI Regime Confirmation
    if !cci_confirm_long(ind) { return false; }

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

    // RUN113: CCI Regime Confirmation
    if !cci_confirm_short(ind) { return false; }

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

### RUN113.1 — CCI Regime Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no CCI filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CCI_LONG_MAX` | [-150, -100, -50] |
| `CCI_SHORT_MIN` | [50, 100, 150] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `cci_filter_rate`: % of entries filtered by CCI confirmation
- `cci_at_filtered`: average CCI of filtered entries (should be non-extreme)
- `cci_at_allowed`: average CCI of allowed entries (should be extreme)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN113.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best CCI_LONG_MAX × CCI_SHORT_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- CCI-filtered entries (blocked) have lower win rate than CCI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN113.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no CCI) | CCI Regime Confirmation | Delta |
|--------|----------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| CCI at Blocked LONG | — | X | — |
| CCI at Blocked SHORT | — | X | — |
| CCI at Allowed LONG | — | X | — |
| CCI at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **CCI and z-score are redundant:** Both measure deviation from the mean. Adding CCI confirmation may not add incremental information beyond what z-score already captures, making it a redundant filter.
2. **Different thresholds needed per coin:** CCI is coin-specific due to different price ranges and volatilities. A single threshold may be too strict for some coins and too loose for others.
3. **Adding another oscillator:** COINCLAW already uses RSI, z-score, and Bollinger Bands. Adding CCI is yet another oscillator that may over-complicate the entry logic without proportional benefit.

---

## Why It Could Succeed

1. **Independent oscillator confirmation:** CCI uses mean absolute deviation (not standard deviation like z-score). This gives it different sensitivity — it can catch extremes that z-score misses, particularly in markets with fat tails or sudden spikes.
2. **Different math, different signal:** Standard CCI (< -100 or > +100) is an established extreme threshold used by practitioners. It adds a well-regarded indicator to the confirmation stack.
3. **Institutional-grade tool:** CCI was designed by Donald Lambert specifically for identifying cyclical extremes in commodities. Its use in crypto (also cyclical) is natural.
4. **Second opinion on extremes:** When both CCI and z-score agree that price is at an extreme, the entry has higher conviction than either alone.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN113 CCI Regime Confirmation |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + CCI |
| Oscillator confirmation | RSI (price-based) | RSI + CCI (MAD-based) |
| Oversold confirmation | RSI < 40, z < -1.5 | RSI < 40 + z < -1.5 + CCI < -100 |
| Overbought confirmation | RSI > 60, z > 1.5 | RSI > 60 + z > 1.5 + CCI > +100 |
| Filter strength | Double oscillator | Triple oscillator |
| Extreme detection | z-score + RSI | z-score + RSI + CCI |
| Redundancy | None | Partial (CCI vs z-score) |
