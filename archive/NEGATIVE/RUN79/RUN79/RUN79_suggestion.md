# RUN79 — Breadth-Adaptive Position Sizing: Scale Risk With Market Regime Health

## Hypothesis

**Named:** `breadth_risk_scaling`

**Mechanism:** COINCLAW currently uses a fixed `RISK = 10%` for all regime trades regardless of how extended the market is. But breadth level is a direct measure of market health: when breadth is very low (e.g., 5-10%), almost all coins are near their means — regime LONG entries are strong because the market is primed for mean reversion. When breadth is very high (e.g., 40-50%), the market is deeply oversold — ISO shorts are the high-conviction trades, and LONG entries are fighting a bearish tide.

**Breadth-adaptive position sizing:**
- Measure current breadth at each bar
- Scale `RISK` for regime trades inversely with breadth distance from the nearest threshold:
  - Low breadth (5-15%): market is healthy → increase LONG risk × BREADTH_LONG_BOOST (e.g., 1.3×)
  - High breadth (35-50%): market is oversold → reduce LONG risk × BREADTH_LONG_REDUCE (e.g., 0.7×), increase ISO_SHORT risk × BREADTH_ISO_BOOST (e.g., 1.3×)
- Never exceed `RISK_MAX = 15%` or go below `RISK_MIN = 5%`
- Momentum trades (RUN27/28) are exempt — they operate on different logic

**Why this is not a duplicate:**
- RUN42 (regime-conditional leverage) scales LEVERAGE by regime type — this scales RISK by breadth level, a continuous market health metric
- RUN52 (z-confidence sizing) scales position size by entry z-score — this scales by breadth (market-wide condition, not per-trade entry quality)
- RUN19 (Kelly sizing) uses aggregate historical stats — this uses current breadth as a real-time market condition input
- No prior RUN has used breadth as a direct position sizing multiplier

**Mechanistic rationale:** Breadth is the clearest available measure of market-wide positioning. When almost all coins are near their means (low breadth), mean reversion long entries are working with the market. When most coins are far below means (high breadth), mean reversion short entries (ISO shorts) are working with the market. Scaling risk to align with market health reduces exposure to low-probability regime entries.

---

## Proposed Config Changes

```rust
// RUN79: Breadth-Adaptive Position Sizing
pub const BREADTH_RISK_ENABLE: bool = true;
pub const BREADTH_RISK_BASE: f64 = 0.10;       // baseline RISK (current default)
pub const BREADTH_RISK_MIN: f64 = 0.05;        // minimum RISK (5%)
pub const BREADTH_RISK_MAX: f64 = 0.15;        // maximum RISK (15%)
pub const BREADTH_LONG_BOOST: f64 = 1.30;      // multiply LONG risk when breadth < 15%
pub const BREADTH_LONG_REDUCE: f64 = 0.70;    // multiply LONG risk when breadth > 35%
pub const BREADTH_ISO_BOOST: f64 = 1.30;      // multiply ISO_SHORT risk when breadth > 35%
pub const BREADTH_BOOST_TRANSITION: f64 = 0.15; // breadth zones: 0-15% = low, 15-35% = mid, 35-50% = high
```

**`engine.rs` — compute_effective_risk:**
```rust
/// Compute effective RISK for a regime trade based on current breadth.
pub fn compute_effective_risk(state: &SharedState, trade_dir: Direction, trade_subtype: &str) -> f64 {
    if !config::BREADTH_RISK_ENABLE { return config::RISK; }

    let breadth = state.breadth;

    let base = config::BREADTH_RISK_BASE;
    let effective = match trade_dir {
        Direction::Long => {
            if breadth < config::BREADTH_BOOST_TRANSITION {
                // Low breadth: market healthy — boost LONG risk
                base * config::BREADTH_LONG_BOOST
            } else if breadth > (1.0 - config::BREADTH_BOOST_TRANSITION) {
                // High breadth: market oversold — reduce LONG risk
                base * config::BREADTH_LONG_REDUCE
            } else {
                base
            }
        }
        Direction::Short => {
            // For ISO shorts: boost when breadth is high (oversold = high conviction)
            if breadth > (1.0 - config::BREADTH_BOOST_TRANSITION) {
                base * config::BREADTH_ISO_BOOST
            } else if breadth < config::BREADTH_BOOST_TRANSITION {
                // Low breadth: suppress SHORT risk (market not oversold)
                base * config::BREADTH_LONG_REDUCE  // same reduce factor for shorts in healthy market
            } else {
                base
            }
        }
    };

    effective.clamp(config::BREADTH_RISK_MIN, config::BREADTH_RISK_MAX)
}

// In open_position — replace config::RISK with:
let risk = if trade_type == TradeType::Scalp || trade_type == TradeType::Momentum {
    config::RISK  // scalp and momentum use fixed risk
} else {
    compute_effective_risk(state, dir, strat)  // regime trades use breadth-adaptive risk
};
let trade_amt = cs.bal * risk;
```

