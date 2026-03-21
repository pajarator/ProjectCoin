# RUN140 — Keltner Channel Breakout Filter: Use ATR-Based Channels for Trend/Range Detection

## Hypothesis

**Named:** `keltner_channel_filter`

**Mechanism:** COINCLAW uses Bollinger Bands (SMA ± standard deviation) for mean-reversion entry confirmation, but Bollinger Bands can be noisy on 15m bars. The Keltner Channel, developed by Chester Keltner, uses EMA ± ATR × multiplier instead — ATR-based channels are smoother and more stable than Bollinger Bands because ATR changes more slowly than standard deviation. The Keltner Channel Breakout Filter uses the channel to detect trending vs ranging markets: when price closes above the upper Keltner band, the market is in a trending state (suppress LONG entries); when price is within the bands, the market is ranging (allow entries).

**Keltner Channel Breakout Filter:**
- Track Keltner Channel: `keltner_upper = EMA(close, KC_PERIOD) + MULT × ATR(KC_PERIOD)`, `keltner_lower = EMA(close, KC_PERIOD) - MULT × ATR(KC_PERIOD)`, `keltner_mid = EMA(close, KC_PERIOD)`
- For regime LONG entry: require price < keltner_upper AND price > keltner_lower (within channel) — NOT broken out above
- For regime SHORT entry: require price > keltner_lower AND price < keltner_upper (within channel) — NOT broken out below
- This ensures entries only fire when price is within the Keltner Channel (ranging), not when in breakout mode

**Why this is not a duplicate:**
- RUN97 (BB width scalp gate) used Bollinger Band width — Keltner Channel uses ATR instead of standard deviation, different calculation and more stable
- RUN13 (complement signals) used KST Cross, Kalman Filter — Keltner Channel is not used as an entry filter
- RUN110 (BB compression entry) used BB width compression — Keltner Channel uses ATR-based bands, different mechanism
- Bollinger Bands use SMA ± std dev. Keltner Channel uses EMA ± ATR — fundamentally different calculation
- No prior RUN has used Keltner Channel as an entry filter for regime trades

**Mechanistic rationale:** Keltner Channels were specifically designed to identify trending vs ranging markets. When price closes outside the upper band, it indicates a strong trending move — in this state, mean-reversion entries are fighting the trend and should be suppressed. When price is inside the channel, it indicates a ranging market — exactly the environment where mean-reversion strategies work best. The ATR-based bands are smoother than Bollinger Bands, providing more stable trend/range detection that is less sensitive to single-bar price spikes.

---

## Proposed Config Changes

```rust
// RUN140: Keltner Channel Breakout Filter
pub const KELTNER_ENABLE: bool = true;
pub const KC_PERIOD: usize = 20;           // EMA and ATR period (standard is 20)
pub const KC_MULT: f64 = 2.0;           // ATR multiplier for bands (standard is 2.0)
```

**`indicators.rs` — add Keltner Channel computation:**
```rust
// Keltner Channel:
// EMA = EMA(close, KC_PERIOD)
// ATR = ATR(KC_PERIOD) = EMA(true_range, KC_PERIOD)
// Upper = EMA + KC_MULT × ATR
// Lower = EMA - KC_MULT × ATR
// Mid = EMA
// Ind15m should have: keltner_upper: f64, keltner_lower: f64, keltner_mid: f64
```

