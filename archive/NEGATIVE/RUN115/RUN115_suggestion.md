# RUN115 — Supertrend Confirmation: Use Supertrend as Trailing Stop and Entry Direction Filter

## Hypothesis

**Named:** `supertrend_confirm`

**Mechanism:** COINCLAW uses fixed stop losses and SMA20 crossback for exits. The Supertrend indicator is a volatility-based trailing stop that adapts to market conditions — it uses Average True Range (ATR) with a multiplier to create an upper and lower band that acts as a dynamic stop. When price is below the lower Supertrend band, it confirms a DOWNTREND (sell signal); when above the upper band, it confirms an UPTREND (buy signal). The Supertrend Confirmation uses Supertrend as both an entry direction filter AND a trailing stop: entries are only allowed when Supertrend confirms the direction aligns with the trade, and exits are triggered when Supertrend flips.

**Supertrend Confirmation:**
- Track Supertrend on 15m: `supertrend_upper = mid + (mult × ATR)`, `supertrend_lower = mid - (mult × ATR)`, where `mid = (high + low) / 2`
- For regime LONG entry: require price above Supertrend lower band (no downtrend confirmation)
- For regime SHORT entry: require price below Supertrend upper band (no uptrend confirmation)
- As trailing stop: exit LONG when price crosses below supertrend_lower; exit SHORT when price crosses above supertrend_upper
- This adds adaptive volatility-based stop that self-adjusts to market conditions

**Why this is not a duplicate:**
- RUN26 (trailing stops) used fixed % trailing — Supertrend uses ATR-based adaptive stops, completely different mechanism
- RUN76 (volatility-adaptive SL) scaled SL by ATR — Supertrend is a band-based system, not a scalar multiplier on SL
- RUN88 (trailing z-exit) used z-score recovery — Supertrend is ATR-based, not z-score based
- RUN103 (stochastic extreme exit) used stochastic — Supertrend is ATR-based with multiplier, completely different
- RUN99 (z-momentum divergence) — completely different mechanism
- No prior RUN has used Supertrend as either an entry filter or trailing stop mechanism

**Mechanistic rationale:** Supertrend is a well-established technical indicator that naturally adapts to volatility. During high-volatility periods, the ATR widens, making the Supertrend bands wider and giving trades more room. During low-volatility periods, the bands tighten, providing earlier exit signals. This self-adaptive nature makes it superior to fixed-percentage trailing stops for regime trading, where volatility regimes change. Additionally, Supertrend's directional confirmation ensures entries only fire when the short-term trend doesn't contradict the mean-reversion trade.

---

## Proposed Config Changes

```rust
// RUN115: Supertrend Confirmation
pub const SUPERTREND_ENABLE: bool = true;
pub const SUPERTREND_MULT: f64 = 3.0;      // ATR multiplier (standard is 3)
pub const SUPERTREND_ENTRY_FILTER: bool = true;  // use Supertrend as entry direction filter
pub const SUPERTREND_TRAIL_STOP: bool = true;     // use Supertrend as trailing stop
```

**`indicators.rs` — add Supertrend computation:**
```rust
// Supertrend needs ATR (already in Ind15m) and high/low/close
// Upper = mid + (mult * ATR)
// Lower = mid - (mult * ATR)
// where mid = (high + low) / 2
// Final upper/lower toggles based on close vs prior upper/lower
```

