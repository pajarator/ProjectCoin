# RUN122 — Elder Ray Bull Power Confirmation: Use Dr. Alexander Elder's Bull Power for Entry Direction

## Hypothesis

**Named:** `elder_ray_confirm`

**Mechanism:** COINCLAW uses z-score, RSI, and volume for regime entry signals, but these measure price deviation and momentum without isolating the *strength* of the bulls or bears independently. Dr. Alexander Elder's Elder Ray indicator separates market power into Bull Power (how far price can push above the EMA) and Bear Power (how far price can push below the EMA). Bull Power = High - EMA(13); Bear Power = Low - EMA(13). The Elder Ray Bull Power Confirmation uses Bull Power as an entry filter: for regime LONG entries, require Bull Power to be negative but rising (bears weakening, bulls starting to push), confirming the mean-reversion entry has directional backing.

**Elder Ray Bull Power Confirmation:**
- Track Bull Power on 15m: `bull_power = high - EMA(13)`
- Track Bear Power on 15m: `bear_power = low - EMA(13)`
- For regime LONG entry: require `bull_power < 0` AND `bull_power > bull_power_prior` (bears losing grip, bulls starting to push up)
- For regime SHORT entry: require `bear_power > 0` AND `bear_power < bear_power_prior` (bulls losing grip, bears starting to push down)
- This ensures entries fire when the specific power dynamic confirms the mean-reversion direction

**Why this is not a duplicate:**
- RUN99 (z-momentum divergence) used z-score momentum divergence — Elder Ray uses High/Low vs EMA, completely different calculation
- RUN85 (momentum pulse filter) used ROC — Elder Ray uses High/Low minus EMA, different data and math
- RUN108 (momentum hour filter) used 1m ROC — Elder Ray uses EMA differential, different timeframe and calculation
- RUN114 (Aroon) used time-since-high/low — Elder Ray uses High/Low minus EMA, different normalization
- RUN119 (Vortex) used VI+/VI- — Elder Ray is High/Low minus EMA vs directional movement ratio, different math
- RUN13 (complement signals) used KST, Kalman Filter, Elder Ray was mentioned as complement but NOT as primary filter gate
- No prior RUN has used Elder Ray Bull/Bear Power as an entry filter gate for regime trades

**Mechanistic rationale:** Elder Ray was specifically designed by Dr. Alexander Elder to give traders a window into the battle between bulls and bears. Bull Power measures the bulls' ability to push price above the EMA — when Bull Power is negative but rising, it means the bears have pushed price below EMA but the bulls are starting to regain strength. This is the exact micro-structure that precedes a mean-reversion bounce. For regime LONG entries, requiring rising Bull Power ensures the trade has bull-side momentum confirmation, not just z-score extreme.

---

## Proposed Config Changes

```rust
// RUN122: Elder Ray Bull Power Confirmation
pub const ELDER_RAY_ENABLE: bool = true;
pub const ELDER_RAY_EMA_PERIOD: usize = 13;   // Elder Ray EMA period (standard is 13)
pub const ELDER_BULL_CONFIRM: bool = true;      // require bull power rising for LONG
pub const ELDER_BEAR_CONFIRM: bool = true;     // require bear power falling for SHORT
```

**`indicators.rs` — add Elder Ray computation:**
```rust
// Bull Power = High - EMA(close, 13)
// Bear Power = Low - EMA(close, 13)
// Ind15m should have: bull_power: f64, bear_power: f64
```

