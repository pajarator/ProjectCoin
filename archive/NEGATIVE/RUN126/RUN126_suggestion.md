# RUN126 — Ichimoku Cloud Confirmation: Use Ichimoku Cloud Components for Multi-Dimensional Entry Filtering

## Hypothesis

**Named:** `ichimoku_confirm`

**Mechanism:** COINCLAW uses SMA20 and z-score for trend filtering, but these are single-line measurements. The Ichimoku Cloud system, developed by Goichi Hosoda, consists of multiple components that together paint a comprehensive picture of support, resistance, trend, and momentum: Tenkan-Sen (fast conversion line), Kijun-Sen (slow baseline), Senkou Span A/B (cloud boundaries), and Chikou Span (lagging span). The Ichimoku Cloud Confirmation uses multiple Ichimoku components as entry filters: price must be above the cloud for SHORT (and below for LONG), Tenkan must have crossed Kijun in the right direction, and Chikou must confirm current price action.

**Ichimoku Cloud Confirmation:**
- Track Ichimoku components on 15m: Tenkan (9-period high/low midpoint), Kijun (26-period high/low midpoint), Senkou A/B (midpoints projected forward)
- For regime LONG entry: require `price < cloud_lower` AND `Tenkan > Kijun` (bullish conversion cross) AND `Chikou > price_n_bars_ago` (confirming momentum)
- For regime SHORT entry: require `price > cloud_upper` AND `Tenkan < Kijun` (bearish conversion cross) AND `Chikou < price_n_bars_ago`
- This combines support/resistance (cloud), trend (Tenkan/Kijun cross), and momentum (Chikou) into one multi-dimensional filter

**Why this is not a duplicate:**
- RUN13 (complement signals) used KST Cross, Kalman Filter, Elder Ray — Ichimoku Cloud is a fundamentally different system with multiple interconnected components
- RUN88 (trailing z-exit), RUN99 (z-momentum divergence) — completely different mechanism
- RUN114 (Aroon) used time-since-high/low — Ichimoku uses midpoints of high/low ranges, different math
- RUN95 (scalp momentum alignment) used 15m z < -0.5 for scalp — Ichimoku's multi-component system is more comprehensive
- No prior RUN has used Ichimoku Cloud as an entry filter for regime trades

**Mechanistic rationale:** Ichimoku was designed as an "all-in-one" technical system — it provides support/resistance (the cloud), trend direction (Tenkan/Kijun cross), and momentum (Chikou) in one coherent framework. For regime mean-reversion entries, requiring multiple Ichimoku confirmations ensures the trade has: (1) support/resistance alignment from the cloud boundaries, (2) recent bullish cross confirmation from Tenkan/Kijun, and (3) momentum confirmation from Chikou. This multi-dimensional confirmation is more robust than any single-line indicator.

---

## Proposed Config Changes

```rust
// RUN126: Ichimoku Cloud Confirmation
pub const ICHIMOKU_ENABLE: bool = true;
pub const ICHIMOKU_TENKAN_PERIOD: usize = 9;     // Tenkan-sen period (standard is 9)
pub const ICHIMOKU_KIJUN_PERIOD: usize = 26;     // Kijun-sen period (standard is 26)
pub const ICHIMOKU_CLOUD_CONFIRM: bool = true;    // require price below(above) cloud for LONG(SHORT)
pub const ICHIMOKU_CROSS_CONFIRM: bool = true;     // require Tenkan/Kijun cross confirmation
pub const ICHIMOKU_CHIKOU_CONFIRM: bool = true;   // require Chikou span confirmation
```

**`indicators.rs` — add Ichimoku computation:**
```rust
// Tenkan-sen = (highest high + lowest low) / 2 over TENKAN_PERIOD
// Kijun-sen = (highest high + lowest low) / 2 over KIJUN_PERIOD
// Senkou Span A = (Tenkan + Kijun) / 2, projected forward KIJUN_PERIOD bars
// Senkou Span B = (highest high + lowest low) / 2 over (TENKAN+KIJUN)/2 period, projected forward
// Chikou Span = current close, plotted KIJUN_PERIOD bars behind
// Ind15m should have: tenkan: f64, kijun: f64, senkou_a: f64, senkou_b: f64, cloud_upper: f64, cloud_lower: f64, chikou: f64
```

