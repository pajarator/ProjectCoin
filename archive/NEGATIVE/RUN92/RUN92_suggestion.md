# RUN92 — Exit Reason Weighted Learning: Dynamic Signal Weighting Based on Historical Exit Performance

## Hypothesis

**Named:** `exit_weighted_signals`

**Mechanism:** COINCLAW currently treats all regime entry signals equally — when a signal fires, the trade is opened with no consideration of the historical performance of similar entry setups. But the 5 exit reasons (SL, SMA, Z0, BE, MAX_HOLD) have very different profit implications. The Exit Reason Weighted Learning system tracks, per-coin, which types of entries tend to exit via which reasons, and weights new entries by their expected exit quality.

**Exit Reason Weighted Learning:**
- Track per-coin exit reason distribution over trailing N trades (e.g., 50 trades)
- For each entry, compute `expected_exit_score = weighted_avg(exit_reason_pf)` where weights are based on entry characteristics (z_magnitude, strategy type, time of day)
- If `expected_exit_score < MIN_EXIT_SCORE` → suppress entry (similar setups historically end badly)
- Update weights after each trade using exponential moving average
- Minimum sample: require 20+ historical trades before enabling weighting

**Why this is not a duplicate:**
- RUN19 (Kelly sizing) used aggregate historical stats for position sizing — this uses per-exit-reason performance to WEIGH entry signals, not size positions
- RUN52 (z-confidence sizing) sized positions by entry z-score — this sizes by historical exit quality of similar entries
- No prior RUN has used historical exit reason performance as a dynamic entry gate

**Mechanistic rationale:** Not all entry signals are equal. A LONG entry when the coin's historical LONG exits are 60% via SMA (good) vs 30% via SL (bad) is different from a coin where historical LONG exits are 70% via SL. The Exit Reason Weighted system learns which entry contexts historically produce which exit types and suppresses entries where the historical pattern suggests poor outcomes.

---

## Proposed Config Changes

```rust
// RUN92: Exit Reason Weighted Learning
pub const EXIT_WEIGHTED_ENABLE: bool = true;
pub const EXIT_WEIGHT_WINDOW: usize = 50;       // trailing N trades for exit reason stats
pub const EXIT_WEIGHT_MIN_SAMPLES: usize = 20;  // minimum trades before enabling weighting
pub const EXIT_WEIGHT_MIN_SCORE: f64 = 0.60;    // minimum expected exit score to allow entry
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub exit_reason_stats: ExitReasonStats,  // rolling exit reason performance
}

#[derive(Default)]
pub struct ExitReasonStats {
    pub sl_count: usize,
    pub sl_avg_pnl: f64,
    pub sma_count: usize,
    pub sma_avg_pnl: f64,
    pub z0_count: usize,
    pub z0_avg_pnl: f64,
    pub be_count: usize,
    pub be_avg_pnl: f64,
    pub max_hold_count: usize,
    pub max_hold_avg_pnl: f64,
}
```

