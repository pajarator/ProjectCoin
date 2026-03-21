# RUN89 — Market-Wide ADX Confirmation: Suppress Regime Entries During High-Trend Environments

## Hypothesis

**Named:** `market_adx_confirm`

**Mechanism:** COINCLAW uses per-coin ADX within `detect_regime()` to classify each coin's regime (Ranging, WeakTrend, StrongTrend). But a single coin's ADX can be noisy. A market-wide average ADX across all 18 coins is a cleaner measure of whether the overall market is trending. When average ADX is high (e.g., > 25), the market is in a broadly trending state — regime mean reversion trades are lower probability because the whole market is moving directionally.

**Market-Wide ADX Confirmation:**
- Compute average ADX across all 18 coins each bar (already available via `MarketCtx.avg_rsi` — add ADX similarly)
- When `avg_adx >= ADX_SUPPRESS_THRESHOLD` (e.g., 25.0):
  - Suppress regime LONG entries (long mean reversion doesn't work in trending markets)
  - Allow ISO_SHORT entries to continue (shorting in a trending market can be high-conviction)
- When `avg_adx >= ADX_ALLOW_THRESHOLD` (e.g., 30.0):
  - Suppress ALL regime entries including ISO_SHORT (market too trending for any mean reversion)
- ADX is computed per-coin already — just need to average across all coins in `compute_breadth_and_context`

**Why this is not a duplicate:**
- RUN43 (breadth velocity) tracks market-wide breadth changes — this tracks market-wide ADX
- RUN82 (regime decay exit) monitors per-coin ADX change while IN a position — this gates entries based on market-wide ADX level
- RUN56 (SMA depth) uses per-coin ADX indirectly via regime detection — this uses market-wide ADX as a direct entry gate
- No prior RUN has used average ADX across the portfolio as a market-wide entry filter

**Mechanistic rationale:** A high market-wide ADX means the entire market is in a trending state — BTC, ETH, and altcoins are all making directional moves. Mean reversion works in range-bound markets, not trending ones. When avg ADX > 25, the probability of regime LONG trades succeeding drops significantly. Suppressing entries during high-ADX periods avoids fighting market-wide trends.

---

## Proposed Config Changes

```rust
// RUN89: Market-Wide ADX Confirmation
pub const MARKET_ADX_CONFIRM_ENABLE: bool = true;
pub const MARKET_ADX_SUPPRESS_LONG: f64 = 25.0;   // suppress LONG entries when avg ADX >= 25
pub const MARKET_ADX_SUPPRESS_ALL: f64 = 30.0;    // suppress ALL regime entries when avg ADX >= 30
```

**`coordinator.rs` — MarketCtx additions:**
```rust
pub struct MarketCtx {
    pub avg_z: f64,
    pub avg_rsi: f64,
    pub btc_z: f64,
    pub avg_adx: f64,                  // NEW: market-wide average ADX
    pub avg_z_valid: bool,
    pub avg_rsi_valid: bool,
    pub btc_z_valid: bool,
}

pub fn compute_breadth_and_context(state: &SharedState) -> (f64, usize, usize, MarketMode, MarketCtx) {
    // ... existing code ...
    let mut adx_values = Vec::new();
    for cs in &state.coins {
        if let Some(ref ind) = cs.ind_15m {
            if !ind.adx.is_nan() {
                adx_values.push(ind.adx);
            }
        }
    }
    let avg_adx = if !adx_values.is_empty() {
        adx_values.iter().sum::<f64>() / adx_values.len() as f64
    } else { 0.0 };

    let ctx = MarketCtx {
        // ... existing fields ...
        avg_adx,
    };
}
```

**`engine.rs` — market_adx_entry_check in check_entry:**
```rust
/// Check if market-wide ADX allows regime entries.
fn market_adx_allows_entry(state: &SharedState, mode: MarketMode, dir: Direction) -> bool {
    if !config::MARKET_ADX_CONFIRM_ENABLE { return true; }

    let avg_adx = state.coins.iter()
        .filter_map(|cs| cs.ind_15m.as_ref().map(|i| i.adx))
        .filter(|&a| !a.is_nan())
        .fold(0.0, |s, a| s + a)
        / state.coins.len() as f64;

    if avg_adx >= config::MARKET_ADX_SUPPRESS_ALL {
        return false;  // too trending — no regime entries
    }

    if avg_adx >= config::MARKET_ADX_SUPPRESS_LONG && dir == Direction::Long {
        return false;  // trending — suppress LONG entries only
    }

    true
}

// In check_entry — before open_position:
if !market_adx_allows_entry(state, mode, dir) {
    return;  // suppress entry due to high market-wide ADX
}
```

---

## Validation Method

### RUN89.1 — Market ADX Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no market-wide ADX filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MARKET_ADX_SUPPRESS_LONG` | [20.0, 22.0, 25.0, 28.0] |
| `MARKET_ADX_SUPPRESS_ALL` | [25.0, 28.0, 30.0, 32.0] |

**Per coin:** 4 × 4 = 16 configs × 18 coins = 288 backtests

**Key metrics:**
- `adx_suppression_rate`: % of bars where ADX filter blocks entries
- `avg_adx_at_entries`: average market-wide ADX at filtered-in vs filtered-out entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN89.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best ADX threshold pair per window
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- ADX suppression rate 10–35% (meaningful filtering without over-suppressing)
- HIGH_ADX trades have lower WR than LOW_ADX trades (confirming filter is working)

### RUN89.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no ADX filter) | Market ADX Confirmation | Delta |
|--------|------------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| ADX Suppression Rate | 0% | X% | — |
| Avg ADX at Entries | X | X | — |
| HIGH_ADX Entry WR% | X% | X% | +Ypp |
| LOW_ADX Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **ADX is a lagging indicator:** By the time avg ADX rises above the threshold, the trend may already be reversing. The filter may suppress entries at exactly the wrong time — just as the market is about to range again.
2. **High ADX doesn't mean bad mean reversion:** A trending market can still have mean reversion within the trend — coins that are extreme relative to their own moving averages can still revert within a broadly trending market.
3. **Over-suppression:** If the threshold is too tight, most entries get blocked during what turns out to be a normal, profitable period.

---

## Why It Could Succeed

1. **Market-wide trends are real:** When BTC is making a directional move, alts tend to follow. Regime LONG trades in this environment are fighting the trend. Suppressing them avoids low-probability setups.
2. **ISO shorts can still fire:** Unlike a pure circuit breaker, this filter allows ISO_SHORT entries during high ADX (shorting a trending market is high-conviction). This preserves some portfolio activity during trending periods.
3. **ADX is already computed:** No new indicators needed. Just averaging per-coin ADX values.
4. **Institutional practice:** Trend filters are standard — most systematic equity strategies have market regime filters that halt long exposure when broad markets are in confirmed trends.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN89 Market ADX Confirmation |
|--|--|--|
| Market trend awareness | None (per-coin regime only) | Market-wide avg ADX filter |
| LONG entries suppressed | Never | When avg ADX ≥ 25 |
| ISO_SHORT entries suppressed | Never | When avg ADX ≥ 30 |
| Regime entries in trending | All allowed | Blocked above threshold |
| Trend detection | Per-coin ADX | Portfolio-wide average ADX |
| ISO_SHORT treatment | Always allowed | Allowed until extreme trending |
