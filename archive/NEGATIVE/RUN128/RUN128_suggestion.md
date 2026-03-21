# RUN128 — Schaff Trend Cycle Confirmation: Use Schaff Trend Cycle for Fast Trend/Momentum Confirmation

## Hypothesis

**Named:** `schaff_trend_confirm`

**Mechanism:** COINCLAW uses oscillators like RSI and MACD for entry signals, but these can be slow to respond or give false signals in choppy markets. The Schaff Trend Cycle (STC), developed by Doug Schaff, is an improved trend/momentum indicator that combines the best of MACD (moving average convergence/divergence) with a faster cycle component. STC produces faster, cleaner signals than MACD because it uses a cycle rhythm (based on the stochastics concept) to smooth the MACD, removing the lag. The Schaff Trend Cycle Confirmation uses STC as an entry filter: for LONG entries, require STC to be below STC_LONG_THRESH (oversold territory turning up), confirming momentum is turning; for SHORT entries, require STC to be above STC_SHORT_THRESH (overbought territory turning down).

**Schaff Trend Cycle Confirmation:**
- Track STC on 15m: MACD(12,26) with values fed through a stochastic-like %K with a 10-bar cycle
- For regime LONG entry: require `stc < STC_LONG_THRESH` (e.g., < 25 = oversold) AND `stc_rising` (STC crossed above prior bar)
- For regime SHORT entry: require `stc > STC_SHORT_THRESH` (e.g., > 75 = overbought) AND `stc_falling` (STC crossed below prior bar)
- STC's cycle-based smoothing gives faster, cleaner signals than MACD

**Why this is not a duplicate:**
- RUN116 (KST confirm filter) used KST — STC is different: KST uses multiple ROC periods weighted, STC uses MACD with stochastic cycle smoothing
- RUN85 (momentum pulse filter) used ROC — STC combines MACD with cycle-based smoothing, fundamentally different approach
- RUN99 (z-momentum divergence) used z-score momentum — STC is MACD-based, completely different calculation
- RUN13 (complement signals) used KST Cross, Kalman Filter — STC was not used as primary entry gate
- No prior RUN has used Schaff Trend Cycle as an entry filter for regime trades

**Mechanistic rationale:** STC was specifically designed to be faster than MACD while maintaining accuracy. It achieves this by using a cycle rhythm — the same concept that makes stochastic oscillators fast and responsive. The MACD provides the trend baseline; the stochastic-like cycle removes the lag. For regime mean-reversion entries, requiring STC to be in oversold territory (below 25) and turning up means: (1) momentum has reached an extreme, and (2) the turn is confirmed by the cycle component. This is more responsive than waiting for MACD to cross its signal line, and cleaner than raw stochastic because it has the MACD trend baseline built in.

---

## Proposed Config Changes

```rust
// RUN128: Schaff Trend Cycle Confirmation
pub const SCHAFF_CONFIRM_ENABLE: bool = true;
pub const STC_FAST_PERIOD: usize = 12;        // STC fast EMA period (MACD-style, standard is 12)
pub const STC_SLOW_PERIOD: usize = 26;        // STC slow EMA period (standard is 26)
pub const STC_CYCLE_PERIOD: usize = 10;       // STC cycle period (standard is 10)
pub const STC_LONG_THRESH: f64 = 25.0;       // STC must be below this for LONG (oversold)
pub const STC_SHORT_THRESH: f64 = 75.0;       // STC must be above this for SHORT (overbought)
pub const STC_TURN_REQUIRED: bool = true;     // require STC to be turning in trade direction
```

**`indicators.rs` — add Schaff Trend Cycle computation:**
```rust
// STC = Stochastic(MACD, cycle_period)
// MACD = EMA(close, fast) - EMA(close, slow)
// %K = (MACD - min(MACD, cycle_period)) / (max(MACD, cycle_period) - min(MACD, cycle_period)) * 100
// %D = EMA(%K, cycle_period)
// Ind15m should have: stc: f64
```

