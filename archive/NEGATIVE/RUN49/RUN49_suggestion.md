# RUN49 — Cross-Coin Correlation Filter: Avoiding Clustered Over-Concentration

## Hypothesis

**Named:** `correlation_cluster_filter`

**Mechanism:** COINCLAW currently treats each coin independently — when BTC pumps and 8 altcoins simultaneously show oversold conditions, all 8 can generate LONG entries at the same time. These 8 altcoins are likely highly correlated with each other (and with BTC), meaning they will all move in the same direction. Opening 8 correlated positions is effectively one large concentrated bet, not 8 independent ones.

The problem: When the market reverses, all 8 correlated positions get stopped out simultaneously — creating a clustered drawdown that looks like a single catastrophic loss.

The fix: Maintain a rolling correlation matrix of 15-bar returns across all 18 coins. When multiple coins generate entry signals simultaneously, select only the most attractive candidate (e.g., lowest z-score for LONG) and suppress the others. This reduces correlated drawdowns without reducing total expected value.

**Correlation filter logic:**
```
rolling_window = 15 bars (~6 hours)
for each coin pair (i, j):
    corr[i,j] = correlation(returns[i,-15:], returns[j,-15:])

At signal time:
    if coin A and coin B both generate signals AND corr[A,B] > CORR_THRESHOLD:
        # Select the one with better signal quality (lower z for LONG, higher z for SHORT)
        # Suppress the other
        suppress the coin with weaker signal
```

**Why this is not a duplicate:**
- No prior RUN has used cross-coin return correlation as a filter
- RUN24 (ensemble) used strategy correlation, not price return correlation
- No prior RUN tested position concentration from correlated signals
- Correlation is a market-structure signal, fundamentally different from price-based filters

---

## Proposed Config Changes

```rust
// RUN49: Cross-Coin Correlation Filter
pub const CORR_WINDOW_BARS: u32 = 15;            // rolling window for correlation
pub const CORR_SUPPRESS_THRESHOLD: f64 = 0.75;   // suppress if corr > 0.75 with a signaled coin
pub const CORR_FILTER_MODE: u8 = 1;              // 0=disabled, 1=suppress_weakest, 2=random_drop
pub const CORR_MAX_SAME_DIRECTION: u32 = 4;       // max N coins with same direction signals at once
```

**`engine.rs` change — `check_entry` coordinated across all coins:**

The correlation filter requires seeing all coins' signals before deciding which to suppress. This needs a two-pass approach:
1. **Pass 1:** Collect all coins with valid entry signals and their signal quality scores
2. **Pass 2:** Build correlation clusters, suppress weakest signals in high-corr clusters

```rust
// In coordinator.rs or engine.rs
pub struct SignalCandidate {
    pub ci: usize,
    pub dir: Direction,
    pub signal_quality: f64,  // lower z for longs, higher z for shorts
    pub strat: String,
}

// Two-pass entry check
pub fn check_entry_with_correlation_filter(state: &mut SharedState) {
    // PASS 1: collect all candidates
    let mut candidates: Vec<SignalCandidate> = Vec::new();
    for ci in 0..state.coins.len() {
        if let Some((dir, strat)) = get_entry_candidate(state, ci) {
            let z = state.coins[ci].ind_15m.as_ref().map(|i| i.z).unwrap_or(0.0);
            let quality = match dir {
                Direction::Long => -z,  // lower z = better for longs
                Direction::Short => z,   // higher z = better for shorts
            };
            candidates.push(SignalCandidate { ci, dir, signal_quality: quality, strat });
        }
    }

    // PASS 2: correlation filter
    let mut suppressed = vec![false; state.coins.len()];
    if config::CORR_FILTER_MODE > 0 && candidates.len() > 1 {
        let corr_matrix = compute_correlation_matrix(state);
        for i in 0..candidates.len() {
            for j in (i+1)..candidates.len() {
                if candidates[i].dir == candidates[j].dir {  // same direction only
                    let cij = corr_matrix[candidates[i].ci][candidates[j].ci];
                    if cij > config::CORR_SUPPRESS_THRESHOLD {
                        // Suppress the weaker signal
                        if candidates[i].signal_quality < candidates[j].signal_quality {
                            suppressed[candidates[i].ci] = true;
                        } else {
                            suppressed[candidates[j].ci] = true;
                        }
                    }
                }
            }
        }
    }

    // PASS 3: execute non-suppressed entries
    for candidate in &candidates {
        if !suppressed[candidate.ci] {
            execute_entry(state, candidate.ci, candidate.dir, &candidate.strat);
        }
    }
}
```

