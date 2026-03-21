# RUN54 — Volatility Regime Entry Filter: Trade Only When Volatility Is Below Median

## Hypothesis

**Named:** `volatility_regime_filter`

**Mechanism:** Volatility is itself mean-reverting. When ATR spikes above its 20-bar rolling average (high-vol regime), the market is in an unstable, noise-dominated state. Mean-reversion strategies perform poorly in high-vol regimes because:
1. The "oversold" condition is more extreme and takes longer to resolve
2. The noise around the mean is larger, causing more SL hits before reversion completes
3. The relationship between z-score and actual price deviation breaks down

The hypothesis is that **regime trades should only fire when volatility is below or near its median**:
- Compute rolling volatility percentile rank: `vol_rank = rank of current ATR within last N bars`
- Only take LONG/SHORT entries when `vol_rank < VOL_THRESHOLD` (e.g., < 0.60 — below 60th percentile)
- This ensures trades are taken in calm, low-noise environments where mean-reversion signals are more reliable

**Why mean-reversion loves low vol:**
- In low-vol environments, z-score is a cleaner signal (less noise around the mean)
- Price deviations are more likely to be genuine mean-reversion opportunities
- SL=0.3% is more likely to be sufficient when vol is low

**Why this is not a duplicate:**
- RUN26 (ATR dynamic stops) used ATR to widen SL — this uses volatility to *suppress entries entirely*
- RUN27 (momentum) used ADX for trend confirmation — this uses ATR for vol suppression
- RUN43 (breadth momentum) tested breadth velocity — this tests volatility percentile rank
- No prior RUN used volatility percentile rank as an entry gate for mean-reversion strategies

---

## Proposed Config Changes

```rust
// RUN54: Volatility Regime Entry Filter
pub const VOL_FILTER_ENABLE: bool = true;
pub const VOL_RANK_WINDOW: u32 = 20;          // rolling window for ATR percentile
pub const VOL_ENTRY_THRESHOLD: f64 = 0.60;   // only trade when ATR rank < 60th percentile
pub const VOL_SUPPRESS_MODE: u8 = 1;          // 0=disabled, 1=strict (< threshold), 2=relaxed (< 75th pct)
```

**`indicators.rs` — add volatility rank to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub atr: f64,
    pub atr_ma: f64,           // rolling 20-bar average ATR
    pub atr_rank: f64,         // NEW: percentile rank of current ATR (0.0 to 1.0)
}
```

**`strategies.rs` — add volatility filter to entry functions:**
```rust
fn passes_vol_filter(ind: &Ind15m) -> bool {
    if !config::VOL_FILTER_ENABLE { return true; }
    if ind.atr_rank.is_nan() { return true; }  // not enough data
    match config::VOL_SUPPRESS_MODE {
        0 => true,
        1 => ind.atr_rank < config::VOL_ENTRY_THRESHOLD,  // strict
        2 => ind.atr_rank < 0.75,                         // relaxed
        _ => true,
    }
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }
    if !passes_vol_filter(ind) { return false; }  // NEW
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN54.1 — Volatility Regime Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no volatility filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VOL_ENTRY_THRESHOLD` | [0.50, 0.60, 0.70, 0.80] |
| `VOL_RANK_WINDOW` | [15, 20, 30] |
| `VOL_SUPPRESS_MODE` | [1=strict, 2=relaxed] |

**Per coin:** 4 × 3 × 2 = 24 configs × 18 coins = 432 backtests

**Also test:** Is the effect different by strategy type? VwapReversion (tight reversion) might be more sensitive to vol than AdrReversal (larger move target).

**Key metrics:**
- `vol_filter_block_rate`: % of entries blocked by volatility filter
- `WR_delta`: win rate change vs baseline for non-blocked entries
- `PF_delta`: profit factor change vs baseline
- `Sharpe_delta`: Sharpe ratio change vs baseline
- `false_block_rate`: % of blocked entries that would have been winners

### RUN54.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `VOL_THRESHOLD × VOL_WINDOW` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- Portfolio OOS Sharpe ≥ baseline
- Block rate 15–50% (not too aggressive or too lenient)

### RUN54.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Volatility Filter | Delta |
|--------|---------------|-----------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K (−X%) |
| Vol Filter Block Rate | 0% | X% | — |
| False Block Rate | — | X% | — |
| Avg ATR Rank at Entry | X% | X% | — |

---

## Why This Could Fail

1. **Volatility spikes during trends:** In a sustained uptrend (which COINCLAW avoids with LONG mode), volatility often rises. If the filter blocks entries during rising-vol periods, it might block exactly when the best opportunities arise.
2. **Low-vol environments are rare:** If most of the time volatility is above the threshold, the filter blocks too many trades and reduces P&L below acceptable levels.
3. **ATR rank doesn't capture the right vol:** ATR measures volatility in price terms. The relevant measure for mean-reversion reliability might be z-score dispersion or BB width, not ATR.

---

## Why It Could Succeed

1. **Mechanically sound:** Low-vol = less noise = cleaner mean-reversion signals = higher win rate
2. **Proven concept:** Many trading systems use Keltner Channel breakouts (volatility above average = breakout, below = range-bound). This inverts the logic: trade mean-reversion when vol is low
3. **Simple implementation:** ATR rank is already computable from existing data. No new fetches required

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN54 Volatility Regime Filter |
|--|--|--|
| Entry filter | None | ATR percentile rank < 60th |
| Volatility awareness | None | Rolling 20-bar ATR rank |
| Trade count | N | N × (1 − block_rate) |
| Expected WR% | ~38% | +3–8pp |
| Expected PF | ~0.88 | +0.05–0.15 |
| Expected block rate | 0% | 20–40% |
| ATR rank at blocked | — | >70th percentile |
