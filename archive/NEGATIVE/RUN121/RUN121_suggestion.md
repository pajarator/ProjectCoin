# RUN121 — TD Sequential Entry Filter: Use Thomas DeMark's TD Sequential for Countdown Entry Confirmation

## Hypothesis

**Named:** `td_sequential_filter`

**Mechanism:** COINCLAW uses z-score extremes and oscillators for entry signals, but these don't account for the *count* of directional price bars. Thomas DeMark's TD Sequential is a powerful countdown/countdown system that identifies when price has been moving in one direction for an extended period and measures the probability of reversal at specific counts. The TD Sequential has two phases: a Setup (which counts bars of directional movement) and a Countdown (which counts bars of momentum exhaustion). The TD Sequential Entry Filter uses the TD Setup as an entry gate: require a completed TD Setup (9 consecutive bars closing in the trade direction) before allowing regime entries, ensuring entries fire only when price has demonstrated sustained directional movement and is therefore likely to reverse.

**TD Sequential Entry Filter:**
- Track TD Sequential Setup on 15m: 9 consecutive bars where close closes greater than the close 4 bars prior (for LONG setup) or less than (for SHORT setup)
- A "completed" TD Setup means the 9th bar has closed, confirming sustained directional movement
- For regime LONG entry: require TD LONG Setup completed within last TD_SETUP_BARS bars (e.g., 4 bars ago)
- For regime SHORT entry: require TD SHORT Setup completed within last TD_SETUP_BARS bars
- The 9-bar setup counts directional pressure; when completed, mean-reversion probability increases

**Why this is not a duplicate:**
- RUN99 (z-momentum divergence) used z-score momentum — TD Sequential counts directional closes, completely different mechanism
- RUN85 (momentum pulse filter) used ROC — TD Sequential uses close-vs-close-4-bars-ago, different math
- RUN105 (z-persistence filter) required z beyond threshold for N consecutive bars — TD Sequential uses directional close comparisons, not z-score
- RUN120 (Mass Index) used High-Low range — TD Sequential uses close price comparisons, different data
- RUN114 (Aroon) used time-since-high/low — TD Sequential uses directional close comparison
- RUN13 (complement signals) used KST, Kalman Filter — TD Sequential is a countdown system, not a moving average oscillator
- No prior RUN has used TD Sequential as an entry filter for regime trades

**Mechanistic rationale:** Mean-reversion works because price that has moved too far in one direction must return. The TD Sequential quantifies "how far" by counting consecutive bars of directional closes — 9 bars of closes above the close 4 bars ago means sustained buying pressure that has built up "exhaustion energy." When this 9-bar setup completes and then a regime entry fires, the trade has higher probability because the directional pressure has been building. This is different from oscillators that measure current momentum — TD counts the *accumulation* of directional bars, a unique dimension.

---

## Proposed Config Changes

```rust
// RUN121: TD Sequential Entry Filter
pub const TD_SEQUENTIAL_ENABLE: bool = true;
pub const TD_SETUP_LENGTH: usize = 9;       // TD Sequential setup length (standard is 9)
pub const TD_SETUP_LOOKBACK: usize = 4;      // Bars since setup completion required (must complete within last N bars)
pub const TD_STRICT_COUNT: bool = true;      // Require exactly 9 (not >= 9)
```