**`coordinator.rs` — add correlation matrix computation:**
```rust
pub fn compute_correlation_matrix(state: &SharedState, window: u32) -> Vec<Vec<f64>> {
    // Compute rolling 15-bar returns for each coin
    // Compute Pearson correlation matrix
    // Returns n×n matrix
}
```

---

## Validation Method

### RUN49.1 — Correlation Filter Grid Search (Rust + Rayon)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — all coins trade independently

**Implementation note:** Computing a rolling 18×18 correlation matrix every bar is O(n²) per bar. For 35,000 bars × 18 coins, this is manageable but must be implemented efficiently.

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CORR_SUPPRESS_THRESHOLD` | [0.60, 0.70, 0.75, 0.80, 0.85] |
| `CORR_WINDOW_BARS` | [10, 15, 20, 30] |
| `CORR_FILTER_MODE` | [1=suppress_weakest, 2=random_drop] |

**Per coin:** 5 × 4 × 2 = 40 configs (but this is a portfolio-level filter, so it's evaluated on all 18 coins together)

**Portfolio-level evaluation:** Unlike other RUNs where per-coin PF is measured, this RUN must be evaluated at the portfolio level (all 18 coins together) because correlation effects are systemic.

**Key metrics:**
- `portfolio_PF_delta`: portfolio profit factor change vs baseline
- `portfolio_max_DD_delta`: max drawdown change vs baseline
- `correlation_cluster_size_avg`: average number of correlated coins suppressed per signal event
- `suppression_rate`: % of entries suppressed by the filter
- `suppression_quality`: % of suppressed entries that would have been losing trades (i.e., correct suppressions)

### RUN49.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `CORR_THRESHOLD × CORR_WINDOW` on portfolio performance
2. Test: evaluate on held-out month with those params

**Pass criteria:**
- Portfolio OOS max_DD < baseline portfolio max_DD (primary goal: reduce clustered drawdowns)
- Portfolio OOS P&L ≥ 90% of baseline P&L (don't sacrifice too much for drawdown reduction)
- Suppression rate 10–30% (filter isn't too aggressive or too lenient)

### RUN49.3 — Combined Comparison

Side-by-side (portfolio-level):

| Metric | Baseline (v16) | Correlation Filter | Delta |
|--------|---------------|------------------|-------|
| Portfolio P&L | $X | $X | +$X (−X%) |
| Portfolio WR% | X% | X% | +Ypp |
| Portfolio PF | X.XX | X.XX | +0.XX |
| Portfolio Max DD | X% | X% | −Ypp |
| Avg Cluster Size | 1.0 | X | +N |
| Suppression Rate | 0% | X% | — |
| Correct Suppressions | — | X% | — |
| Wrong Suppressions | — | X% | — |

---

## Why This Could Fail

1. **Correlation is backward-looking:** Rolling 15-bar correlation may not reflect current market structure. By the time you detect high correlation, it may have already broken down.
2. **Loses diversification benefit:** If BTC and ETH are correlated at 0.8 but each has independent signals, suppressing one reduces total expected value without proportional drawdown reduction.
3. **Wrong suppress:** The "weakest" signal by z-score may still be a good trade. Suppressing it because it's correlated with another coin is a false negative.

---

## Why It Could Succeed

1. **Clustered drawdowns are the primary risk:** When all coins correlate during a broad sell-off, COINCLAW takes 8 simultaneous SL hits. Suppressing correlated entries reduces this clustering.
2. **Simple, intuitive:** The logic is sound — don't over-concentrate in correlated positions. This is standard portfolio management practice.
3. **Complementary to existing filters:** This doesn't change entry signals — it only prevents *simultaneous* entries in correlated coins. The system still gets the exposure, just not all at once.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN49 Correlation Filter |
|--|--|--|
| Coin independence | Full independence | Correlation-adjusted |
| Cluster detection | None | Rolling 15-bar return correlation |
| Position concentration | Uncapped | Max 4 correlated in same direction |
| Suppression logic | None | Signal quality ranking |
| Expected Max DD | X% | −15–30% |
| Expected P&L | $X | −5–10% (opportunity cost) |
| Implementation | check_entry only | Two-pass with correlation matrix |
