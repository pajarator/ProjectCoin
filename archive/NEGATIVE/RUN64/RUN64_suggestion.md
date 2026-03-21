# RUN64 — Portfolio Position Density Filter: Risk Managing Market crowdedness

## Hypothesis

**Named:** `portfolio_density_filter`

**Mechanism:** COINCLAW currently opens positions on each coin independently, without considering how many other coins have open positions simultaneously. When 12 out of 18 coins all have open positions, the portfolio is highly concentrated — if the market moves against all of them simultaneously (a broad sell-off), the clustered drawdown will be severe.

The hypothesis is that **when too many coins have open positions simultaneously, new entries should be suppressed** until some positions close. This is a portfolio-level risk management mechanism, not an entry-level signal quality filter.

**Density signal:**
```
open_position_count = number of coins with active positions
portfolio_density = open_position_count / total_coins  (0.0 to 1.0)
if portfolio_density > MAX_DENSITY_THRESHOLD:
    block new entries (regardless of how good the signal is)
```

**Why this is not a duplicate:**
- No prior RUN has measured or gated based on number of simultaneous open positions
- All prior RUNs optimize per-coin signals — this is a portfolio-level aggregation
- No prior RUN has considered market "crowdedness" as a risk signal
- This is fundamentally different from breadth (which is about market direction), correlation (which is about coin relationships), and density (which is about portfolio exposure)

---

## Proposed Config Changes

```rust
// RUN64: Portfolio Position Density Filter
pub const DENSITY_FILTER_ENABLE: bool = true;
pub const MAX_DENSITY_THRESHOLD: f64 = 0.60;  // block new entries if >60% of coins have positions
pub const DENSITY_COOLDOWN: u32 = 4;          // bars between density checks
```

**`engine.rs` — check_entry uses density gate:**
```rust
pub fn check_entry(state: &mut SharedState, ci: usize, mode: MarketMode, ctx: &MarketCtx) {
    // Density filter check
    if config::DENSITY_FILTER_ENABLE {
        let open_count = state.coins.iter().filter(|c| c.pos.is_some()).count();
        let density = open_count as f64 / state.coins.len() as f64;
        if density > config::MAX_DENSITY_THRESHOLD {
            return;  // portfolio too crowded — suppress all new entries
        }
    }
    // ... rest of check_entry unchanged ...
}
```

**Note:** This doesn't affect momentum entries (RUN27/28) — those are independent layer decisions. The filter gates regime and scalp entries only.

---

## Validation Method

### RUN64.1 — Portfolio Density Grid Search (Rust, portfolio-level backtest)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no density filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MAX_DENSITY_THRESHOLD` | [0.50, 0.60, 0.70, 0.80] |

4 configs — but this is portfolio-level, so it's evaluated as a single system across all 18 coins.

**Key metrics:**
- `density_block_rate`: % of bars where new entries would be blocked
- `avg_open_positions`: average number of simultaneous open positions
- `portfolio_max_DD_delta`: max drawdown change vs baseline
- `portfolio_PnL_delta`: P&L change vs baseline (opportunity cost of blocked trades)
- `density_event_PnL`: P&L during high-density events (when filter was active)

### RUN64.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `MAX_DENSITY_THRESHOLD` on portfolio metrics
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio OOS max_DD < baseline portfolio max_DD (primary)
- Portfolio OOS P&L ≥ 90% of baseline (acceptable opportunity cost)

### RUN64.3 — Combined Comparison

Side-by-side (portfolio-level):

| Metric | Baseline (v16) | Density Filter | Delta |
|--------|---------------|---------------|-------|
| Portfolio P&L | $X | $X | +$X (−X%) |
| Portfolio Max DD | X% | X% | −Ypp |
| Portfolio WR% | X% | X% | +Ypp |
| Avg Open Positions | N | N | −K |
| Max Simultaneous | N | N | −K |
| Density Block Rate | 0% | X% | — |
| High-Density Event P&L | — | $X | — |

---

## Why This Could Fail

1. **Density doesn't predict direction:** Having 12 positions open doesn't mean the market will go against all of them. The filter blocks entries based on quantity, not quality.
2. **Opportunity cost:** Blocking entries during high-density periods means missing some of the best signals. If those blocked signals would have been winners, the filter costs more than it saves.
3. **Positions close over time:** By the time density is computed, some positions are already closing. The filter may be blocking entries after the crowdedness has peaked.

---

## Why It Could Succeed

1. **Crowded positions amplify drawdowns:** When all 18 coins are open and the market sells off, the drawdown is 18× a single position. Reducing to 10 concurrent positions materially lowers worst-case drawdown.
2. **Portfolio-level risk management:** This is the first RUN that treats COINCLAW as a portfolio, not a collection of independent coin traders. The whole is more fragile than the sum of its parts.
3. **Low implementation complexity:** One counter check in `check_entry`. No new data required.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN64 Density Filter |
|--|--|--|
| Entry gate | Signal quality only | Signal quality + portfolio density |
| Max concurrent positions | 18 | MAX_DENSITY × 18 |
| Avg open positions | ~8-12 | ~6-10 |
| Risk management | None | Portfolio crowdedness gate |
| Expected Max DD | X% | −20–35% |
| Expected P&L | $X | −5–10% (opportunity cost) |
| Implementation | — | check_entry only |
