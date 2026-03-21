# RUN183 — Cointegration Pair Trade: BTC/ETH Spread Mean-Reversion

## Hypothesis

**Mechanism**: BTC and ETH are cointegrated — their ratio (BTC/ETH) mean-reverts to a long-term average. When the ratio deviates >2 standard deviations from its 100-bar z-score, it's an extreme — the spread will contract. SHORT the ratio (short BTC, long ETH equivalent) when ratio is high; LONG the ratio when ratio is low.

**Why not duplicate**: No prior RUN uses cointegration or pair trading between BTC and ETH.

## Proposed Config Changes (config.rs)

```rust
// ── RUN183: Cointegration Pair Trade ────────────────────────────────────
// ratio = BTC_price / ETH_price
// ratio > z-score threshold → SHORT ratio (expect contraction → short BTC/long ETH)
// ratio < -z-score threshold → LONG ratio (expect expansion → long BTC/short ETH)

pub const PAIR_ENABLED: bool = true;
pub const PAIR_Z_THRESH: f64 = 2.0;     // 2 std devs = extreme
pub const PAIR_WINDOW: usize = 100;        // rolling window for ratio mean/std
pub const PAIR_SL: f64 = 0.005;
pub const PAIR_TP: f64 = 0.004;
pub const PAIR_MAX_HOLD: u32 = 48;          // ~12 hours at 15m
```

Add to `CoinState` in `state.rs`:

```rust
pub btc_eth_ratio: f64,
pub btc_eth_ratio_history: Vec<f64>,
```

---

## Validation Method

1. **Historical backtest** (run183_1_pair_backtest.py)
2. **Walk-forward** (run183_2_pair_wf.py)
3. **Combined** (run183_3_combined.py)

## Out-of-Sample Testing

- Z_THRESH sweep: 1.5 / 2.0 / 2.5
- WINDOW sweep: 50 / 100 / 200 bars
