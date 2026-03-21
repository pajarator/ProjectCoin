# RUN275 — Price Volume Trend Rank: Composite Momentum Signal

## Hypothesis

**Mechanism**: PVT = Price × Volume Trend = cumulative volume × percentage price change. Rank PVT across all coins. When a coin's PVT rank rises into the top 3 → strong relative accumulation → LONG. When PVT rank falls to bottom 3 → strong relative distribution → SHORT.

**Why not duplicate**: No prior RUN uses PVT rank. All prior volume RUNs trade single coins in isolation. PVT rank is a *relative* measure across all coins, identifying which coins are attracting the most volume-weighted price movement.

## Proposed Config Changes (config.rs)

```rust
// ── RUN275: Price Volume Trend Rank ───────────────────────────────────────
// pvt = prior_pvt + volume × (close - prior_close) / prior_close
// rank all 18 coins by PVT ascending/descending
// LONG: coin's PVT rank in top 3 AND PVT rising
// SHORT: coin's PVT rank in bottom 3 AND PVT falling

pub const PVT_RANK_ENABLED: bool = true;
pub const PVT_RANK_PERIOD: usize = 20;       // PVT smoothing period
pub const PVT_RANK_TOP_N: usize = 3;         // coins to long
pub const PVT_RANK_BOT_N: usize = 3;         // coins to short
pub const PVT_RANK_SL: f64 = 0.005;
pub const PVT_RANK_TP: f64 = 0.004;
pub const PVT_RANK_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run275_1_pvt_rank_backtest.py)
2. **Walk-forward** (run275_2_pvt_rank_wf.py)
3. **Combined** (run275_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- TOP_N sweep: 2 / 3 / 5
- BOT_N sweep: 2 / 3 / 5
