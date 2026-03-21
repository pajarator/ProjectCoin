# RUN125 — Ease of Movement Confirmation: Use EMV to Confirm Price Moves "Easily" Before Entry

## Hypothesis

**Named:** `emv_confirm`

**Mechanism:** COINCLAW uses volume as a filter (volume must be above average), but volume alone doesn't measure the *quality* of price movement. The Ease of Movement (EMV) indicator, developed by Richard Arms, measures how easily price moves on a given volume — it captures the relationship between volume and price change. High EMV means price is moving "easily" on low volume (effort vs result), indicating healthy market conditions. Low or negative EMV means price is struggling to move even with high volume (difficult movement), indicating unhealthy conditions. The EMV Confirmation requires EMV to be above a threshold for LONG (price moving up easily) or below for SHORT (price moving down easily), ensuring entries only fire when the market is moving efficiently.

**Ease of Movement Confirmation:**
- Track EMV on 15m: `EMV = (midpoint_diff / box_ratio) × volume_scaled`, where `midpoint_diff = (high + low)/2 - (prior_high + prior_low)/2`, `box_ratio = volume / (high - low)`
- For regime LONG entry: require `emv > EMV_THRESH` (price moving up easily)
- For regime SHORT entry: require `emv < -EMV_THRESH` (price moving down easily)
- This filters out entries where price is struggling to move — high volume but little price change

**Why this is not a duplicate:**
- RUN80 (volume imbalance) measured buy/sell ratio — EMV measures effort (volume) vs result (price change), completely different
- RUN109 (min volume surge) required volume magnitude — EMV is about the ratio of volume to price movement, not absolute volume
- RUN112 (MFI confirmation) measured money flow direction — EMV measures how easily price moves, different concept
- RUN123 (A/D line divergence) measured money flow accumulation — EMV measures ease of movement, not accumulation
- RUN104 (volume dryup exit) used volume absence — EMV uses volume/price ratio, completely different
- No prior RUN has used Ease of Movement as an entry filter for regime trades

**Mechanistic rationale:** A price move is "healthy" when it occurs on low volume — it means the market is absorbing the move naturally, without requiring heavy volume to push price. EMV captures this: high EMV = easy movement. When EMV is low or negative, price is struggling to move even with high volume — this is "difficult" movement that tends to fail. For regime mean-reversion entries, requiring positive EMV for LONG ensures the market is in a state where price can move up "easily," increasing the probability that the mean-reversion bounce will succeed without needing heavy volume to sustain it.

---

## Proposed Config Changes

```rust
// RUN125: Ease of Movement Confirmation
pub const EMV_ENABLE: bool = true;
pub const EMV_LOOKBACK: usize = 14;         // EMV smoothing period (standard is 14)
pub const EMV_THRESH: f64 = 0.5;          // EMV threshold for LONG (must be above this)
pub const EMV_SHORT_THRESH: f64 = -0.5;    // EMV threshold for SHORT (must be below this)
```

**`indicators.rs` — add EMV computation:**
```rust
// EMV = (Midpoint Move / Volume / Box Ratio)
// Midpoint Move = ((High + Low) / 2 - (Prior High + Prior Low) / 2)
// Box Ratio = (Volume / (High - Low)) / 10000 (scaled)
// EMV = EMV + (Midpoint Move / Box Ratio) smoothed over EMV_LOOKBACK
// Ind15m should have: emv: f64
```

**`strategies.rs` — add EMV helper functions:**
```rust
/// Check if EMV confirms LONG entry (price moving up easily).
fn emv_confirm_long(ind: &Ind15m) -> bool {
    if !config::EMV_ENABLE { return true; }
    if ind.emv.is_nan() { return true; }
    ind.emv > config::EMV_THRESH
}

/// Check if EMV confirms SHORT entry (price moving down easily).
fn emv_confirm_short(ind: &Ind15m) -> bool {
    if !config::EMV_ENABLE { return true; }
    if ind.emv.is_nan() { return true; }
    ind.emv < config::EMV_SHORT_THRESH
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN125: Ease of Movement Confirmation
    if !emv_confirm_long(ind) { return false; }

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

    // RUN125: Ease of Movement Confirmation
    if !emv_confirm_short(ind) { return false; }

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

### RUN125.1 — EMV Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no EMV filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `EMV_LOOKBACK` | [10, 14, 20] |
| `EMV_THRESH` | [0.1, 0.5, 1.0] |
| `EMV_SHORT_THRESH` | [-0.1, -0.5, -1.0] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `emv_filter_rate`: % of entries filtered by EMV confirmation
- `emv_at_filtered`: average EMV at filtered entries
- `emv_at_allowed`: average EMV at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN125.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best EMV_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- EMV-filtered entries (blocked) have lower win rate than EMV-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN125.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no EMV) | EMV Confirmation | Delta |
|--------|---------------------|----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| EMV at Blocked LONG | — | X | — |
| EMV at Blocked SHORT | — | X | — |
| EMV at Allowed LONG | — | X | — |
| EMV at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **EMV is volume-based and noisy:** EMV's box ratio (volume / (high - low)) can be extremely volatile bar-to-bar. Small changes in range can create large swings in EMV. The indicator may be too noisy for 15m bars.
2. **EMV requires range data:** If a bar has the same high and low (doji or very narrow range), the box ratio becomes infinite. EMV needs careful handling of zero-range bars.
3. **Volume-based filters may duplicate volume logic:** COINCLAW already has a volume filter. Adding EMV — which includes volume in its formula — may be partially redundant with the existing volume filter.

---

## Why It Could Succeed

1. **Measures movement quality, not just volume:** EMV captures the efficiency of price movement — how much price change occurs per unit volume. High EMV = efficient, easy movement = healthy market. This is a genuinely different dimension from absolute volume.
2. **Identifies difficult movement:** Low or negative EMV means price is struggling to move even with volume. These are exactly the setups where mean-reversion is likely to fail — the market lacks the "ease" needed for price to revert.
3. **Filters out congested markets:** When volume is high but price barely moves (low EMV), the market is congested. EMV helps filter these out, ensuring entries fire when price CAN move easily.
4. **Institutional-grade tool:** Ease of Movement is a respected indicator that relates volume to price range. It's used by practitioners who care about the quality, not just quantity, of market movement.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN125 EMV Confirmation |
|--|--|--|
| Volume filter | vol > vol_ma × 1.2 | vol × EMV ease-of-movement |
| Movement quality | None | EMV efficiency ratio |
| Entry quality | Volume magnitude | Volume/price efficiency |
| LONG filter | vol spike | vol spike + EMV > 0.5 |
| SHORT filter | vol spike | vol spike + EMV < -0.5 |
| Effort vs result | None | EMV (effort = volume, result = price move) |
| Choppy detection | None | EMV low/negative = difficult movement |
| Filter type | Absolute volume | Ratio (volume/price movement) |
