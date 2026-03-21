# RUN93 — Consecutive Wins Streak Boost: Increase Position Size After Winning Streaks

## Hypothesis

**Named:** `streak_risk_boost`

**Mechanism:** COINCLAW currently uses a fixed `RISK = 10%` for all regime trades regardless of recent performance. But after a streak of consecutive winning trades, the market conditions may be favorable for the strategy — more trades fire, more win. After losing streaks, conditions may be unfavorable. The Consecutive Wins Streak Boost increases RISK during favorable periods and decreases RISK during unfavorable periods.

**Consecutive Wins Streak Boost:**
- Track `consecutive_wins` and `consecutive_losses` per coin
- After `STREAK_BOOST_THRESHOLD` consecutive wins (e.g., 3), increase RISK by `STREAK_RISK_MULT` (e.g., 1.3×) for the next trade
- After `STREAK_LOSS_THRESHOLD` consecutive losses (e.g., 3), decrease RISK by `STREAK_LOSS_MULT` (e.g., 0.7×) for the next trade
- Reset counters after any change in streak direction
- Cap RISK at [5%, 15%]
- Momentum trades exempt (fixed risk)

**Why this is not a duplicate:**
- RUN19 (Kelly sizing) used aggregate historical stats for sizing — this uses recent consecutive win/loss streaks
- RUN69 (streak TP discount) took profits after winning streaks — this scales RISK
- RUN51 (DD SL widening) sized positions by per-coin drawdown — this sizes by recent win/loss streak count, not drawdown magnitude
- No prior RUN has scaled RISK based on consecutive win/loss streaks

**Mechanistic rationale:** Winning streaks often mean the market is in a favorable regime for the strategy. Increasing RISK during these periods amplifies profits when the edge is largest. Conversely, losing streaks often mean the market has shifted to an unfavorable regime — decreasing RISK preserves capital. This is a simple momentum-like adjustment to position sizing.

---

## Proposed Config Changes

```rust
// RUN93: Consecutive Wins Streak Boost
pub const STREAK_BOOST_ENABLE: bool = true;
pub const STREAK_WIN_THRESHOLD: u32 = 3;       // consecutive wins to activate boost
pub const STREAK_LOSS_THRESHOLD: u32 = 3;      // consecutive losses to activate reduction
pub const STREAK_RISK_MULT: f64 = 1.30;        // multiply risk after winning streak
pub const STREAK_LOSS_MULT: f64 = 0.70;       // multiply risk after losing streak
pub const STREAK_RISK_MIN: f64 = 0.05;         // minimum 5% risk
pub const STREAK_RISK_MAX: f64 = 0.15;         // maximum 15% risk
```

**`state.rs` — CoinPersist additions:**
```rust
pub struct CoinPersist {
    // ... existing fields ...
    pub consecutive_wins: u32,
    pub consecutive_losses: u32,
}
```

**`engine.rs` — compute_streak_risk in open_position:**
```rust
/// Compute effective risk based on current win/loss streak.
fn compute_streak_risk(cs: &CoinState) -> f64 {
    if !config::STREAK_BOOST_ENABLE { return config::RISK; }

    let base = config::RISK;
    if cs.consecutive_wins >= config::STREAK_WIN_THRESHOLD {
        return (base * config::STREAK_RISK_MULT).clamp(
            config::STREAK_RISK_MIN, config::STREAK_RISK_MAX);
    }
    if cs.consecutive_losses >= config::STREAK_LOSS_THRESHOLD {
        return (base * config::STREAK_LOSS_MULT).clamp(
            config::STREAK_RISK_MIN, config::STREAK_RISK_MAX);
    }
    base
}

// In open_position — use compute_streak_risk for regime trades:
let risk = match trade_type {
    TradeType::Regime => compute_streak_risk(cs),
    TradeType::Scalp => config::SCALP_RISK,
    TradeType::Momentum => config::RISK,
};
let trade_amt = cs.bal * risk;

// In close_position — update streak counters:
if pnl > 0.0 {
    cs.consecutive_wins += 1;
    cs.consecutive_losses = 0;
} else {
    cs.consecutive_losses += 1;
    cs.consecutive_wins = 0;
}
```

---

## Validation Method

### RUN93.1 — Streak Boost Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed RISK = 10%

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `STREAK_WIN_THRESHOLD` | [2, 3, 4] |
| `STREAK_LOSS_THRESHOLD` | [2, 3, 4] |
| `STREAK_RISK_MULT` | [1.2, 1.3, 1.5] |
| `STREAK_LOSS_MULT` | [0.6, 0.7, 0.8] |

**Per coin:** 3 × 3 × 3 × 3 = 81 configs × 18 coins = 1,458 backtests

**Key metrics:**
- `streak_boost_rate`: % of trades where risk was boosted
- `streak_reduce_rate`: % of trades where risk was reduced
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN93.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best streak thresholds per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Max drawdown does not increase >20% vs baseline
- Streak boost activates at least 5% of trades

### RUN93.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 10%) | Streak Boost | Delta |
|--------|--------------------------|-------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Effective Risk | 10% | X% | +/-Xpp |
| Streak Boost Rate | 0% | X% | — |
| Streak Reduce Rate | 0% | X% | — |
| Post-Streak Win Rate | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Streaks are often random:** In a random walk market, consecutive wins/losses are memoryless — past streaks don't predict future performance. The boost may be applied at exactly the wrong time (just before the streak ends).
2. **Risk increases during winning streaks magnify drawdowns:** If a winning streak ends and the next trade loses, the larger position size means a larger loss. The boost could amplify the very drawdowns it's trying to avoid.
3. **Compounding effect of large positions in losing periods:** Using 1.3× risk after a winning streak, then hitting a 3-trade losing streak with larger positions, could be catastrophic.

---

## Why It Could Succeed

1. **Market regimes cluster:** In a favorable mean-reversion regime, multiple consecutive wins are common. Boosting during this period amplifies profits when the edge is largest.
2. **Preserves capital in unfavorable regimes:** Reducing risk after losing streaks reduces exposure when the strategy is underperforming — the market has likely shifted.
3. **Intuitive and simple:** Consecutive win/loss streaks are easy to track and understand. The logic is clear and interpretable.
4. **Psychological anchor:** Using 10% base risk with ±30% adjustment is a bounded, reasonable range. No extreme positions.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN93 Streak Boost |
|--|--|--|
| RISK after 3 consecutive wins | 10% | 13% |
| RISK after 3 consecutive losses | 10% | 7% |
| RISK range | 10% (fixed) | 5–15% (streak-adaptive) |
| Win streak awareness | None | Increases size |
| Loss streak awareness | None | Decreases size |
| Drawdown protection | None | Partial |
