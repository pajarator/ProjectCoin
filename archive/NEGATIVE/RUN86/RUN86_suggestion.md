# RUN86 — Coin Correlation Clustering: Suppress Correlated Coin Entries to Reduce Concentration Risk

## Hypothesis

**Named:** `correlation_cluster_suppress`

**Mechanism:** COINCLAW currently treats all 18 coins independently — when BTC is oversold and many alts are also oversold, all coins fire their LONG entries simultaneously. But coins are highly correlated — when BTC crashes, most alts follow. This creates correlated drawdowns where the portfolio loses on many coins at once. The fix: when N+ coins in a correlation cluster are all signaling the same direction, suppress the weaker ones (lower recent Sharpe) and let only the best one trade.

**Coin Correlation Clustering:**
- Compute rolling 20-bar return correlation matrix across all 18 coins
- Identify correlation clusters: coins with pairwise correlation > `CLUSTER_THRESHOLD` (e.g., 0.70) form a cluster
- When 3+ coins in the same cluster are simultaneously signaling LONG or SHORT:
  - Rank by trailing Sharpe (last 20 trades)
  - Allow only the top `CLUSTER_MAX_COINS` (e.g., 2) coins in the cluster to trade
  - Suppress the rest — `COOLDOWN = cluster_suppress_cooldown` bars
- Suppressed coins are not allowed to enter until the cluster signal count drops below threshold

**Why this is not a duplicate:**
- RUN49 (corr suppress threshold) uses rolling return correlation to suppress same-direction signals — but RUN49 was a same-direction consecutive signal suppressor (not an inter-coin clustering suppressor)
- RUN64 (position density filter) uses portfolio concentration — this uses return correlation specifically, not position count
- No prior RUN has used cross-coin return correlation clustering to filter which coins get to trade when multiple correlated coins signal simultaneously

**Mechanistic rationale:** When ETH and SOL are both at z = -2.0 and both signaling LONG, they are likely to move together. Taking both positions exposes the portfolio to a single drawdown event counted twice. Correlation clustering reduces this by picking only the highest-quality signal within a correlated group.

---

## Proposed Config Changes

```rust
// RUN86: Coin Correlation Clustering
pub const CORR_CLUSTER_ENABLE: bool = true;
pub const CORR_CLUSTER_THRESHOLD: f64 = 0.70;   // pairwise return correlation threshold
pub const CORR_CLUSTER_MIN_SIZE: u32 = 3;       // min coins in cluster to activate suppression
pub const CORR_CLUSTER_MAX_COINS: u32 = 2;      // max coins allowed to trade within cluster
pub const CORR_CLUSTER_LOOKBACK: usize = 20;    // bars for rolling correlation
pub const CORR_SUPPRESS_COOLDOWN: u32 = 8;      // bars to suppress suppressed coins
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub suppress_until_bar: u32,   // if set, no entries allowed until this bar
}
```

**`engine.rs` — find_correlation_clusters and apply suppression:**
```rust
/// Compute rolling return correlation matrix for all coins.
fn compute_corr_matrix(state: &SharedState, lookback: usize) -> Vec<Vec<f64>> {
    let n = state.coins.len();
    let mut returns: Vec<Vec<f64>> = vec![vec![]; n];

    // Collect 20-bar returns for each coin
    for (i, cs) in state.coins.iter().enumerate() {
        if cs.candles_15m.len() < lookback {
            continue;
        }
        let bars = &cs.candles_15m[cs.candles_15m.len() - lookback..];
        let mut rets = Vec::with_capacity(bars.len() - 1);
        for w in bars.windows(2) {
            rets.push((w[1].c - w[0].c) / w[0].c);
        }
        returns[i] = rets;
    }

    // Compute correlation matrix
    let mut corr = vec![vec![0.0; n]; n];
    for i in 0..n {
        corr[i][i] = 1.0;
        for j in (i+1)..n {
            let r = pearson_corr(&returns[i], &returns[j]);
            corr[i][j] = r;
            corr[j][i] = r;
        }
    }
    corr
}

fn pearson_corr(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 5 { return 0.0; }
    let n = a.len() as f64;
    let (sum_a, sum_b, sum_ab, sum_a2, sum_b2) = a.iter().zip(b.iter())
        .fold((0.0, 0.0, 0.0, 0.0, 0.0), |(sa,sb,sab,sa2,sb2), (x,y)| {
            (sa+x, sb+y, sab+x*y, sa2+x*x, sb2+y*y)
        });
    let num = n*sum_ab - sum_a*sum_b;
    let den = ((n*sum_a2-sum_a*sum_a)*(n*sum_b2-sum_b*sum_b)).sqrt();
    if den == 0.0 { 0.0 } else { num/den }
}

/// Find clusters of coins with pairwise correlation above threshold.
fn find_clusters(corr: &[Vec<f64>], names: &[&str], threshold: f64) -> Vec<Vec<usize>> {
    let n = corr.len();
    let mut visited = vec![false; n];
    let mut clusters = Vec::new();

    for i in 0..n {
        if visited[i] { continue; }
        let mut cluster = vec![i];
        visited[i] = true;
        for j in (i+1)..n {
            if !visited[j] && corr[i][j] > threshold {
                cluster.push(j);
                visited[j] = true;
            }
        }
        if cluster.len() >= 3 {
            clusters.push(cluster);
        }
    }
    clusters
}

/// In the coordinator tick — suppress correlated cluster entries:
pub fn apply_correlation_suppression(state: &mut SharedState, current_bar: u32) {
    if !config::CORR_CLUSTER_ENABLE { return; }

    let corr = compute_corr_matrix(state, config::CORR_CLUSTER_LOOKBACK);
    let names: Vec<&str> = state.coins.iter().map(|c| c.name).collect();
    let clusters = find_clusters(&corr, &names, config::CORR_CLUSTER_THRESHOLD);

    for cluster in clusters {
        // Count how many coins in cluster are currently signaling
        let mut signaling = Vec::new();
        for &ci in &cluster {
            let cs = &state.coins[ci];
            if cs.pos.is_none() && cs.cooldown == 0 && cs.suppress_until_bar <= current_bar {
                // Coin has a valid signal — check if it's the direction we're counting
                // For simplicity: all non-position, non-cooldown coins in cluster count as signaling
                signaling.push((ci, trailing_sharpe(&state.coins[ci].trades, 20)));
            }
        }

        if signaling.len() >= config::CORR_CLUSTER_MIN_SIZE as usize {
            // Sort by trailing Sharpe descending — keep top N
            signaling.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let suppressed = &signaling[config::CORR_CLUSTER_MAX_COINS as usize..];
            for &(ci, _) in suppressed {
                state.coins[ci].suppress_until_bar = current_bar + config::CORR_SUPPRESS_COOLDOWN;
            }
        }
    }
}

// In check_entry — add suppression check:
if state.coins[ci].suppress_until_bar > current_bar {
    return;  // suppressed due to correlation cluster
}
```

