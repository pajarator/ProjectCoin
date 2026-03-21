# RUN134 — Connors RSI Confirmation: Use Short-Term RSI Streak for Faster Entry Confirmation

## Hypothesis

**Named:** `connors_rsi_confirm`

**Mechanism:** COINCLAW uses standard RSI (14-period) for entry filtering, but standard RSI is relatively slow to respond and can stay overbought/oversold for extended periods. Connors RSI, developed by Larry Connors, is specifically designed for short-term mean-reversion trading — it combines three components: (1) standard RSI on price, (2) RSI on streak length (counting consecutive up/down days), and (3) percentile rank of the price over a lookback period. The combined indicator is faster and more responsive to short-term extremes than standard RSI, making it ideal for regime mean-reversion entries. The Connors RSI Confirmation requires Connors RSI to be below a threshold (e.g., < 25) for LONG entries, providing faster entry signals than standard RSI alone.

**Connors RSI Confirmation:**
- Track Connors RSI on 15m: `ConnorsRSI = (RSI(price, 3) + RSI(streak, 2) + PercentRank(price, 100)) / 3`
- `streak = consecutive up/down closes (+1 per up, -1 per down, resets to 0 on flat)`
- `PercentRank = % of bars in last 100 where price was below current price`
- For regime LONG entry: require `connors_rsi < CONNORS_LONG_MAX` (e.g., < 25)
- For regime SHORT entry: require `connors_rsi > CONNORS_SHORT_MIN` (e.g., > 75)
- This adds a faster, more short-term focused oscillator specifically designed for mean-reversion

**Why this is not a duplicate:**
- RUN132 (RSI divergence) used standard RSI — Connors RSI is a completely different calculation combining RSI + streak RSI + percentile rank
- RUN103 (stochastic extreme exit) used stochastic — Connors RSI uses streak-based RSI, different formula
- RUN131 (VWRSI) used volume-weighted RSI — Connors RSI uses percentile rank and streak, completely different components
- Standard RSI (used in DualRsi and others) is single-component (price only). Connors RSI is 3-component.
- No prior RUN has used Connors RSI as an entry filter

**Mechanistic rationale:** Connors RSI was specifically designed by Larry Connors for short-term mean-reversion trading — exactly what COINCLAW's regime trades attempt. The 3-period RSI component responds quickly to price changes. The streak RSI component penalizes consecutive directional closes (a 5-bar losing streak pushes the streak RSI lower than a single loss). The percentile rank normalizes the price relative to recent history. The combination gives a more responsive and more "mean-reversion-focused" signal than standard RSI. For regime entries, requiring Connors RSI < 25 means the short-term mean-reversion indicator is deeply extreme, giving higher conviction than standard RSI < 40.

---

## Proposed Config Changes

```rust
// RUN134: Connors RSI Confirmation
pub const CONNORS_RSI_ENABLE: bool = true;
pub const CONNORS_LONG_MAX: f64 = 25.0;    // Connors RSI must be below this for LONG
pub const CONNORS_SHORT_MIN: f64 = 75.0;   // Connors RSI must be above this for SHORT
pub const CONNORS_RSI_PERIOD: usize = 3;   // RSI period for price component (standard is 3)
pub const CONNORS_STREAK_PERIOD: usize = 2; // RSI period for streak component (standard is 2)
pub const CONNORS_PCT_PERIOD: usize = 100;  // Percentile rank lookback (standard is 100)
```

**`indicators.rs` — add Connors RSI computation:**
```rust
// Connors RSI = (RSI(price, 3) + RSI(streak, 2) + PercentRank(price, 100)) / 3
// RSI(streak, 2): streak = consecutive up/down closes; up = +1, down = -1, flat = 0
// PercentRank(price, 100) = % of bars in last 100 where price was below current
// Ind15m should have: connors_rsi: f64
```