**`state.rs` — no changes required**

---

## Validation Method

### RUN79.1 — Breadth-Risk Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed `RISK = 10%` for all regime trades

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BREADTH_LONG_BOOST` | [1.2, 1.3, 1.5] |
| `BREADTH_LONG_REDUCE` | [0.6, 0.7, 0.8] |
| `BREADTH_ISO_BOOST` | [1.2, 1.3, 1.5] |
| `BREADTH_BOOST_TRANSITION` | [0.12, 0.15, 0.18] |

**Per coin:** 3 × 3 × 3 × 3 = 81 configs × 18 coins = 1,458 backtests

**Key metrics:**
- `avg_effective_risk`: average risk used across all regime trades
- `risk_spread`: max_risk_used / min_risk_used at each breadth level
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN79.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best boost/reduce multipliers per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Average effective risk stays within [5%, 15%] range
- Max drawdown does not increase >20% vs baseline

### RUN79.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 10%) | Breadth-Adaptive Risk | Delta |
|--------|--------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Effective Risk | 10% | X% | +/-Xpp |
| Low-Breadth Avg Risk | 10% | X% | +Xpp |
| High-Breadth Avg Risk | 10% | X% | +/-Xpp |
| ISO Short P&L | $X | $X | +$X |
| Regime Long P&L | $X | $X | +$X |

---

## Why This Could Fail

1. **Breadth is mean-reverting by nature:** By the time breadth reaches extreme levels (5% or 45%), the reversion is already underway. Scaling risk at extremes may be too late — the best opportunity was at moderate breadth levels.
2. **Risk scaling amplifies drawdowns:** Using 1.3× risk during low-breadth periods means larger position sizes when the market is already extended. A sudden breadth expansion (market-wide selloff) would hit harder with larger positions.
3. **Coarse discretization:** The boost/reduce model uses sharp transitions at fixed breadth thresholds. The market transitions smoothly, not at discrete boundaries.

---

## Why It Could Succeed

1. **Aligns capital with market health:** More capital deployed when the setup is high-conviction (low breadth for longs), less when conviction is low (high breadth for longs, low breadth for shorts).
2. **Reduces drawdown in oversold markets:** When breadth is high (45%), longs are fighting the tape. Reducing LONG risk during this period prevents large losses from bad LONG entries.
3. **Amplifies ISO short gains:** When breadth is high, ISO shorts are the dominant profit engine. Boosting ISO_SHORT risk during these periods directly increases the highest-conviction trades.
4. **Complementary to existing breadth logic:** COINCLAW already uses breadth for market mode detection. Adding risk scaling is a natural extension, not a new indicator.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN79 Breadth-Adaptive Risk |
|--|--|--|
| Regime position risk | Fixed 10% | Dynamic 5–15% based on breadth |
| LOW breadth (5-15%) LONG risk | 10% | 13-15% |
| HIGH breadth (35-50%) LONG risk | 10% | 5-7% |
| HIGH breadth (35-50%) ISO_SHORT risk | 10% | 13-15% |
| Risk range | 10% (fixed) | 5-15% (adaptive) |
| Drawdown in oversold periods | Full exposure | Reduced exposure |
| ISO short conviction | Fixed | Boosted in high-breadth periods |
