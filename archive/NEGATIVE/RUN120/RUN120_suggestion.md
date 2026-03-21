# RUN120 — Mass Index Reversal Filter: Use Mass Index Trend Reversal Detection for Entry Confirmation

## Hypothesis

**Named:** `mass_index_filter`

**Mechanism:** COINCLAW uses oscillators like RSI and z-score to identify mean-reversion extremes, but these can remain extreme for extended periods before reverting. The Mass Index, developed by Donald Dorsey, is specifically designed to identify trend reversals — it measures the narrowing and widening of the High-Low range (not price itself). When the Mass Index rises above a threshold (e.g., 27) and then drops below (e.g., 26.5), it signals a trend reversal is likely. The Mass Index Reversal Filter uses this as an entry confirmation: for regime LONG, require Mass Index has recently peaked and turned down (indicating the downtrend is exhausting), confirming a reversal is imminent.

**Mass Index Reversal Filter:**
- Track Mass Index on 15m: Mass Index = sum of EMA(high-low) ratios over MASS_PERIOD (e.g., 25 bars)
- When Mass Index > MASS_HIGH (e.g., 27): watch for reversal
- When Mass Index drops below MASS_LOW (e.g., 26.5): reversal signal confirmed
- For regime LONG entry: require Mass Index peaked (> 27) and has turned down below 26.5 (downtrend exhausting)
- For regime SHORT entry: require Mass Index peaked (> 27) and has turned down below 26.5 (uptrend exhausting)
- This specifically identifies trend reversal setups, complementing the mean-reversion entries

**Why this is not a duplicate:**
- RUN82 (regime decay detection) used ADX rise — Mass Index is completely different, measuring range narrowing/widening, not directional movement
- RUN114 (Aroon regime confirm) used time-since-high/low — Mass Index uses High-Low range expansion/contraction
- RUN89 (market-wide ADX confirmation) used ADX — Mass Index is a different calculation (EMA range ratios, not directional movement)
- RUN26 (trailing stops) — Mass Index is a reversal detector, not a trailing stop
- RUN88 (trailing z-exit) — completely different mechanism
- No prior RUN has used Mass Index for entry confirmation

**Mechanistic rationale:** The Mass Index was specifically created to identify when a prevailing trend is losing momentum and a reversal is imminent. It's based on the observation that trending markets have expanding High-Low ranges (mass), and when ranges begin to contract after being expanded, the trend is fatiguing. A Mass Index peak above 27 followed by a drop below 26.5 is a well-established reversal signal in technical analysis. For regime mean-reversion entries, requiring this confirmation means entries fire when the prior trend has demonstrated exhaustion — increasing the probability that the mean-reversion has room to work.

---

## Proposed Config Changes

```rust
// RUN120: Mass Index Reversal Filter
pub const MASS_INDEX_ENABLE: bool = true;
pub const MASS_PERIOD: usize = 25;          // Mass Index lookback period (standard is 25)
pub const MASS_HIGH: f64 = 27.0;           // Mass Index peak threshold
pub const MASS_LOW: f64 = 26.5;            // Mass Index reversal confirmation threshold
pub const MASS_LOOKBACK: usize = 2;        // Number of bars to confirm turn-down
```

**`indicators.rs` — add Mass Index computation:**
```rust
// Mass Index = sum over MASS_PERIOD of EMA(high-low) ratio
// where EMA ratio = EMA(high-low, 9) / EMA(high-low, 9) of prior bar
// Or simpler: Mass Index = 9-period EMA(high-low) / 9-period EMA of 9-period EMA(high-low)
// Ind15m should have: mass_index: f64
```

