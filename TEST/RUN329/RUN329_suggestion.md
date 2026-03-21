# RUN329 — Price Volume Rank Correlation: Cross-Asset Momentum Scoring

## Hypothesis

**Mechanism**: Compute a composite score for each coin combining price momentum rank and volume momentum rank (each ranked 0-100 across all 18 coins). The composite score = (price_rank × weight) + (volume_rank × (1-weight)). When composite score crosses above its 20-bar SMA → LONG (accumulation phase). When composite crosses below → SHORT (distribution phase). Cross-coin ranking captures relative strength.

**Why not duplicate**: No prior RUN uses cross-coin ranking of combined price AND volume momentum. RUN251 uses correlation between price and volume. RUN275 uses PVT rank. This RUN specifically combines price momentum rank and volume momentum rank into a single composite score using relative ranking across all coins.

## Proposed Config Changes (config.rs)

```rust
// ── RUN329: Price Volume Rank Correlation ─────────────────────────────────────
// price_ret_rank = percentile_rank(close_ret_20, all_coins)
// vol_ret_rank = percentile_rank(volume_ret_20, all_coins)
// composite = price_ret_rank * WEIGHT + vol_ret_rank * (1 - WEIGHT)
// composite_ma = SMA(composite, period)
// LONG: composite crosses above composite_ma
// SHORT: composite crosses below composite_ma

pub const PV_RANK_ENABLED: bool = true;
pub const PV_RANK_RET_PERIOD: usize = 20;    // return lookback for ranking
pub const PV_RANK_MA_PERIOD: usize = 20;    // SMA of composite score
pub const PV_RANK_PRICE_WEIGHT: f64 = 0.7;   // weight for price rank (vol = 0.3)
pub const PV_RANK_SL: f64 = 0.005;
pub const PV_RANK_TP: f64 = 0.004;
pub const PV_RANK_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run329_1_pv_rank_backtest.py)
2. **Walk-forward** (run329_2_pv_rank_wf.py)
3. **Combined** (run329_3_combined.py)

## Out-of-Sample Testing

- RET_PERIOD sweep: 10 / 20 / 40
- MA_PERIOD sweep: 14 / 20 / 30
- PRICE_WEIGHT sweep: 0.5 / 0.7 / 0.8
