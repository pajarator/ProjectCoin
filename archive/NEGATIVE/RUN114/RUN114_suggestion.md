# RUN114 — Aroon Regime Confirmation: Use Aroon Indicator to Confirm Trending vs Range-Bound States

## Hypothesis

**Named:** `aroon_regime_confirm`

**Mechanism:** COINCLAW uses ADX and z-score breadth to detect market regime (ranging vs trending). The Aroon indicator is another established regime detector that measures the time since the last high or low within a lookback period. Aroon Up > Aroon Down indicates a bullish trend; Aroon Down > Aroon Up indicates a bearish trend; both near each other indicates a range-bound market. Unlike ADX (which measures trend strength), Aroon identifies *whether* a trend exists and its direction. The Aroon Regime Confirmation uses Aroon to filter regime entries: require Aroon to confirm a non-trending state (Aroon Up ≈ Aroon Down) before entering mean-reversion trades, and require trending Aroon alignment for momentum trades.

**Aroon Regime Confirmation:**
- Track Aroon Up and Aroon Down on 15m over AROON_LOOKBACK (e.g., 25 bars)
- `aroon_up = (lookback - bars_since_high) / lookback × 100`
- `aroon_down = (lookback - bars_since_low) / lookback × 100`
- For regime LONG entry: require `aroon_down > aroon_up + AROON_Osc_MIN` AND `aroon_up < AROON_TREND_MAX` (confirming not in strong uptrend)
- For regime SHORT entry: require `aroon_up > aroon_down + AROON_Osc_MIN` AND `aroon_down < AROON_TREND_MAX` (confirming not in strong downtrend)
- This ensures mean-reversion entries only fire when Aroon confirms a range-bound state

**Why this is not a duplicate:**
- RUN89 (market-wide ADX confirmation) used ADX — Aroon measures trend existence differently (time-since-high/low vs directional movement strength)
- RUN25 (BTC 4-regime framework) used BULL/BEAR × HIGH/LOW vol — Aroon is a cleaner range/trend detector, not BTC-relative
- RUN82 (regime decay detection) used ADX rise — this uses Aroon crossover, a different mechanism
- RUN12 (scalp market mode filter) used breadth — Aroon is a single-coin indicator, not breadth-based
- No prior RUN has used Aroon as an entry filter for regime trades

**Mechanistic rationale:** Mean-reversion strategies work best in range-bound markets. When Aroon shows a strong uptrend (Aroon Up >> Aroon Down), price is consistently making new highs — mean-reversion entries against this trend face strong headwind. Requiring Aroon to confirm a non-trending state (both Aroon readings near 50) ensures mean-reversion entries fire only in environments where price oscillates around a mean rather than trending. This is more direct for regime detection than ADX, which rises in both trending and volatile ranging markets.

---

## Proposed Config Changes

```rust
// RUN114: Aroon Regime Confirmation
pub const AROON_CONFIRM_ENABLE: bool = true;
pub const AROON_LOOKBACK: usize = 25;       // Aroon lookback period (standard is 25)
pub const AROON_OSC_MIN: f64 = 20.0;        // minimum Aroon oscillator spread (Aroon Down - Aroon Up for LONG)
pub const AROON_TREND_MAX: f64 = 70.0;      // Aroon must be below this to confirm no strong trend
```

**`indicators.rs` — add Aroon computation to Ind15m:**
```rust
// Ind15m already has aroon_up: f64 and aroon_down: f64 fields
// Aroon Up = (lookback - bars_since_high) / lookback * 100
// Aroon Down = (lookback - bars_since_low) / lookback * 100
// Aroon Oscillator = Aroon Up - Aroon Down
```

