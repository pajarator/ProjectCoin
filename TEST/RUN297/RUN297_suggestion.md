# RUN297 — Volatility-Adjusted Momentum Rank: Multi-Window ATR Normalization

## Hypothesis

**Mechanism**: Raw returns are misleading across coins with different volatilities. Normalize returns by ATR so that a 2% move in a low-vol coin is comparable to a 2% move in a high-vol coin. Compute normalized momentum across multiple windows (4h, 1d, 3d), rank all 18 coins by composite score. Long top-3 rank (strongest risk-adjusted momentum), Short bottom-3 rank (weakest risk-adjusted momentum). Rebalance every N bars.

**Why not duplicate**: No prior RUN uses ATR-normalized momentum ranking across coins. RUN28 classifies coins as momentum vs mean-reversion but doesn't rank them. RUN238/268 use directional rotation but not volatility-adjusted. ATR normalization is the key distinction — it makes momentum comparable across coins of different volatility profiles.

## Proposed Config Changes (config.rs)

```rust
// ── RUN297: Volatility-Adjusted Momentum Rank ───────────────────────────────
// norm_ret(window) = (close - close[N]) / ATR(14)   // ATR-normalized return
// composite = avg(norm_ret_4h, norm_ret_1d, norm_ret_3d)
// rank all 18 coins by composite descending
// LONG: coin rank in top 3
// SHORT: coin rank in bottom 3
// rebalance every REBALANCE_BARS

pub const MOM_RANK_ENABLED: bool = true;
pub const MOM_RANK_REBALANCE: u32 = 16;    // rebalance every 16 x 15m = 4h
pub const MOM_RANK_LONG_N: usize = 3;       // long top 3
pub const MOM_RANK_SHORT_N: usize = 3;     // short bottom 3
pub const MOM_RANK_WIN1: usize = 4;         // 4-bar (1h) window
pub const MOM_RANK_WIN2: usize = 16;        // 16-bar (4h) window
pub const MOM_RANK_WIN3: usize = 96;        // 96-bar (24h) window
pub const MOM_RANK_SL: f64 = 0.005;
pub const MOM_RANK_TP: f64 = 0.004;
pub const MOM_RANK_MAX_HOLD: u32 = 16;     // 4h max (aligns with rebalance)
```

---

## Validation Method

1. **Historical backtest** (run297_1_mom_rank_backtest.py)
2. **Walk-forward** (run297_2_mom_rank_wf.py)
3. **Combined** (run297_3_combined.py)

## Out-of-Sample Testing

- REBALANCE sweep: 8 / 16 / 32
- LONG_N sweep: 2 / 3 / 5
- SHORT_N sweep: 2 / 3 / 5
