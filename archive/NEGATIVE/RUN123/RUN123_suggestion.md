# RUN123 — Accumulation/Distribution Line Divergence: Use A/D Line for Institutional Money Flow Confirmation

## Hypothesis

**Named:** `ad_line_divergence`

**Mechanism:** COINCLAW uses oscillators like RSI, z-score, and volume for entry signals, but these measure price deviation or raw volume without capturing the *sustained flow* of money into or out of an asset. The Accumulation/Distribution (A/D) Line, developed by Marc Chaikin, is a cumulative money flow indicator that combines price and volume to measure the net money flow: when price closes in the upper half of the range on increasing volume, money is accumulating; when price closes in the lower half on increasing volume, money is distributing. The A/D Line Divergence uses A/D Line direction as a filter: for regime LONG entries, require the A/D Line to be rising (confirming accumulation), which means institutional money is flowing in — the mean-reversion has institutional backing.

**A/D Line Divergence:**
- Track A/D Line on 15m: `A/D = prior_A/D + money_flow_multiplier × volume`, where `money_flow_multiplier = ((close - low) - (high - close)) / (high - low)`
- Track A/D Line slope (e.g., over A/D_SLOPE_PERIOD bars)
- For regime LONG entry: require `ad_slope > AD_SLOPE_MIN` (A/D Line rising = accumulation)
- For regime SHORT entry: require `ad_slope < -AD_SLOPE_MIN` (A/D Line falling = distribution)
- This adds institutional money flow direction as a confirmation layer

**Why this is not a duplicate:**
- RUN80 (volume imbalance) estimated buy/sell volume from candle body ratios — A/D uses the full money flow multiplier, a more complete formula
- RUN109 (min volume surge) required raw vol spike — A/D Line tracks sustained cumulative flow, not single-bar volume
- RUN112 (MFI confirmation) used MFI — MFI is an oscillator (0-100), A/D is a cumulative line that shows direction of flow
- RUN104 (volume dryup exit) used volume — A/D measures *direction* of money flow, not just volume magnitude
- RUN119 (Vortex) used directional movement — A/D uses money flow multiplier, different math
- RUN88 (trailing z-exit), RUN99 (z-momentum divergence) — completely different mechanism
- No prior RUN has used A/D Line as an entry filter for regime trades

**Mechanistic rationale:** Institutional traders leave footprints in price and volume, but they often accumulate positions gradually over many bars. The A/D Line captures this gradual accumulation/distribution by cumulatively tracking whether price closes in the upper or lower half of the range on each bar. A rising A/D Line means institutional money is steadily flowing into the asset — even if price hasn't moved much yet. For regime LONG entries, requiring a rising A/D Line ensures the trade has institutional backing, not just statistical extreme. The A/D Line is a leading indicator of price movement: institutions accumulate first, price follows.

---

## Proposed Config Changes

```rust
// RUN123: Accumulation/Distribution Line Divergence
pub const AD_LINE_ENABLE: bool = true;
pub const AD_SLOPE_PERIOD: usize = 5;        // number of bars to compute A/D slope
pub const AD_SLOPE_MIN: f64 = 0.0001;       // minimum A/D slope (positive for LONG, negative for SHORT)
```

**`indicators.rs` — add A/D Line computation:**
```rust
// Money Flow Multiplier = ((Close - Low) - (High - Close)) / (High - Low)
// Money Flow Volume = Money Flow Multiplier × Volume
// A/D Line = Prior A/D + Money Flow Volume
// Ind15m should have: ad_line: f64
```

