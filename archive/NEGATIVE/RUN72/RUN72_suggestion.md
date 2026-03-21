# RUN72 — Scalp Mode: Disabling Scalp During Sustained Low-Volatility Markets

## Hypothesis

**Named:** `scalp_choppy_mode`

**Mechanism:** Scalp trades require volatility to work — they profit from small price oscillations. During sustained low-volatility (choppy) markets, scalp entries fire frequently but fail because the price oscillation is too small to reach the TP (0.8%) before reversing back toward the SL (0.1%).

The fix: Detect sustained low-volatility across the market and suppress scalp entries during these periods:
1. Measure **market-wide volatility**: average ATR across all 18 coins (relative to price)
2. When avg ATR drops below `CHOPPY_ATR_THRESHOLD` for `CHOPPY_BARS` consecutive bars → activate **choppy mode**
3. During choppy mode: **suppress all scalp entries** (regime trades continue normally)
4. Exit choppy mode when avg ATR rises above threshold for `CHOPPY_EXIT_BARS` bars

**Why this is not a duplicate:**
- RUN36 (choppiness detector) tested CI, ADX, and Bayesian win-rate regime detection for scalp improvement
- RUN36 did NOT specifically suppress scalp during detected choppy periods — it tried to predict good/bad scalp bars
- This RUN uses a simpler market-wide ATR threshold approach and **fully suppresses** scalp during choppy mode
- Scalp is the primary P&L loser under realistic fees (RUN37); choppy mode suppression directly addresses this

---

## Proposed Config Changes

```rust
// RUN72: Scalp Choppy Mode
pub const SCALP_CHOPPY_ENABLE: bool = true;
pub const CHOPPY_ATR_THRESHOLD: f64 = 0.0015;  // avg rel ATR must be < 0.15%
pub const CHOPPY_BARS: u32 = 8;                  // sustained for 8 bars (~2h) to activate
pub const CHOPPY_EXIT_BARS: u32 = 4;             // ATR above threshold for 4 bars to exit
```

**`engine.rs` — check_market_volatility in coordinator:**
```rust
pub fn is_market_choppy(state: &SharedState, bars_in_chop: u32) -> bool {
    if !config::SCALP_CHOPPY_ENABLE { return false; }
    let mut below_threshold_count = 0;
    for cs in &state.coins {
        if let Some(ref ind) = cs.ind_15m {
            if !ind.atr_pct.is_nan() && ind.atr_pct < config::CHOPPY_ATR_THRESHOLD {
                below_threshold_count += 1;
            }
        }
    }
    let pct_below = below_threshold_count as f64 / state.coins.len() as f64;
    pct_below >= 0.6  // 60%+ of coins below ATR threshold = market is choppy
}
```

**`engine.rs` — check_scalp_entry suppressed during choppy:**
```rust
pub fn check_scalp_entry(state: &mut SharedState, ci: usize) {
    if is_market_choppy(state, config::CHOPPY_BARS) {
        return;  // suppress all scalp during choppy market
    }
    // ... rest of scalp entry logic unchanged ...
}
```

---

## Validation Method

### RUN72.1 — Choppy Mode Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — scalp always active

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CHOPPY_ATR_THRESHOLD` | [0.0010, 0.0015, 0.0020, 0.0025] |
| `CHOPPY_BARS` | [4, 8, 12, 16] |
| `CHOPPY_EXIT_BARS` | [2, 4, 6] |

**Per coin:** 4 × 4 × 3 = 48 configs × 18 coins = 864 backtests

**Key metrics:**
- `choppy_activation_rate`: % of bars where choppy mode is active
- `choppy_scalp_block_rate`: % of scalp entries blocked during choppy
- `choppy_WR%`: scalp win rate during non-choppy periods vs choppy periods
- `PF_delta`: profit factor change vs baseline
- `total_scalp_PnL_delta`: scalp P&L change vs baseline

### RUN72.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `CHOPPY_ATR_THRESHOLD × CHOPPY_BARS` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS scalp P&L delta vs baseline
- Choppy activation rate 15–40% (meaningful but not too frequent)

### RUN72.3 — Combined Comparison

Side-by-side scalp trades:

| Metric | Baseline Scalp (v16) | Choppy-Mode Scalp | Delta |
|--------|---------------------|------------------|-------|
| Scalp WR% | X% | X% | +Ypp |
| Scalp PF | X.XX | X.XX | +0.XX |
| Scalp P&L | $X | $X | +$X |
| Choppy Activation | 0% | X% | — |
| Scalp Block Rate | 0% | X% | — |
| Non-Choppy Scalp WR% | X% | X% | +Ypp |
