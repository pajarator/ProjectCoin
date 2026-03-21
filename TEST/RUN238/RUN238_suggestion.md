# RUN238 — Momentum Rotation: Top-N Coin Rotation Based on Relative Strength

## Hypothesis

**Mechanism**: At each rebalance interval (e.g., every 4 bars = 1 hour), rank all 18 coins by their 20-bar momentum (ROC). Rotate INTO the top 3 momentum coins, rotating OUT of the bottom 3. This captures relative strength — always being in the coins going up the most. The rotation acts as a dynamic stop: when a coin falls out of the top 3, you exit.

**Why not duplicate**: No prior RUN uses momentum rotation across coins. All prior RUNs trade single coins in isolation. This is a portfolio-level strategy that uses relative momentum ranking to continuously be in the best-performing coins.

## Proposed Config Changes (config.rs)

```rust
// ── RUN238: Momentum Rotation Strategy ───────────────────────────────────
// rotation_period = 4 bars (1 hour at 15m)
// momentum[coin] = ROC(close, 20)
// top_3 = coins with highest momentum
// bottom_3 = coins with lowest momentum
// LONG top_3, SKIP bottom_3
// Rebalance at each rotation_period

pub const MOM_ROT_ENABLED: bool = true;
pub const MOM_ROT_PERIOD: usize = 4;         // rebalance frequency in 15m bars
pub const MOM_ROT_MOM_PERIOD: usize = 20;     // momentum lookback
pub const MOM_ROT_TOP_N: usize = 3;           // number of coins to hold
pub const MOM_ROT_SL: f64 = 0.005;
pub const MOM_ROT_TP: f64 = 0.004;
```

Modify engine to implement the rotation logic at each rebalance period.

---

## Validation Method

1. **Historical backtest** (run238_1_mom_rot_backtest.py)
2. **Walk-forward** (run238_2_mom_rot_wf.py)
3. **Combined** (run238_3_combined.py)

## Out-of-Sample Testing

- ROTATION_PERIOD sweep: 2 / 4 / 8
- MOM_PERIOD sweep: 10 / 20 / 40
- TOP_N sweep: 2 / 3 / 5