**`strategies.rs` — add Elder Ray helper functions:**
```rust
/// Get prior bar's Elder Ray values (need state for prior bar).
fn elder_ray_prior(cs: &CoinState) -> (f64, f64) {
    let candles = &cs.candles_15m;
    if candles.len() < 2 {
        return (0.0, 0.0);
    }
    let prior = &candles[candles.len() - 2];
    // Need EMA(prior_close, 13) - compute or store prior EMA
    // For simplicity, if we have prior EMA stored:
    let prior_ema = cs.prior_ema13.unwrap_or(0.0);
    let prior_bull = prior.h - prior_ema;
    let prior_bear = prior.l - prior_ema;
    (prior_bull, prior_bear)
}

/// Check if Elder Ray confirms LONG entry (bull power negative but rising).
fn elder_ray_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ELDER_RAY_ENABLE || !config::ELDER_BULL_CONFIRM { return true; }
    if ind.bull_power.is_nan() { return true; }

    // Bull power must be negative (price below EMA = bearish pressure)
    // But rising (less negative or positive = bulls gaining)
    let (prior_bull, _) = elder_ray_prior(cs);
    ind.bull_power < 0.0 && ind.bull_power > prior_bull
}

/// Check if Elder Ray confirms SHORT entry (bear power positive but falling).
fn elder_ray_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ELDER_RAY_ENABLE || !config::ELDER_BEAR_CONFIRM { return true; }
    if ind.bear_power.is_nan() { return true; }

    // Bear power must be positive (price above EMA = bullish pressure)
    // But falling (less positive or negative = bears gaining)
    let (_, prior_bear) = elder_ray_prior(cs);
    ind.bear_power > 0.0 && ind.bear_power < prior_bear
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN122: Elder Ray Bull Power Confirmation
    if !elder_ray_confirm_long(cs, ind) { return false; }

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

    // RUN122: Elder Ray Bear Power Confirmation
    if !elder_ray_confirm_short(cs, ind) { return false; }

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

**`state.rs` — CoinState addition:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub prior_ema13: Option<f64>,  // prior bar's EMA(13) for Elder Ray calculation
}
```

---

## Validation Method

### RUN122.1 — Elder Ray Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Elder Ray filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `ELDER_RAY_EMA_PERIOD` | [9, 13, 21] |
| `ELDER_BULL_CONFIRM` | [true, false] |
| `ELDER_BEAR_CONFIRM` | [true, false] |

**Per coin:** 3 × 2 × 2 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `elder_filter_rate`: % of entries filtered by Elder Ray confirmation
- `bull_power_at_filtered`: average bull power at filtered LONG entries
- `bull_power_at_allowed`: average bull power at allowed LONG entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN122.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best ELDER_RAY_EMA_PERIOD per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Elder Ray-filtered entries (blocked) have lower win rate than Elder Ray-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN122.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Elder) | Elder Ray Confirmation | Delta |
|--------|----------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Bull Power at Blocked LONG | — | X | — |
| Bull Power at Allowed LONG | — | X | — |
| Bear Power at Blocked SHORT | — | X | — |
| Bear Power at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Bull/Bear Power requires EMA continuity:** Elder Ray depends on EMA(13) continuity across bars. Any data gaps or anomalies can disrupt the calculation, making it unreliable in crypto markets with occasional data issues.
2. **Rising Bull Power can be noise:** A single bar of rising Bull Power may be noise, not a genuine reversal signal. The filter may give false confirmation in choppy markets.
3. **Not specifically designed for mean-reversion:** Elder Ray was designed as a trend-following tool (part of the Triple Screen system). Using it for mean-reversion confirmation may be an unconventional application.

---

## Why It Could Succeed

1. **Isolates bull vs bear strength:** Elder Ray uniquely separates the bulls' power (High - EMA) from the bears' power (Low - EMA). This gives granular insight into which side is gaining or losing — exactly what mean-reversion needs to know.
2. **Rising Bull Power confirms bounce:** When Bull Power transitions from negative (bears in control) to rising, it means the bulls are starting to push. This is the micro-structure confirmation that a mean-reversion bounce has support.
3. **Institutional-grade tool:** Dr. Alexander Elder is one of the most respected trading educators. His Elder Ray indicator is widely used in futures and equity trading. Its application to crypto is natural.
4. **Different from Vortex and Aroon:** Vortex uses High/Low/Close comparisons. Aroon uses time-since-high/low. Elder Ray uses High/Low minus EMA — a clean separation of directional power from the exponential moving average baseline.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN122 Elder Ray Confirmation |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + Elder Ray |
| Directional power | None | Bull/Bear Power (High/Low - EMA) |
| LONG filter | z < -1.5 + RSI + vol | + Bull Power < 0 and rising |
| SHORT filter | z > 1.5 + RSI + vol | + Bear Power > 0 and falling |
| Trend baseline | SMA20 | EMA(13) |
| Power measurement | Price vs SMA | High/Low vs EMA |
| Bull/Bear isolation | None | Bull Power, Bear Power separate |
| Entry conviction | Oscillator extremes | Bull/Bear momentum change |