**`strategies.rs` — add Aroon helper functions:**
```rust
/// Check if Aroon confirms range-bound/non-trending for LONG entry.
fn aroon_confirm_long(ind: &Ind15m) -> bool {
    if !config::AROON_CONFIRM_ENABLE { return true; }
    if ind.aroon_up.is_nan() || ind.aroon_down.is_nan() { return true; }

    // Aroon Down should be above Aroon Up (bearish or neutral)
    // but neither should be in strong trend territory
    let osc = ind.aroon_down - ind.aroon_up;
    osc >= config::AROON_OSC_MIN
        && ind.aroon_up < config::AROON_TREND_MAX
        && ind.aroon_down < config::AROON_TREND_MAX
}

/// Check if Aroon confirms range-bound/non-trending for SHORT entry.
fn aroon_confirm_short(ind: &Ind15m) -> bool {
    if !config::AROON_CONFIRM_ENABLE { return true; }
    if ind.aroon_up.is_nan() || ind.aroon_down.is_nan() { return true; }

    let osc = ind.aroon_up - ind.aroon_down;
    osc >= config::AROON_OSC_MIN
        && ind.aroon_down < config::AROON_TREND_MAX
        && ind.aroon_up < config::AROON_TREND_MAX
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN114: Aroon Regime Confirmation
    if !aroon_confirm_long(ind) { return false; }

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

    // RUN114: Aroon Regime Confirmation
    if !aroon_confirm_short(ind) { return false; }

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

### RUN114.1 — Aroon Regime Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Aroon filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `AROON_LOOKBACK` | [14, 25, 50] |
| `AROON_OSC_MIN` | [10, 20, 30] |
| `AROON_TREND_MAX` | [60, 70, 80] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `aroon_filter_rate`: % of entries filtered by Aroon confirmation
- `aroon_trend_at_filtered`: average Aroon Up/Down at filtered entries (should show trending)
- `aroon_osc_at_allowed`: average Aroon oscillator at allowed entries (should show range-bound)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN114.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best AROON_LOOKBACK × OSC_MIN × TREND_MAX per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Aroon-filtered entries (blocked) have lower win rate than Aroon-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN114.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Aroon) | Aroon Regime Confirmation | Delta |
|--------|----------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Aroon Up at Blocked LONG | — | X | — |
| Aroon Down at Blocked LONG | — | X | — |
| Aroon Osc at Allowed LONG | — | X | — |
| Aroon Osc at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Aroon is slow to respond:** Aroon uses a lookback period, so it lags in detecting trend onset. By the time Aroon confirms a range-bound state, the ranging period may be ending.
2. **ADX already captures this:** COINCLAW already uses ADX for regime detection. Adding Aroon may be redundant — both are regime indicators that filter the same type of entries.
3. **Crypto markets are always trending somewhat:** BTC and most crypto assets exhibit persistent trends. Requiring a range-bound state may filter out most entries in a perpetually trending market.

---

## Why It Could Succeed

1. **Cleaner range/trend detection than ADX:** ADX rises in both strong trends and volatile ranging markets. Aroon distinguishes between trending (one Aroon near 100) and ranging (both Aroon near 50) more cleanly.
2. **Direct regime confirmation:** Aroon directly measures if price is oscillating (range-bound) vs making consistent new highs/lows (trending). This is exactly the regime information mean-reversion needs.
3. **Different math from ADX:** ADX uses smoothed directional movement. Aroon uses time-since-high/low — a fundamentally different approach that may capture regime information ADX misses.
4. **Well-established indicator:** Aroon was created by Tushar Chande in 1995 and is a standard part of technical analysis toolkits, particularly for regime detection.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN114 Aroon Regime Confirmation |
|--|--|--|
| Regime filter | ADX-based (trend strength) | Aroon-based (trend existence) |
| Range detection | ADX < 25 | Both Aroon < 70 |
| Trend detection | ADX > 25 | One Aroon > 70 |
| LONG filter | ADX context | Aroon Down > Aroon Up + 20 |
| SHORT filter | ADX context | Aroon Up > Aroon Down + 20 |
| Indicator math | Directional movement | Time-since-high/low |
| Regime clarity | Trend vs ranging | Clean oscillator reading |
| Filter strictness | Single ADX value | Triple Aroon condition |