**`strategies.rs` — add Keltner Channel helper functions:**
```rust
/// Check if price is within Keltner Channel (ranging) — good for mean-reversion.
fn keltner_in_channel(ind: &Ind15m) -> bool {
    if !config::KELTNER_ENABLE { return true; }
    if ind.keltner_upper.is_nan() || ind.keltner_lower.is_nan() { return true; }
    ind.p < ind.keltner_upper && ind.p > ind.keltner_lower
}

/// Check if price has broken out above Keltner Channel (trending).
fn keltner_broken_above(ind: &Ind15m) -> bool {
    if !config::KELTNER_ENABLE { return false; }
    if ind.keltner_upper.is_nan() { return false; }
    ind.p > ind.keltner_upper
}

/// Check if price has broken out below Keltner Channel (trending).
fn keltner_broken_below(ind: &Ind15m) -> bool {
    if !config::KELTNER_ENABLE { return false; }
    if ind.keltner_lower.is_nan() { return false; }
    ind.p < ind.keltner_lower
}

/// Check if Keltner Channel allows LONG entry (within channel, not broken out).
fn keltner_confirm_long(ind: &Ind15m) -> bool {
    if !config::KELTNER_ENABLE { return true; }
    // Must be within channel — not in breakout mode
    if keltner_broken_above(ind) { return false; }
    keltner_in_channel(ind)
}

/// Check if Keltner Channel allows SHORT entry (within channel, not broken out).
fn keltner_confirm_short(ind: &Ind15m) -> bool {
    if !config::KELTNER_ENABLE { return true; }
    // Must be within channel — not in breakout mode
    if keltner_broken_below(ind) { return false; }
    keltner_in_channel(ind)
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN140: Keltner Channel Breakout Filter
    if !keltner_confirm_long(ind) { return false; }

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

    // RUN140: Keltner Channel Breakout Filter
    if !keltner_confirm_short(ind) { return false; }

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

### RUN140.1 — Keltner Channel Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Keltner Channel filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `KC_PERIOD` | [14, 20, 30] |
| `KC_MULT` | [1.5, 2.0, 2.5] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `keltner_filter_rate`: % of entries filtered by Keltner Channel breakout
- `keltner_in_channel_rate`: % of allowed entries that were within channel
- `keltner_breakout_rate_at_filtered`: % of filtered entries where price had broken out
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN140.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best KC_PERIOD × KC_MULT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Keltner-filtered entries (blocked) have lower win rate than Keltner-confirmed entries
- Filter rate 10–35%

### RUN140.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Keltner) | Keltner Channel Breakout Filter | Delta |
|--------|-------------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Breakout at Blocked LONG | — | X% | — |
| Breakout at Blocked SHORT | — | X% | — |
| In-Channel at Allowed | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Keltner Channel bands can be wide:** With KC_MULT = 2.0, the bands can be quite wide, meaning price is "within channel" even during significant moves. The filter may not be strict enough to suppress breakout-mode entries.
2. **ATR changes slowly:** Keltner Channel uses ATR, which changes more slowly than standard deviation. The bands may be too slow to adapt to sudden volatility changes.
3. **EMA-based channels may lag:** The Keltner Channel's EMA baseline may lag in fast-moving markets, making the channel boundaries outdated by the time we check them.

---

## Why It Could Succeed

1. **Smoother than Bollinger Bands:** Keltner Channel uses ATR (which is a smoothed measure of range) instead of standard deviation (which can spike on single bars). This makes the channel more stable and less prone to false breakout signals.
2. **ATR is more stable than StdDev:** ATR is an exponentially smoothed average of true range. Standard deviation can be volatile. The Keltner Channel's ATR-based bands give more stable trend/range boundaries.
3. **Clear breakout detection:** When price closes outside the Keltner Channel, it indicates a significant move beyond the normal range — exactly the condition where mean-reversion strategies fail.
4. **Complements Bollinger Bands:** If COINCLAW already uses BB for some signals, Keltner provides an independent view using ATR. Entries that pass both BB AND Keltner filters have higher conviction.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN140 Keltner Channel Breakout Filter |
|--|--|--|
| Channel type | None | Keltner (EMA ± ATR) |
| Trend detection | None | Price outside channel = trending |
| Range detection | None | Price inside channel = ranging |
| LONG filter | z < -1.5 + RSI | + price within Keltner Channel |
| SHORT filter | z > 1.5 + RSI | + price within Keltner Channel |
| Band calculation | N/A | EMA ± ATR × mult |
| Smoothing | N/A | ATR-based (more stable) |
| Breakout detection | None | Price beyond Keltner bands |
