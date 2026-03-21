# RUN379 — Market Breadth Momentum Divergence: Cross-Coin Momentum Imbalance

## Hypothesis

**Mechanism**: Track the average momentum (e.g., rate of change) across all 18 coins. When the average momentum is positive but breadth (percent of coins above their own SMA) is declining → divergence: the few strongest coins are bucking the trend while most coins are weakening. This breadth momentum divergence precedes market reversals. Use it to fade the move.

**Why not duplicate**: No prior RUN uses breadth momentum divergence. This specifically measures the DISAGREEMENT between breadth (how many coins are up) and average momentum (how much they're up by). When these diverge, it signals an unstable market state.

## Proposed Config Changes (config.rs)

```rust
// ── RUN379: Market Breadth Momentum Divergence ─────────────────────────────────
// avg_momentum = average(ROC(close, PERIOD)) across all coins
// breadth = % of coins where close > SMA(close, period)
// divergence = breadth and avg_momentum disagree
// LONG: breadth < BREADTH_THRESH AND avg_momentum < 0 (but breadth divergence indicates reversal)
// SHORT: breadth > (100 - BREADTH_THRESH) AND avg_momentum > 0

pub const BREADTH_DIV_ENABLED: bool = true;
pub const BREADTH_DIV_PERIOD: usize = 20;
pub const BREADTH_DIV_THRESH: f64 = 30.0;  // breadth below 30% = exhausted
pub const BREADTH_DIV_SL: f64 = 0.005;
pub const BREADTH_DIV_TP: f64 = 0.004;
pub const BREADTH_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run379_1_breadth_div_backtest.py)
2. **Walk-forward** (run379_2_breadth_div_wf.py)
3. **Combined** (run379_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- THRESH sweep: 20 / 30 / 40