**`strategies.rs` — add Supertrend entry filter:**
```rust
/// Check if Supertrend allows LONG entry (price above lower band = no downtrend).
fn supertrend_confirm_long(ind: &Ind15m) -> bool {
    if !config::SUPERTREND_ENABLE || !config::SUPERTREND_ENTRY_FILTER { return true; }
    if ind.supertrend_lower.is_nan() { return true; }
    ind.p > ind.supertrend_lower
}

/// Check if Supertrend allows SHORT entry (price below upper band = no uptrend).
fn supertrend_confirm_short(ind: &Ind15m) -> bool {
    if !config::SUPERTREND_ENABLE || !config::SUPERTREND_ENTRY_FILTER { return true; }
    if ind.supertrend_upper.is_nan() { return true; }
    ind.p < ind.supertrend_upper
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN115: Supertrend Entry Filter
    if !supertrend_confirm_long(ind) { return false; }

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

    // RUN115: Supertrend Entry Filter
    if !supertrend_confirm_short(ind) { return false; }

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

**`engine.rs` — add Supertrend trail stop exit:**
```rust
/// Check Supertrend trailing stop exit.
// In check_exit:
if config::SUPERTREND_ENABLE && config::SUPERTREND_TRAIL_STOP {
    // LONG: exit if price crosses below supertrend_lower
    if trade.direction == Direction::Long && !cs.supertrend_lower.is_nan() {
        if cs.ind_15m.as_ref().map(|i| i.p < i.supertrend_lower).unwrap_or(false) {
            return ExitReason::SupertrendTrail;
        }
    }
    // SHORT: exit if price crosses above supertrend_upper
    if trade.direction == Direction::Short && !cs.supertrend_upper.is_nan() {
        if cs.ind_15m.as_ref().map(|i| i.p > i.supertrend_upper).unwrap_or(false) {
            return ExitReason::SupertrendTrail;
        }
    }
}
```

---

## Validation Method

### RUN115.1 — Supertrend Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed 0.3% SL, no Supertrend

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SUPERTREND_MULT` | [2.0, 3.0, 4.0] |
| `SUPERTREND_ENTRY_FILTER` | [true, false] |
| `SUPERTREND_TRAIL_STOP` | [true, false] |

**Per coin:** 3 × 2 × 2 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `supertrend_filter_rate`: % of entries filtered by Supertrend entry filter
- `supertrend_exit_rate`: % of trades exiting via Supertrend trail
- `supertrend_exit_wr`: win rate of Supertrend trail exits
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_SL_distance_delta`: change in effective stop loss distance

### RUN115.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best SUPERTREND_MULT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Supertrend trail exits have higher win rate than fixed SL exits
- Entry filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN115.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed SL) | Supertrend Confirmation | Delta |
|--------|----------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Supertrend Filter Rate | 0% | X% | — |
| Supertrend Exit Rate | 0% | X% | — |
| Fixed SL Exit Rate | X% | X% | -Y% |
| Supertrend Exit WR% | — | X% | — |
| Avg Effective Stop Dist | 0.3% | X% | +/-N% |

---

## Why This Could Fail

1. **Supertrend is a trend-following tool:** COINCLAW is a mean-reversion system. Using a trend-following indicator like Supertrend as an entry filter may filter out valid mean-reversion setups that occur against the short-term trend.
2. **ATR multiplier is arbitrary:** The optimal multiplier (2, 3, 4) is coin/timeframe specific and may not generalize. A multiplier that's too tight causes premature exits; too loose gives too much room.
3. **ATR itself is volatile:** ATR changes over time, making the Supertrend bands unstable. The band can widen significantly during high-volatility events, causing the trail stop to be far from price.

---

## Why It Could Succeed

1. **Adaptive to volatility:** Supertrend self-adjusts based on ATR — it gives more room in volatile markets and less room in calm markets. This is exactly what a good trailing stop should do.
2. **Self-adaptive stop beats fixed SL:** A fixed 0.3% SL is arbitrary. Supertrend's ATR-based stop is data-driven and adapts to the coin's natural volatility range.
3. **Dual use:** Works as both entry filter (avoid counter-trend entries) and trailing stop (lock in profits adaptively). This is efficient — one indicator serves two purposes.
4. **Well-established practitioner tool:** Supertrend was popularized by Oliver Seban and is widely used in futures/forex trading. Its application to crypto regime trading is natural.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN115 Supertrend Confirmation |
|--|--|--|
| Stop loss | Fixed 0.3% | Adaptive Supertrend ATR-based |
| Trailing stop | None | Supertrend band crossover |
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + Supertrend |
| Trend awareness | None | Supertrend lower/upper band |
| Stop adaptation | None (fixed) | ATR-based (vol-adaptive) |
| Exit mechanism | Z0/SMA20/MAX_HOLD | + Supertrend band flip |
| Stop distance | 0.3% fixed | ATR × mult (varies) |
| Indicator type | Mean-reversion | Trend-following (used for mean-rev) |
