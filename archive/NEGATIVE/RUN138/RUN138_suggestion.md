# RUN138 — Opening Range Breakout Filter: Suppress Regime Entries When Price Has Broken Out of the Opening Range

## Hypothesis

**Named:** `orb_filter`

**Mechanism:** COINCLAW's regime trades are mean-reversion strategies — they work best when price is oscillating within a range. But institutional traders often "range" the market in the first N minutes of a session, then break out. When price has broken out of the opening range (ORB), the market is no longer in a ranging state — it's trending. The Opening Range Breakout Filter uses the opening N bars to define a range, then suppresses regime entries when price has moved beyond that range by ORB_THRESH in either direction, indicating the market is in a trending/breakout state where mean-reversion entries are less reliable.

**Opening Range Breakout Filter:**
- Track the opening range: highest high and lowest low of the first ORB_BARS (e.g., 5) 15m bars of the session
- `orb_upper = max(high of first ORB_BARS)`
- `orb_lower = min(low of first ORB_BARS)`
- `orb_mid = (orb_upper + orb_lower) / 2`
- For regime LONG entry: require price < orb_lower × ORB_BUFFER (not broken below range) — or if already below, not too far below
- For regime SHORT entry: require price > orb_upper × ORB_BUFFER (not broken above range)
- Suppress when price has moved beyond ORB range by more than ORB_THRESH (trending state)
- Sessions reset at UTC midnight (or configurable session start)

**Why this is not a duplicate:**
- RUN84 (session-based partial exit scaling) used UTC session — ORB uses the opening range as a dynamic support/resistance, not just scaling
- RUN91 (hourly z-threshold scaling) used UTC hour — ORB uses the opening N bars of EACH session, not hourly
- No prior RUN has used Opening Range Breakout as an entry filter for regime trades

**Mechanistic rationale:** COINCLAW's SMA20 and z-score filters measure mean-reversion conditions, but they don't capture the market's current state relative to the opening range. When the market opens and spends the first 5 bars establishing a range, that range becomes a "battlefield" for institutional traders. Breaking above the range signals bullish intent; breaking below signals bearish intent. Mean-reversion entries are most reliable when price is oscillating WITHIN the opening range — not when it has already "decided" a direction by breaking out. ORB suppression ensures COINCLAW doesn't fight established breakout momentum.

---

## Proposed Config Changes

```rust
// RUN138: Opening Range Breakout Filter
pub const ORB_ENABLE: bool = true;
pub const ORB_BARS: usize = 5;              // number of bars to establish opening range
pub const ORB_THRESH: f64 = 0.001;          // breakout threshold (price moved this far beyond range)
pub const ORB_BUFFER: f64 = 1.0;           // buffer multiplier (1.0 = at range boundary)
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub orb_upper: f64,    // opening range upper bound
    pub orb_lower: f64,    // opening range lower bound
    pub orb_mid: f64,     // opening range midpoint
    pub orb_established: bool,  // whether ORB has been established this session
    pub orb_session_start: i64, // timestamp of current session start
}
```

**`strategies.rs` — add ORB helper:**
```rust
/// Update ORB each bar. Called each bar.
/// Session resets at UTC midnight.
fn update_orb(cs: &mut CoinState, ind: &Ind15m, timestamp: i64) {
    // Check if new session
    let session_start = timestamp - (timestamp % 86400); // UTC day start
    if session_start != cs.orb_session_start {
        // New session — reset ORB
        cs.orb_established = false;
        cs.orb_session_start = session_start;
        cs.orb_upper = ind.p;
        cs.orb_lower = ind.p;
        cs.orb_mid = ind.p;
    }

    if !cs.orb_established {
        // Update range with current bar
        cs.orb_upper = cs.orb_upper.max(ind.h);
        cs.orb_lower = cs.orb_lower.min(ind.l);

        // Check if we've seen enough bars to establish range
        // Track bar count in session
        cs.orb_bar_count += 1;
        if cs.orb_bar_count >= config::ORB_BARS {
            cs.orb_established = true;
            cs.orb_mid = (cs.orb_upper + cs.orb_lower) / 2.0;
        }
    }
}

/// Check if price has broken out of the opening range.
fn orb_broken(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ORB_ENABLE { return false; }
    if !cs.orb_established { return false; }

    let range_size = cs.orb_upper - cs.orb_lower;
    let threshold = range_size * config::ORB_THRESH;

    // Broken above: price moved above upper by threshold
    let broken_above = ind.p > cs.orb_upper + threshold;
    // Broken below: price moved below lower by threshold
    let broken_below = ind.p < cs.orb_lower - threshold;

    broken_above || broken_below
}

/// Check if ORB allows LONG entry (not broken below range).
fn orb_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ORB_ENABLE { return true; }
    if !cs.orb_established { return true; }
    !orb_broken(cs, ind)
}

/// Check if ORB allows SHORT entry (not broken above range).
fn orb_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ORB_ENABLE { return true; }
    if !cs.orb_established { return true; }
    !orb_broken(cs, ind)
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN138: Opening Range Breakout Filter
    if !orb_confirm_long(cs, ind) { return false; }

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

    // RUN138: Opening Range Breakout Filter
    if !orb_confirm_short(cs, ind) { return false; }

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

### RUN138.1 — ORB Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no ORB filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `ORB_BARS` | [3, 5, 7] |
| `ORB_THRESH` | [0.0005, 0.001, 0.002] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `orb_filter_rate`: % of entries filtered by ORB breakout suppression
- `orb_breakout_at_filtered`: % of filtered entries where price had broken out
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN138.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best ORB_BARS × ORB_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- ORB-filtered entries (blocked) have lower win rate than ORB-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN138.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no ORB) | Opening Range Breakout Filter | Delta |
|--------|---------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| ORB Breakout at Filtered | — | X% | — |
| ORB Range at Allowed | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Session start is arbitrary:** UTC midnight is a human-defined session start. Crypto markets trade 24/7 — there may not be a meaningful "opening" in the traditional sense. The concept of an "opening range" may not apply well to crypto.
2. **ORB only established once per session:** If the market breaks out early in the session, COINCLAW suppresses entries for the rest of the day. But the breakout could fail and mean-reversion could work. The suppression is too broad.
3. **Breakout vs mean-reversion timing:** The market often breaks out of the opening range only to mean-revert later. Suppressing entries for the rest of the session after a breakout may miss the best opportunities.

---

## Why It Could Succeed

1. **Institutional session setup:** Institutional traders often establish positions in the first 30-60 minutes of a session. The opening range becomes support/resistance. Breaking out signals their intent. COINCLAW suppressing entries after a breakout avoids fighting institutional direction.
2. **Simple and intuitive:** ORB is one of the most widely-used concepts in futures trading (ORB was popularized by Toby Crabel). Its application to crypto is natural.
3. **Dynamic support/resistance:** The opening range adapts to each session's price action — it's not a fixed level, but a dynamic zone established by actual price movement.
4. **Session-level discipline:** ORB adds session-level awareness to COINCLAW's bar-level entries. The system now knows when the market has "decided a direction" for the session.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN138 Opening Range Breakout Filter |
|--|--|--|
| Session awareness | None | Opening range breakout |
| Entry filter | z-score + RSI + vol | + ORB breakout suppression |
| Support/resistance | SMA20 | Opening range bounds |
| Breakout detection | None | ORB range boundary |
| Session concept | None | UTC day + ORB bars |
| Mean-reversion timing | Any bar | Within opening range |
| Trending detection | None | Price beyond ORB threshold |
| Institutional alignment | None | ORB breakout = institutional intent |