**`strategies.rs` — add Ichimoku helper functions:**
```rust
/// Check if Ichimoku Cloud confirms LONG entry.
fn ichimoku_confirm_long(cs: &CoinState, ind: &Ind15m) -> bool {
    use std::cmp::Ordering;
    if !config::ICHIMOKU_ENABLE { return true; }

    // Cloud confirmation: price must be below cloud
    if config::ICHIMOKU_CLOUD_CONFIRM {
        if ind.cloud_lower.is_nan() || ind.p >= ind.cloud_lower { return false; }
    }

    // Tenkan/Kijun cross: Tenkan must be above Kijun (bullish)
    if config::ICHIMOKU_CROSS_CONFIRM {
        if ind.tenkan.is_nan() || ind.kijun.is_nan() { return true; }
        if ind.tenkan <= ind.kijun { return false; }
    }

    // Chikou confirmation: Chikou must be above price KIJUN_PERIOD bars ago
    if config::ICHIMOKU_CHIKOU_CONFIRM {
        if ind.chikou.is_nan() { return true; }
        // Chikou is current close projected KIJUN bars behind — compare to prior price
        let closes = &cs.candles_15m;
        if closes.len() < config::ICHIMOKU_KIJUN_PERIOD + 1 { return true; }
        let prior_price = closes[closes.len() - 1 - config::ICHIMOKU_KIJUN_PERIOD].c;
        if ind.chikou <= prior_price { return false; }
    }

    true
}

/// Check if Ichimoku Cloud confirms SHORT entry.
fn ichimoku_confirm_short(cs: &CoinState, ind: &Ind15m) -> bool {
    if !config::ICHIMOKU_ENABLE { return true; }

    // Cloud confirmation: price must be above cloud
    if config::ICHIMOKU_CLOUD_CONFIRM {
        if ind.cloud_upper.is_nan() || ind.p <= ind.cloud_upper { return false; }
    }

    // Tenkan/Kijun cross: Tenkan must be below Kijun (bearish)
    if config::ICHIMOKU_CROSS_CONFIRM {
        if ind.tenkan.is_nan() || ind.kijun.is_nan() { return true; }
        if ind.tenkan >= ind.kijun { return false; }
    }

    // Chikou confirmation: Chikou must be below price KIJUN_PERIOD bars ago
    if config::ICHIMOKU_CHIKOU_CONFIRM {
        if ind.chikou.is_nan() { return true; }
        let closes = &cs.candles_15m;
        if closes.len() < config::ICHIMOKU_KIJUN_PERIOD + 1 { return true; }
        let prior_price = closes[closes.len() - 1 - config::ICHIMOKU_KIJUN_PERIOD].c;
        if ind.chikou >= prior_price { return false; }
    }

    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN126: Ichimoku Cloud Confirmation
    if !ichimoku_confirm_long(cs, ind) { return false; }

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

    // RUN126: Ichimoku Cloud Confirmation
    if !ichimoku_confirm_short(cs, ind) { return false; }

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

### RUN126.1 — Ichimoku Cloud Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Ichimoku filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `ICHIMOKU_CLOUD_CONFIRM` | [true, false] |
| `ICHIMOKU_CROSS_CONFIRM` | [true, false] |
| `ICHIMOKU_CHIKOU_CONFIRM` | [true, false] |

**Per coin:** 2 × 2 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `ichimoku_filter_rate`: % of entries filtered by Ichimoku confirmation
- `cloud_thickness_at_filtered`: cloud thickness at filtered entries
- `tenkan_kijun_state_at_allowed`: Tenkan/Kijun relationship at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN126.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best component combination per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Ichimoku-filtered entries (blocked) have lower win rate than Ichimoku-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN126.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no Ichimoku) | Ichimoku Cloud Confirmation | Delta |
|--------|--------------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Cloud Below at Blocked LONG | — | X% | — |
| Tenkan > Kijun at Allowed LONG | — | X% | — |
| Chikou Confirm at Allowed LONG | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Ichimoku is complex and parameter-heavy:** Ichimoku has multiple components with fixed periods (9, 26, 52). These periods were calibrated for daily trading on the Nikkei in the 1960s. They may not be optimal for 15m crypto bars.
2. **Ichimoku is a trend-following system:** Ichimoku was designed for trend-following (identify trend direction and ride it). Using it as a filter for mean-reversion (which is counter-trend) may be contradictory — Ichimoku's bullish signals fire when price is already trending up.
3. **Cloud provides resistance/support, not entry signals:** The cloud (Senkou Span A/B) provides future support/resistance zones, but these are not precise entry triggers. Requiring price below cloud for LONG may be too broad a filter.

---

## Why It Could Succeed

1. **Multi-dimensional confirmation:** Ichimoku provides support/resistance (cloud), trend (Tenkan/Kijun cross), and momentum (Chikou) in one integrated system. No other indicator in COINCLAW provides this range of confirmation in one package.
2. **Cloud as dynamic support/resistance:** The Senkou Span A/B cloud provides a dynamic zone of support/resistance that adapts to recent price action. For mean-reversion, price below the cloud means the cloud acts as resistance — a good environment for LONG.
3. **Tenkan/Kijun cross is a leading signal:** The conversion/baseline cross happens before the trend fully establishes. For mean-reversion, a bullish cross after a downtrend signals the downtrend may be ending — exactly the right time for a mean-reversion entry.
4. **Chikou confirms momentum:** The lagging span confirms whether current price action is genuine by comparing to prior price. For mean-reversion, Chikou above prior price confirms upward momentum is present.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN126 Ichimoku Cloud Confirmation |
|--|--|--|
| Entry filter | SMA20 + z-score | Ichimoku multi-component |
| Support/resistance | None | Cloud (Senkou A/B) |
| Trend direction | SMA20 vs price | Tenkan/Kijun cross |
| Momentum | None | Chikou span |
| LONG confirmation | price < SMA20 | Price below cloud + T>K cross + Chikou |
| SHORT confirmation | price > SMA20 | Price above cloud + T<K cross + Chikou |
| Filter dimensions | 1 (price vs MA) | 3 (cloud + cross + momentum) |
| Indicator type | Single line | Multi-component system |
| Timeframe | 15m | 15m (Ichimoku standard periods) |
