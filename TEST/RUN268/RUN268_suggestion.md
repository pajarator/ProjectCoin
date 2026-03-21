# RUN268 — Momentum Rotation Rank: Top/Bottom N Coins by ROC

## Hypothesis

**Mechanism**: At each rebalance, rank all 18 coins by their N-bar Rate of Change. Go LONG the top 3 coins, SHORT the bottom 3. Rebalance every N bars. This is a relative momentum strategy — always long the strongest, short the weakest. The rotation provides natural stop-loss (exit when coin falls out of top/bottom).

**Why not duplicate**: RUN238 uses momentum rotation but with different parameters. This RUN specifically focuses on BOTH long AND short rotation (long top 3, short bottom 3) simultaneously for a market-neutral approach.

## Proposed Config Changes (config.rs)

```rust
// ── RUN268: Momentum Rotation Rank (Long/Short) ─────────────────────────
// roc[coin] = ROC(close, period)
// rank coins by ROC
// LONG top 3 coins by ROC
// SHORT bottom 3 coins by ROC
// rebalance every N bars

pub const MOM_ROT_LS_ENABLED: bool = true;
pub const MOM_ROT_LS_PERIOD: usize = 20;     // ROC lookback
pub const MOM_ROT_LS_BARS: usize = 8;       // rebalance frequency
pub const MOM_ROT_LS_TOP_N: usize = 3;     // coins to long
pub const MOM_ROT_LS_BOT_N: usize = 3;     // coins to short
pub const MOM_ROT_LS_SL: f64 = 0.005;
pub const MOM_ROT_LS_TP: f64 = 0.004;
```

---

## Validation Method

1. **Historical backtest** (run268_1_mom_rot_ls_backtest.py)
2. **Walk-forward** (run268_2_mom_rot_ls_wf.py)
3. **Combined** (run268_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 40
- BARS sweep: 4 / 8 / 16
- TOP_N sweep: 2 / 3 / 5
- BOT_N sweep: 2 / 3 / 5