**`strategies.rs` — add TD Sequential helper:**
```rust
/// Compute TD Sequential LONG setup count (consecutive closes > close 4 bars ago).
fn td_long_setup_count(ind: &Ind15m, cs: &CoinState) -> usize {
    // For TD LONG setup: close_t > close_{t-4}
    // Count consecutive bars satisfying this condition
    // Need at least 4 bars of history to compare
    // Returns count from 0 (no setup) to 9 (complete setup)
    let closes = &cs.candles_15m;
    if closes.len() < TD_SETUP_LOOKBACK + 1 {
        return 0;
    }

    let mut count = 0;
    let start = closes.len() - 1;
    let end = closes.len().saturating_sub(config::TD_SETUP_LENGTH + 1);

    for i in (end..start).rev() {
        let current_close = closes[i].c;
        let compare_close = closes[i.saturating_sub(TD_SETUP_LOOKBACK)].c;
        if current_close > compare_close {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Compute TD Sequential SHORT setup count (consecutive closes < close 4 bars ago).
fn td_short_setup_count(ind: &Ind15m, cs: &CoinState) -> usize {
    let closes = &cs.candles_15m;
    if closes.len() < TD_SETUP_LOOKBACK + 1 {
        return 0;
    }

    let mut count = 0;
    let start = closes.len() - 1;
    let end = closes.len().saturating_sub(config::TD_SETUP_LENGTH + 1);

    for i in (end..start).rev() {
        let current_close = closes[i].c;
        let compare_close = closes[i.saturating_sub(TD_SETUP_LOOKBACK)].c;
        if current_close < compare_close {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Check if TD Sequential confirms LONG entry (recent completed setup).
fn td_confirm_long(cs: &CoinState) -> bool {
    if !config::TD_SEQUENTIAL_ENABLE { return true; }
    let count = td_long_setup_count(&cs.ind_15m.as_ref().unwrap_or(&Ind15m::default()), cs);
    if config::TD_STRICT_COUNT {
        count >= config::TD_SETUP_LENGTH
    } else {
        count >= config::TD_SETUP_LENGTH.min(6)  // allow partial setups
    }
}

/// Check if TD Sequential confirms SHORT entry (recent completed setup).
fn td_confirm_short(cs: &CoinState) -> bool {
    if !config::TD_SEQUENTIAL_ENABLE { return true; }
    let count = td_short_setup_count(&cs.ind_15m.as_ref().unwrap_or(&Ind15m::default()), cs);
    if config::TD_STRICT_COUNT {
        count >= config::TD_SETUP_LENGTH
    } else {
        count >= config::TD_SETUP_LENGTH.min(6)
    }
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN121: TD Sequential Entry Filter
    if !td_confirm_long(cs) { return false; }

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

    // RUN121: TD Sequential Entry Filter
    if !td_confirm_short(cs) { return false; }

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

### RUN121.1 — TD Sequential Entry Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no TD Sequential filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `TD_SETUP_LENGTH` | [7, 9, 11] |
| `TD_SETUP_LOOKBACK` | [3, 4, 5] |
| `TD_STRICT_COUNT` | [true, false] |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `td_filter_rate`: % of entries filtered by TD Sequential requirement
- `td_count_at_filtered`: average TD count of filtered entries (should be incomplete)
- `td_count_at_allowed`: average TD count of allowed entries (should be complete)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN121.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best TD_SETUP_LENGTH × LOOKBACK per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- TD-filtered entries (blocked) have lower win rate than TD-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN121.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no TD) | TD Sequential Entry Filter | Delta |
|--------|---------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| TD Count at Blocked LONG | — | X | — |
| TD Count at Blocked SHORT | — | X | — |
| TD Count at Allowed LONG | — | X | — |
| TD Count at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **TD Sequential is complex and subjective:** The TD Sequential has many rules and variations. Implementing it correctly is non-trivial, and different practitioners interpret the rules differently. The implementation here is simplified and may not capture the full system.
2. **9-bar count is arbitrary:** The count of 9 is a fixed number that may not be optimal for crypto 15m bars. Crypto markets may need different counts than what DeMark calibrated for equities.
3. **Choppy markets break the count:** In choppy markets, the close-4-bars-ago comparison flips frequently, breaking the setup. This may filter out too many valid mean-reversion opportunities in sideways markets.

---

## Why It Could Succeed

1. **Quantifies directional pressure:** The TD Sequential counts the *number of bars* of sustained directional movement — a unique dimension that oscillators don't capture. A completed 9-bar setup means real directional pressure has built up, and mean-reversion is more likely after such pressure exhausts.
2. **Respects the trend-first principle:** The TD Sequential's premise is "trend before reversal." Price must demonstrate directional commitment (9 bars of closes in one direction) before a reversal is expected. This is exactly what regime mean-reversion needs — entries after a sustained move.
3. **Institutional-grade tool:** Thomas DeMark is one of the most respected technical analysts in the world. His TD Sequential is used by major institutional traders. Applying it to crypto regime trading is a principled approach.
4. **Drop-in filter:** The TD Sequential filter adds a time dimension (bar count) to the existing oscillator-based filters, complementing without replacing.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN121 TD Sequential Entry Filter |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + TD Sequential |
| Directional pressure | None | 9-bar close comparison |
| Time dimension | None (oscillators only) | Bar count of directional moves |
| Setup concept | None | Close vs close 4 bars ago |
| LONG filter | z < -1.5 + RSI + vol | + TD LONG setup (9 bars) |
| SHORT filter | z > 1.5 + RSI + vol | + TD SHORT setup (9 bars) |
| Trend respect | SMA20 vs price | TD 9-bar directional count |
| Reversal readiness | Z0 / SMA20 | TD setup completion |