**`engine.rs` — exit_weighted_entry_check:**
```rust
/// Compute expected exit score for a potential entry based on historical exit reasons.
fn compute_expected_exit_score(cs: &CoinState, strat: &str, z_magnitude: f64) -> f64 {
    if !config::EXIT_WEIGHTED_ENABLE { return 1.0; }

    let stats = &cs.exit_reason_stats;
    let total = (stats.sl_count + stats.sma_count + stats.z0_count
                 + stats.be_count + stats.max_hold_count) as f64;

    if total < config::EXIT_WEIGHT_MIN_SAMPLES as f64 {
        return 1.0;  // not enough data — allow entry
    }

    // Weight each exit reason by its historical performance
    // Normalize counts to weights
    let sl_weight = stats.sl_count as f64 / total;
    let sma_weight = stats.sma_count as f64 / total;
    let z0_weight = stats.z0_count as f64 / total;
    let be_weight = stats.be_count as f64 / total;
    let max_weight = stats.max_hold_count as f64 / total;

    // Score = weighted average of avg_pnl per exit reason
    // Positive pnl per exit is good, negative is bad
    let score = sl_weight * stats.sl_avg_pnl
              + sma_weight * stats.sma_avg_pnl
              + z0_weight * stats.z0_avg_pnl
              + be_weight * stats.be_avg_pnl
              + max_weight * stats.max_hold_avg_pnl;

    // Normalize to 0-1 range: positive scores are good, negative are bad
    // Map to [0, 1] where 0.5 = breakeven
    (score / 2.0) + 0.5  // approximately maps pnl to score
}

/// Update exit reason stats after a trade closes.
fn update_exit_stats(cs: &mut CoinState, reason: &str, pnl: f64) {
    let stats = &mut cs.exit_reason_stats;
    match reason {
        "SL" => { stats.sl_count += 1; stats.sl_avg_pnl = EMA(stats.sl_avg_pnl, pnl, stats.sl_count); }
        "SMA" => { stats.sma_count += 1; stats.sma_avg_pnl = EMA(stats.sma_avg_pnl, pnl, stats.sma_count); }
        "Z0" => { stats.z0_count += 1; stats.z0_avg_pnl = EMA(stats.z0_avg_pnl, pnl, stats.z0_count); }
        "BE" => { stats.be_count += 1; stats.be_avg_pnl = EMA(stats.be_avg_pnl, pnl, stats.be_count); }
        "MAX_HOLD" => { stats.max_hold_count += 1; stats.max_hold_avg_pnl = EMA(stats.max_hold_avg_pnl, pnl, stats.max_hold_count); }
        _ => {}
    }
}

fn EMA(prev: f64, new: f64, n: usize) -> f64 {
    let alpha = 2.0 / (n as f64 + 1.0);
    prev * (1.0 - alpha) + new * alpha
}

// In check_entry — gate on expected exit score:
let strat_name = /* ... */;
let z_mag = /* ... */;
let exit_score = compute_expected_exit_score(cs, &strat_name, z_mag);
if exit_score < config::EXIT_WEIGHT_MIN_SCORE {
    return;  // suppress entry — similar setups historically underperform
}

// In close_position — update stats:
update_exit_stats(cs, reason, pnl);
```

---

## Validation Method

### RUN92.1 — Exit Weighted Learning Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — all entry signals treated equally

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `EXIT_WEIGHT_WINDOW` | [30, 50, 100] |
| `EXIT_WEIGHT_MIN_SAMPLES` | [15, 20, 30] |
| `EXIT_WEIGHT_MIN_SCORE` | [0.50, 0.60, 0.70] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `weighted_entry_suppression_rate`: % of entries blocked by exit score filter
- `exit_score_distribution`: distribution of exit scores when entry allowed
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN92.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best MIN_SAMPLES × MIN_SCORE per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Suppression rate 5–30% (meaningful filtering)
- Suppressed entries have lower avg pnl than allowed entries (filter working)

### RUN92.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, equal signals) | Exit-Weighted Signals | Delta |
|--------|------------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Suppressed | 0% | X% | — |
| Avg Exit Score (allowed) | — | X | — |
| Suppressed Entry Avg PnL | — | $X | — |
| Allowed Entry Avg PnL | — | $X | — |

---

## Why This Could Fail

1. **Exit reason is forward-looking — we don't know it at entry:** At entry time, we don't know whether a trade will exit via SL or SMA. The correlation between entry characteristics and exit reason may be weak, making the weighting noisy.
2. **Sample size is limited:** With only 18 coins and 50-trade windows, the exit reason statistics may not be statistically significant. Coins with few trades per month won't have enough data.
3. **Self-reinforcing bias:** If a coin has a bad run of SL exits, the system suppresses entries — but these suppressed entries might have been winners. The system could over-suppress during normal variance.

---

## Why It Could Succeed

1. **Exit reasons have real signal:** Coins that consistently exit via SL (bad) vs SMA (good) have different underlying mean-reversion quality. Learning this pattern and suppressing bad-exit entries is principled.
2. **Adaptive to coin personality:** Each coin has different characteristics. Exit Reason Weighted Learning adapts to each coin's historical exit distribution rather than using a fixed threshold.
3. **Complementary to all other signals:** This doesn't replace any existing entry filter — it adds a historical performance layer on top.
4. **Self-correcting:** As conditions change, the exponential moving average update ensures the weights track recent performance rather than ancient history.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN92 Exit-Weighted Signals |
|--|--|--|
| Entry signals | All treated equally | Weighted by historical exit quality |
| Historical learning | None | Tracks exit reason performance per coin |
| Entry gate | z-score + vol + F6 | z-score + vol + F6 + exit score |
| Suppression trigger | None | Exit score < 0.60 |
| Adaptiveness | Static params | EMA-updated per-coin stats |
| Min sample | N/A | 20 trades |