**`strategies.rs` — add A/D Line helper functions:**
```rust
/// Compute A/D Line slope over AD_SLOPE_PERIOD bars.
fn ad_slope(cs: &CoinState) -> f64 {
    let candles = &cs.candles_15m;
    if candles.len() < config::AD_SLOPE_PERIOD + 1 {
        return 0.0;
    }

    // Need to compute A/D line at each bar
    // For simplicity, use stored ad_line values
    let ind = match &cs.ind_15m {
        Some(i) => i,
        None => return 0.0,
    };
    if ind.ad_line.is_nan() { return 0.0; }

    // Compare current A/D to A/D from AD_SLOPE_PERIOD bars ago
    // For precise slope, need historical A/D values
    // Using ad_history rolling store
    let current_ad = ind.ad_line;
    let prior_ad = cs.ad_history.last().copied().unwrap_or(current_ad);
    (current_ad - prior_ad) / config::AD_SLOPE_PERIOD as f64
}

/// Check if A/D Line confirms LONG entry (rising = accumulation).
fn ad_confirm_long(cs: &CoinState) -> bool {
    if !config::AD_LINE_ENABLE { return true; }
    let slope = ad_slope(cs);
    slope > config::AD_SLOPE_MIN
}

/// Check if A/D Line confirms SHORT entry (falling = distribution).
fn ad_confirm_short(cs: &CoinState) -> bool {
    if !config::AD_LINE_ENABLE { return true; }
    let slope = ad_slope(cs);
    slope < -config::AD_SLOPE_MIN
}
```

**`state.rs` — CoinState addition:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub ad_history: VecDeque<f64>,  // rolling A/D Line history for slope calculation
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN123: A/D Line Divergence Confirmation
    if !ad_confirm_long(cs) { return false; }

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

    // RUN123: A/D Line Divergence Confirmation
    if !ad_confirm_short(cs) { return false; }

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

### RUN123.1 — A/D Line Divergence Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no A/D Line filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `AD_SLOPE_PERIOD` | [3, 5, 10] |
| `AD_SLOPE_MIN` | [0.00001, 0.0001, 0.001] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `ad_filter_rate`: % of entries filtered by A/D Line slope requirement
- `ad_slope_at_filtered`: average A/D slope of filtered entries (should be neutral)
- `ad_slope_at_allowed`: average A/D slope of allowed entries (should be rising/falling)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN123.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best AD_SLOPE_PERIOD × AD_SLOPE_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- A/D-filtered entries (blocked) have lower win rate than A/D-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN123.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no A/D) | A/D Line Divergence | Delta |
|--------|---------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| A/D Slope at Blocked LONG | — | X | — |
| A/D Slope at Blocked SHORT | — | X | — |
| A/D Slope at Allowed LONG | — | X | — |
| A/D Slope at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **A/D Line is cumulative and can diverge:** Because A/D is cumulative, it can diverge from price for extended periods. A rising A/D Line while price is falling might indicate hidden selling pressure that hasn't manifested yet — not a confirmation of a LONG entry.
2. **Money flow multiplier is unstable:** The money flow multiplier `((close-low) - (high-close)) / (high-low)` can be volatile in choppy markets with small ranges, making the A/D Line noisy.
3. **Single-timeframe measurement:** A/D Line is computed from a single timeframe (15m). Institutional accumulation often occurs over longer timeframes. The 15m A/D may not capture the full institutional picture.

---

## Why It Could Succeed

1. **Captures institutional money flow:** The A/D Line was specifically designed to track institutional accumulation/distribution. When institutions buy, they push price up AND volume up, and the A/D Line rises. A rising A/D confirms this is happening.
2. **Leading indicator of price movement:** A/D Line often leads price — institutions accumulate first, price follows later. Requiring rising A/D for LONG ensures the trade is aligned with institutional flow, not just statistical extreme.
3. **Cumulative vs single-bar:** Unlike volume or MFI (which are single-bar oscillators), A/D is cumulative — it tracks the *direction* of sustained money flow, not just today's reading. This is more aligned with how institutions actually operate.
4. **Different from volume imbalance:** RUN80 estimated buy/sell imbalance from candle bodies. A/D uses the Chaikin money flow multiplier, a more complete formula that accounts for where price closed within the range.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN123 A/D Line Divergence |
|--|--|--|
| Money flow awareness | None | A/D Line (cumulative) |
| Institutional signal | None | Rising A/D = accumulation |
| Entry filter | z-score + RSI + vol | + A/D Line slope |
| LONG confirmation | z + RSI + vol | + A/D rising |
| SHORT confirmation | z + RSI + vol | + A/D falling |
| Flow measurement | Volume magnitude | Money flow multiplier × volume |
| Time horizon | Single bar | Cumulative across bars |
| Indicator type | Price oscillator | Cumulative money flow line |
