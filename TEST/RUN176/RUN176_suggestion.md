# RUN176 — Market-Wide Oversold Cluster: Breadth >80% + Per-Coin RSI Confirmation

## Hypothesis

**Mechanism**: COINCLAW already computes breadth (market-wide % of coins above SMA20). When breadth >80% (most coins in uptrend), LONG is dangerous. When breadth drops below 20%, LONG regime triggers. But when breadth is in the middle AND a cluster of coins have RSI < 25 simultaneously, it's a market-wide oversold cluster — a strong bounce signal. This RUN adds per-coin RSI clustering as a counter-signal to breadth.

**Why not duplicate**: No prior RUN uses breadth + per-coin RSI clustering. RUN43 uses breadth momentum velocity. This combines breadth threshold with per-coin RSI clustering — a different signal class.

## Proposed Config Changes (config.rs)

```rust
// ── RUN176: Market-Wide Oversold Cluster ────────────────────────────────
// Cluster: N or more coins have RSI < RSI_CLUSTER_THRESH simultaneously
// Cluster + breadth above BREADTH_CLUSTER → BOOST LONG entry by RISK_MULT

pub const CLUSTER_ENABLED: bool = true;
pub const CLUSTER_RSI_THRESH: f64 = 25.0;    // per-coin RSI threshold
pub const CLUSTER_MIN_COINS: usize = 5;       // minimum coins in cluster
pub const CLUSTER_BREADTH_MIN: f64 = 0.20;  // breadth must be above this
pub const CLUSTER_LONG_MULT: f64 = 1.5;      // multiply RISK by 1.5x during cluster
```

Add to `SharedState` in `state.rs`:

```rust
pub oversold_cluster_count: usize,  // number of coins with RSI < threshold
```

Add in `engine.rs`:

```rust
fn detect_oversold_cluster(state: &SharedState) -> usize {
    state.coins.iter()
        .filter(|cs| {
            cs.ind_15m.as_ref()
                .map(|ind| !ind.rsi.is_nan() && ind.rsi < config::CLUSTER_RSI_THRESH)
                .unwrap_or(false)
        })
        .count()
}

fn cluster_risk_mult(state: &SharedState) -> f64 {
    if !config::CLUSTER_ENABLED { return 1.0; }
    if state.oversold_cluster_count >= config::CLUSTER_MIN_COINS
        && state.breadth > config::CLUSTER_BREADTH_MIN
    {
        return config::CLUSTER_LONG_MULT;
    }
    1.0
}
```

---

## Validation Method

1. **Historical backtest** (run176_1_cluster_backtest.py)
2. **Walk-forward** (run176_2_cluster_wf.py)
3. **Combined** (run176_3_combined.py)

## Out-of-Sample Testing

- RSI_THRESH sweep: 20 / 25 / 30
- MIN_COINS sweep: 3 / 5 / 7
- LONG_MULT sweep: 1.25 / 1.5 / 2.0