**`strategies.rs` — add Connors RSI helper functions:**
```rust
/// Compute streak: consecutive up (+1), down (-1), or flat (0) closes.
fn compute_streak(candles: &[Candle15m]) -> i32 {
    if candles.len() < 2 { return 0; }
    let current = candles[candles.len() - 1].c;
    let prior = candles[candles.len() - 2].c;
    if current > prior {
        // Count consecutive ups — find where streak started
        let mut streak = 1i32;
        for i in (0..candles.len() - 1).rev() {
            if candles[i].c > candles[i + 1].c {
                streak += 1;
            } else {
                break;
            }
        }
        streak
    } else if current < prior {
        let mut streak = -1i32;
        for i in (0..candles.len() - 1).rev() {
            if candles[i].c < candles[i + 1].c {
                streak -= 1;
            } else {
                break;
            }
        }
        streak
    } else {
        0
    }
}

/// Check if Connors RSI confirms LONG entry (deeply oversold).
fn connors_confirm_long(ind: &Ind15m) -> bool {
    if !config::CONNORS_RSI_ENABLE { return true; }
    if ind.connors_rsi.is_nan() { return true; }
    ind.connors_rsi < config::CONNORS_LONG_MAX
}

/// Check if Connors RSI confirms SHORT entry (deeply overbought).
fn connors_confirm_short(ind: &Ind15m) -> bool {
    if !config::CONNORS_RSI_ENABLE { return true; }
    if ind.connors_rsi.is_nan() { return true; }
    ind.connors_rsi > config::CONNORS_SHORT_MIN
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN134: Connors RSI Confirmation
    if !connors_confirm_long(ind) { return false; }

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

    // RUN134: Connors RSI Confirmation
    if !connors_confirm_short(ind) { return false; }

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

### RUN134.1 — Connors RSI Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — standard RSI confirmation

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CONNORS_LONG_MAX` | [20, 25, 30] |
| `CONNORS_SHORT_MIN` | [70, 75, 80] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `connors_filter_rate`: % of entries filtered by Connors RSI confirmation
- `connors_vs_rsi_divergence`: average gap between Connors RSI and standard RSI at filtered entries
- `connors_at_allowed`: average Connors RSI at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN134.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best CONNORS_LONG_MAX × CONNORS_SHORT_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Connors RSI-filtered entries (blocked) have lower win rate than Connors RSI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN134.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, RSI only) | Connors RSI Confirmation | Delta |
|--------|----------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| RSI at Blocked LONG | X | X | — |
| Connors RSI at Blocked LONG | — | X | — |
| RSI at Allowed LONG | X | X | — |
| Connors RSI at Allowed LONG | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Connors RSI is short-term focused:** The 3-period RSI component makes Connors RSI very sensitive to recent price changes. It can flip from oversold to overbought very quickly, creating whipsaws in choppy markets.
2. **Streak component is noisy:** The streak count (consecutive up/down closes) can be disrupted by a single flat bar, resetting the streak to 0. This can create instability in the streak RSI component.
3. **May conflict with standard RSI:** If standard RSI (RSI < 40) fires but Connors RSI does NOT confirm (because the move was gradual, not sudden), there's a conflict in the confirmation stack.

---

## Why It Could Succeed

1. **Designed specifically for short-term mean-reversion:** Larry Connors wrote the book on short-term trading strategies. Connors RSI was built for exactly the type of trading COINCLAW does — short-term mean-reversion on 15m bars.
2. **More responsive than standard RSI:** The 3-period RSI component is much faster than the 14-period RSI currently used. Connors RSI catches extremes earlier, giving better entry timing.
3. **Three components provide robustness:** The combination of price RSI + streak RSI + percentile rank means the signal requires agreement across three different dimensions — harder to fake, more reliable.
4. **Percentile rank normalizes:** The percentile rank component normalizes the price relative to recent history, making the signal adaptive to each coin's recent price distribution.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN134 Connors RSI Confirmation |
|--|--|--|
| Entry filter | Standard RSI (14-period) | Connors RSI (3-component) |
| RSI component | Single (price only) | Triple (price + streak + percentile) |
| Responsiveness | Moderate (14-period) | High (3-period + streak) |
| LONG threshold | RSI < 40 | Connors RSI < 25 |
| SHORT threshold | RSI > 60 | Connors RSI > 75 |
| Streak awareness | None | RSI(streak) component |
| Normalization | None | Percentile rank component |
| Mean-reversion focus | Moderate | High (Connors' specialty) |
| Filter type | Single oscillator | Triple composite oscillator |
