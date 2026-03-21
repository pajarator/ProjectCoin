# RUN133 — Ultimate Oscillator Confirmation: Use Multi-Timeframe Oscillator for Smoother Entry Confirmation

## Hypothesis

**Named:** `ultimate_osc_confirm`

**Mechanism:** COINCLAW uses RSI as a single-timeframe oscillator (typically 14 periods) for entry filtering. Single-timeframe oscillators can be noisy and give false signals. The Ultimate Oscillator (UO), developed by Larry Williams, combines three different timeframes (7, 14, and 28 periods) into one oscillator using weighted averages, reducing false signals and capturing momentum from multiple timeframes simultaneously. The Ultimate Oscillator Confirmation requires UO to be in extreme territory (below 30 for LONG, above 70 for SHORT), ensuring the multi-timeframe momentum confirms the entry.

**Ultimate Oscillator Confirmation:**
- Track UO on 15m: `UO = 100 × (4×avg7 + 2×avg14 + avg28) / (4 + 2 + 1)`
- Where `avg7 = 7-period average of buying pressure`, `avg14 = 14-period average`, `avg28 = 28-period average`
- `Buying Pressure = Close - min(Low, Prior Close)`
- For regime LONG entry: require `uo < UO_LONG_MAX` (e.g., < 30 = deeply oversold)
- For regime SHORT entry: require `uo > UO_SHORT_MIN` (e.g., > 70 = deeply overbought)
- The multi-timeframe synthesis gives smoother, more robust signals than single-period RSI

**Why this is not a duplicate:**
- RUN85 (momentum pulse filter) used single ROC — UO combines three timeframes, completely different
- RUN103 (stochastic extreme exit) used stochastic — UO uses buying pressure across 3 timeframes, different math
- RUN116 (KST confirm) used KST — KST uses multiple ROC periods, UO uses multiple buying pressure periods, similar concept different implementation
- RSI is single-timeframe (14 period). UO combines 7, 14, 28 periods — genuinely multi-timeframe
- RUN13 (complement signals) used KST Cross — UO was never used as an entry filter
- No prior RUN has used Ultimate Oscillator as an entry filter

**Mechanistic rationale:** The Ultimate Oscillator was specifically designed by Larry Williams to solve the problem of false signals from single-timeframe oscillators. By combining three timeframes (short, medium, long), the oscillator smooths out noise and captures genuine momentum shifts. When UO is below 30 (deeply oversold), it means the weighted combination of all three timeframes is confirming bearish pressure — a stronger signal than RSI < 30 alone because RSI < 30 might just be a single-timeframe anomaly.

---

## Proposed Config Changes

```rust
// RUN133: Ultimate Oscillator Confirmation
pub const ULTIMATE_CONFIRM_ENABLE: bool = true;
pub const UO_PERIOD1: usize = 7;         // Short period (standard is 7)
pub const UO_PERIOD2: usize = 14;        // Medium period (standard is 14)
pub const UO_PERIOD3: usize = 28;        // Long period (standard is 28)
pub const UO_LONG_MAX: f64 = 30.0;     // UO must be below this for LONG (deeply oversold)
pub const UO_SHORT_MIN: f64 = 70.0;     // UO must be above this for SHORT (deeply overbought)
```

**`indicators.rs` — add Ultimate Oscillator computation:**
```rust
// Buying Pressure (BP) = Close - min(Low, Prior Close)
// True Range (TR) = max(High, Prior Close) - min(Low, Prior Close)
// Average7 = 7-period SMA of BP
// Average14 = 14-period SMA of BP
// Average28 = 28-period SMA of BP
// UO = 100 × (4×Avg7 + 2×Avg14 + Avg28) / (4 + 2 + 1)
// Ind15m should have: uo: f64
```

**`strategies.rs` — add Ultimate Oscillator helper functions:**
```rust
/// Check if Ultimate Oscillator confirms LONG entry (deeply oversold).
fn ultimate_confirm_long(ind: &Ind15m) -> bool {
    if !config::ULTIMATE_CONFIRM_ENABLE { return true; }
    if ind.uo.is_nan() { return true; }
    ind.uo < config::UO_LONG_MAX
}

/// Check if Ultimate Oscillator confirms SHORT entry (deeply overbought).
fn ultimate_confirm_short(ind: &Ind15m) -> bool {
    if !config::ULTIMATE_CONFIRM_ENABLE { return true; }
    if ind.uo.is_nan() { return true; }
    ind.uo > config::UO_SHORT_MIN
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN133: Ultimate Oscillator Confirmation
    if !ultimate_confirm_long(ind) { return false; }

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

    // RUN133: Ultimate Oscillator Confirmation
    if !ultimate_confirm_short(ind) { return false; }

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

### RUN133.1 — Ultimate Oscillator Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Ultimate Oscillator filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `UO_LONG_MAX` | [25, 30, 35] |
| `UO_SHORT_MIN` | [65, 70, 75] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `uo_filter_rate`: % of entries filtered by Ultimate Oscillator confirmation
- `uo_at_filtered`: average UO at filtered entries
- `uo_at_allowed`: average UO at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN133.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best UO_LONG_MAX × UO_SHORT_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- UO-filtered entries (blocked) have lower win rate than UO-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN133.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no UO) | Ultimate Oscillator Confirmation | Delta |
|--------|---------------------|------------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| UO at Blocked LONG | — | X | — |
| UO at Blocked SHORT | — | X | — |
| UO at Allowed LONG | — | X | — |
| UO at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **UO is complex with multiple periods:** The three-period calculation is more complex than RSI. The weighting (4×7 + 2×14 + 28) is somewhat arbitrary and may not be optimal for crypto 15m bars.
2. **UO may be too slow:** Using 28 bars for the longest period means UO reflects long-term momentum more than short-term. The signal may lag when fast entries are needed.
3. **RSI already covers similar ground:** RSI is already used extensively. UO is essentially RSI with multiple timeframes — it may be redundant.

---

## Why It Could Succeed

1. **Multi-timeframe smoothing:** By combining 7, 14, and 28 periods, UO smooths out bar-to-bar noise that single-period oscillators suffer from. Signals are more robust.
2. **Larry Williams' most important indicator:** UO was Larry Williams' answer to the problem of false oscillator signals. It's specifically designed to reduce whipsaws from single-timeframe oscillators.
3. **Buying pressure formulation:** UO uses Buying Pressure (Close - min(Low, Prior Close)) rather than the standard RSI formula. This is a different calculation that may capture momentum differently.
4. **Fewer false signals:** The multi-timeframe synthesis means UO doesn't flip as easily as RSI when a single bar is extreme. The weight of multiple timeframes must agree for UO to be extreme.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN133 Ultimate Oscillator Confirmation |
|--|--|--|
| Oscillator | Single-period RSI (14) | Multi-period UO (7+14+28) |
| Timeframe | 14-period only | 3 timeframes combined |
| LONG threshold | RSI < 40 | UO < 30 |
| SHORT threshold | RSI > 60 | UO > 70 |
| Smoothing | None | Weighted 3-period average |
| False signal reduction | Low | High (multi-timeframe) |
| Indicator type | Price-only oscillator | Buying pressure oscillator |
| Weighting | N/A | 4×7 + 2×14 + 28 |