**`strategies.rs` — add STC helper functions:**
```rust
/// Check if STC confirms LONG entry (oversold and turning up).
fn stc_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::SCHAFF_CONFIRM_ENABLE { return true; }
    if ind.stc.is_nan() { return true; }
    if ind.stc >= config::STC_LONG_THRESH { return false; }
    if config::STC_TURN_REQUIRED {
        // Check if STC is rising (turning up from oversold)
        let prior_stc = cs.prior_stc.unwrap_or(ind.stc);
        if ind.stc <= prior_stc { return false; }
    }
    true
}

/// Check if STC confirms SHORT entry (overbought and turning down).
fn stc_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::SCHAFF_CONFIRM_ENABLE { return true; }
    if ind.stc.is_nan() { return true; }
    if ind.stc <= config::STC_SHORT_THRESH { return false; }
    if config::STC_TURN_REQUIRED {
        let prior_stc = cs.prior_stc.unwrap_or(ind.stc);
        if ind.stc >= prior_stc { return false; }
    }
    true
}
```

**`state.rs` — CoinState addition:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub prior_stc: Option<f64>,  // prior bar's STC value for turn detection
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN128: Schaff Trend Cycle Confirmation
    if !stc_confirm_long(cs, ind) { return false; }

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

    // RUN128: Schaff Trend Cycle Confirmation
    if !stc_confirm_short(cs, ind) { return false; }

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

### RUN128.1 — Schaff Trend Cycle Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no STC filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `STC_LONG_THRESH` | [20, 25, 30] |
| `STC_SHORT_THRESH` | [70, 75, 80] |
| `STC_TURN_REQUIRED` | [true, false] |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `stc_filter_rate`: % of entries filtered by STC confirmation
- `stc_at_filtered`: average STC at filtered entries
- `stc_at_allowed`: average STC at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN128.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best STC_LONG_THRESH × STC_SHORT_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- STC-filtered entries (blocked) have lower win rate than STC-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN128.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no STC) | Schaff Trend Cycle Confirmation | Delta |
|--------|----------------------|----------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| STC at Blocked LONG | — | X | — |
| STC at Blocked SHORT | — | X | — |
| STC at Allowed LONG | — | X | — |
| STC at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **STC is not well-known:** Unlike MACD, RSI, or Stochastic, STC is relatively obscure. Its behavior in crypto markets is not well-characterized. It may not transfer well from the forex/commodities markets it was designed for.
2. **Cycle period is sensitive:** STC's cycle period (typically 10) is a key parameter that affects responsiveness. An incorrect period can make STC either too noisy or too slow.
3. **STC turn detection is noisy:** Requiring STC to be "turning" (rising for LONG) is a bar-to-bar comparison that can be noisy in choppy markets.

---

## Why It Could Succeed

1. **Faster than MACD:** STC's cycle-based smoothing makes it faster than MACD — signals come earlier. This means entries fire at better prices.
2. **Cleaner than Stochastic:** Unlike raw Stochastic (which can oscillate rapidly), STC has the MACD trend baseline, making it cleaner and less prone to false signals.
3. **Combines trend + cycle:** STC uniquely combines the trend-following MACD with the cycle-based Stochastic — best of both worlds.
4. **Well-regarded in forex:** STC was popularized in forex trading and is used by practitioners who trade short-term mean-reversion in that market. Its application to crypto is natural.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN128 Schaff Trend Cycle Confirmation |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + STC |
| Indicator type | Single oscillators | MACD + Stochastic hybrid |
| Signal speed | MACD (moderate lag) | STC (low lag) |
| LONG filter | z < -1.5 + RSI | + STC < 25 + rising |
| SHORT filter | z > 1.5 + RSI | + STC > 75 + falling |
| Oscillator type | MACD, RSI, Stochastic | STC (hybrid) |
| Choppiness handling | Multiple filters | STC cycle smoothing |
| Trend baseline | MACD | Built into STC |