---

## Validation Method

### RUN86.1 — Correlation Clustering Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no correlation suppression

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CORR_CLUSTER_THRESHOLD` | [0.60, 0.70, 0.80] |
| `CORR_CLUSTER_MIN_SIZE` | [3, 4] |
| `CORR_CLUSTER_MAX_COINS` | [1, 2] |
| `CORR_SUPPRESS_COOLDOWN` | [4, 8, 12] |

**Note:** This is a portfolio-level optimization. Grid search runs full backtests across all coins simultaneously (not per-coin). Total configs: 3 × 2 × 2 × 3 = 36.

**Key metrics:**
- `cluster_activation_rate`: % of bars where at least one cluster suppresses entries
- `suppressed_entry_rate`: % of entries suppressed by cluster logic
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN86.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold/min_size/max_coins per window
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio P&L delta ≥ 0 vs baseline
- Max drawdown reduced vs baseline
- Cluster activation rate 5–25% (meaningful use)

### RUN86.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no corr filter) | Correlation Clustering | Delta |
|--------|------------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Cluster Activations | 0 | X | — |
| Entries Suppressed | 0 | X | — |
| Avg Simultaneous LONG | X | X | -N |
| Avg Simultaneous SHORT | X | X | -N |

---

## Why This Could Fail

1. **Correlation is not stable over time:** Coins that were correlated in training period may decorrelate in the test period. The clustering assignment is backward-looking and may not hold.
2. **Suppressing entries reduces opportunity:** When a cluster fires and 3+ coins are suppressed, we may be suppressing valid setups that would have won. The opportunity cost of suppression may exceed the diversification benefit.
3. **Computation cost:** Computing a full 18×18 correlation matrix each bar is O(n²) — not prohibitive but non-trivial for a live trader.

---

## Why It Could Succeed

1. **Crypto correlation spikes in drawdowns:** BTC crashes tend to drag down all alts simultaneously. During these periods, having 6 simultaneous LONG positions means 6 correlated losses. Clustering reduces this correlated exposure.
2. **Picking best-in-cluster is sound:** When 4 coins are all signaling LONG, the one with the best recent Sharpe is most likely to succeed. Suppressing the others in favor of the best one is a principled filter.
3. **Institutional practice:** Correlation-adjusted position sizing is standard in systematic portfolio management. This is a discrete version of that idea (suppress vs. size differently).
4. **Complementary to breadth:** Breadth tells us market-wide conditions; correlation tells us which coins are moving together. Both reduce portfolio-level concentration.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN86 Correlation Clustering |
|--|--|--|
| Correlated entries | All allowed simultaneously | Top Sharpe in cluster allowed, others suppressed |
| Correlation awareness | None | Rolling 20-bar return correlation |
| Cluster size trigger | N/A | 3+ coins above 0.70 correlation |
| Max per cluster | Unlimited | 2 coins per cluster |
| Suppression cooldown | N/A | 8 bars |
| Drawdown correlation | Full exposure | Reduced simultaneous exposure |