**`strategies.rs` — add Mass Index reversal helper:**
```rust
/// Check if Mass Index confirms LONG reversal (peaked then turned down).
fn mass_index_confirm_long(cs: &CoinState) -> bool {
    use std::collections::VecDeque;
    if !config::MASS_INDEX_ENABLE { return true; }
    let ind = match &cs.ind_15m {
        Some(i) => i,
        None => return true,
    };
    if ind.mass_index.is_nan() { return true; }

    // Need rolling mass history to detect peak-and-turn
    // For simplicity: mass_recent = last 3 mass_index values
    if cs.mass_history.len() < 3 { return true; }

    let recent = &cs.mass_history[cs.mass_history.len() - 3..];
    let peak = recent[0];
    let mid = recent[1];
    let current = recent[2];

    // Confirmed: peaked above MASS_HIGH, then dropped below MASS_LOW
    peak > config::MASS_HIGH && mid > config::MASS_LOW && current <= config::MASS_LOW
}

/// Check if Mass Index confirms SHORT reversal (peaked then turned down).
// Same logic for SHORT — both directions use same reversal signal
fn mass_index_confirm_short(cs: &CoinState) -> bool {
    mass_index_confirm_long(cs)  // same reversal signal for both directions
}
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub mass_history: VecDeque<f64>,  // rolling Mass Index history for reversal detection
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN120: Mass Index Reversal Filter
    if !mass_index_confirm_long(cs) { return false; }

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

    // RUN120: Mass Index Reversal Filter
    if !mass_index_confirm_short(cs) { return false; }

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

### RUN120.1 — Mass Index Reversal Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Mass Index filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MASS_HIGH` | [25, 27, 29] |
| `MASS_LOW` | [24.5, 26.5, 28.5] |
| `MASS_PERIOD` | [19, 25, 31] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `mass_filter_rate`: % of entries filtered by Mass Index reversal requirement
- `mass_index_at_filtered`: average Mass Index of filtered entries (should be rising/not peaked)
- `mass_index_at_allowed`: average Mass Index at peak confirmation of allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN120.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best MASS_HIGH × MASS_LOW per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Mass Index-filtered entries (blocked) have lower win rate than Mass Index-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN120.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Mass) | Mass Index Reversal Filter | Delta |
|--------|----------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Mass Index at Blocked LONG | — | X | — |
| Mass Index at Blocked SHORT | — | X | — |
| Mass Index at Peak LONG | — | X | — |
| Mass Index at Peak SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Mass Index is slow to confirm:** The Mass Index reversal requires a peak above 27 and then a drop below 26.5 — this can take many bars to develop. The reversal confirmation may come too late, missing the best entry point.
2. **Mass Index is obscure:** Unlike RSI, MACD, or even Supertrend, Mass Index is rarely used in practice. Its effectiveness in crypto markets is unproven and may not transfer from the futures markets it was designed for.
3. **Peaks are retrospective:** Identifying a "peak" requires the Mass Index to have already fallen. This is a lagging confirmation — by the time we confirm the peak, the reversal may already be underway.

---

## Why It Could Succeed

1. **Specifically designed for reversals:** The Mass Index was created by Donald Dorsey specifically to identify trend reversals — exactly what regime mean-reversion trades are betting on. It is the right tool for the job.
2. **Detects trend exhaustion:** When the High-Low range contracts after being expanded, the trend is fatiguing. This is independent of price-based oscillators and adds a unique dimension to the confirmation stack.
3. **Dual-direction signal:** The Mass Index peak-and-turn works for both LONG and SHORT reversals — it identifies when any prevailing trend is exhausting, which is the ideal setup for mean-reversion.
4. **Non-price-based:** Mass Index uses the High-Low range, not close price. This makes it independent of price-based oscillators like RSI and z-score, adding genuinely new information.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN120 Mass Index Reversal Filter |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + Mass Index |
| Reversal detection | Z0 / SMA20 | Mass Index peak-and-turn |
| Signal source | Price | High-Low range |
| Trend exhaustion | None | Mass Index contraction |
| LONG filter | z < -1.5 | z < -1.5 + Mass peak |
| SHORT filter | z > 1.5 | z > 1.5 + Mass peak |
| Indicator type | Price oscillators | Range contraction oscillator |
| Reversal confirmation | Lagging (Z0) | Mass Index specific |
