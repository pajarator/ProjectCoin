# RUN75 — Per-Coin Capital Reallocation: Dynamic Portfolio Rebalancing Based on Trailing Sharpe

## Hypothesis

**Named:** `sharpe_weighted_allocation`

**Mechanism:** COINCLAW currently allocates equal capital ($100) and equal risk fraction (10%) to each coin. But different coins have different quality — a coin with a 3-month Sharpe ratio of 1.5 is a better opportunity than one with 0.3. Capital should be allocated toward higher-quality performers and away from lower-quality ones.

**Dynamic reallocation rule:**
- Measure each coin's trailing Sharpe ratio over the last N trades (e.g., 20 trades)
- Compute weight: `w_i = sharpe_i / sum(sharpe_all)`
- Rebalance capital quarterly or monthly so that each coin's capital share matches its weight
- Cap minimum allocation at 50% of base ($50) and maximum at 200% of base ($200)

**Why this is not a duplicate:**
- No prior RUN has reallocated capital based on trailing performance quality
- RUN19 (Kelly) sized based on aggregate historical stats — this sizes based on per-coin trailing Sharpe
- RUN52 (z-confidence sizing) sized based on current signal quality — this sizes based on historical strategy quality
- This is portfolio-level capital reallocation, fundamentally different from per-trade position sizing

---

## Proposed Config Changes

```rust
// RUN75: Sharpe-Weighted Capital Allocation
pub const SHARPE_ALLOC_ENABLE: bool = true;
pub const SHARPE_ALLOC_WINDOW: u32 = 20;  // number of trades to compute trailing Sharpe
pub const SHARPE_ALLOC_FREQ: u32 = 168;  // rebalance every N bars (~1 week)
pub const SHARPE_MIN_CAP: f64 = 0.50;   // minimum 50% of base capital
pub const SHARPE_MAX_CAP: f64 = 2.00;   // maximum 200% of base capital
```

**`state.rs` — CoinPersist additions:**
```rust
pub struct CoinPersist {
    // ... existing fields ...
    pub trailing_sharpe: f64,      // trailing Sharpe over SHARPE_ALLOC_WINDOW trades
    pub current_allocation: f64,    // current capital allocation (multiple of base)
    pub last_rebalance_bar: u32,   // bar of last rebalance
}
```

**`engine.rs` — rebalance logic:**
```rust
pub fn rebalance_by_sharpe(state: &mut SharedState, current_bar: u32) {
    if !config::SHARPE_ALLOC_ENABLE { return; }
    if current_bar - state.last_rebalance_bar < config::SHARPE_ALLOC_FREQ { return; }

    let mut sharpes: Vec<f64> = Vec::new();
    for cs in &state.coins {
        sharpes.push(compute_trailing_sharpe(&cs.trades, config::SHARPE_ALLOC_WINDOW));
    }

    let sum_sharpe: f64 = sharpes.iter().sum();
    if sum_sharpe <= 0.0 { return; }  // avoid division by zero

    let base_cap = config::INITIAL_CAPITAL;
    let mut new_allocations: Vec<f64> = Vec::new();

    for (i, &sh) in sharpes.iter().enumerate() {
        let weight = sh / sum_sharpe;
        let total_portfolio = state.coins.iter().map(|c| c.bal).sum::<f64>();
        let target_cap = total_portfolio * weight;
        let capped = target_cap.clamp(base_cap * config::SHARPE_MIN_CAP,
                                     base_cap * config::SHARPE_MAX_CAP);
        new_allocations.push(capped);
    }

    // Apply reallocation: withdraw from over-allocated, deposit to under-allocated
    for (i, cs) in state.coins.iter_mut().enumerate() {
        let diff = new_allocations[i] - cs.bal;
        cs.bal += diff;  // simple: just move the balance
    }

    state.last_rebalance_bar = current_bar;
}
```

---

## Validation Method

### RUN75.1 — Sharpe Allocation Backtest (Rust, portfolio-level)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — equal $100 capital per coin

**Rebalancing frequency grid:**

| Parameter | Values |
|-----------|--------|
| `SHARPE_ALLOC_FREQ` | [84=daily, 168=weekly, 336=bimonthly] |
| `SHARPE_ALLOC_WINDOW` | [10, 20, 30] |
| `SHARPE_MIN_CAP` | [0.25, 0.50] |
| `SHARPE_MAX_CAP` | [1.5, 2.0, 3.0] |

**Key metrics:**
- `allocation_spread`: max allocation / min allocation at each rebalance
- `sharpe_rebalance_correctness`: % of rebalances that move capital toward the better performer
- `portfolio_sharpe_delta`: Sharpe ratio change vs baseline
- `portfolio_PnL_delta`: P&L change vs baseline
- `concentration_risk`: max allocation to single coin

### RUN75.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: measure trailing Sharpe stability across windows
2. Test: evaluate rebalanced portfolio on held-out month

**Pass criteria:**
- Sharpe-allocated portfolio Sharpe ≥ equal-weight baseline Sharpe
- Max allocation stays within SHARPE_MAX_CAP

### RUN75.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (equal weight) | Sharpe-Allocated | Delta |
|--------|------------------------|------------------|-------|
| Portfolio Sharpe | X.XX | X.XX | +0.XX |
| Portfolio P&L | $X | $X | +$X |
| Max Allocation | $100 | $X | +$Y |
| Min Allocation | $100 | $X | −$Y |
| Avg Allocation Spread | 0% | X% | — |
| Correct Rebalances | — | X% | — |

---

## Why This Could Fail

1. **Trailing Sharpe is backward-looking:** By the time a coin's Sharpe improves enough to get more capital, the good performance may be over. Capital follows yesterday's winners.
2. **Small sample at coin level:** 20 trades per coin may not be enough for a stable Sharpe estimate.
3. **Concentration risk:** If one coin dominates Sharpe, it gets 200% allocation while others get 50%. This concentrates risk in a single coin.

---

## Why It Could Succeed

1. **Capital finds quality:** This is the core promise of momentum/quality allocation: more capital to better strategies, less to worse.
2. **Simple and intuitive:** No new indicators or signals — just reallocates existing capital based on observed quality.
3. **Institutional practice:** Risk parity and quality-weighted portfolios are standard in institutional investing.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN75 Sharpe-Weighted Allocation |
|--|--|--|
| Capital allocation | Equal ($100/coin) | Sharpe-weighted |
| Rebalancing | Never | Weekly/monthly |
| Max allocation | $100 | Up to $200 |
| Min allocation | $100 | Down to $50 |
| Trailing lookback | None | 20 trades |
