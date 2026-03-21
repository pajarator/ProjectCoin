# RUN70 — Z-Score Convergence Filter: Market-Wide Entry Confirmation

## Hypothesis

**Named:** `z_convergence_filter`

**Mechanism:** When multiple coins' Z-scores are simultaneously falling toward their respective means at the same time (converging), the market is in a broad mean-reversion regime. This cross-coin synchronization is more powerful than a single coin's oversold condition — it suggests systemic buying pressure that affects the entire market.

**Convergence signal:**
```
convergence = count of coins where z_score is rising (recovering toward mean)
convergence_pct = convergence / total_valid_coins

For LONG entry:
  if convergence_pct >= CONVERGENCE_THRESHOLD:
    → market-wide mean reversion is active → enhanced LONG confidence

For SHORT entry:
  if convergence_pct >= CONVERGENCE_THRESHOLD:
    → market-wide mean reversion is active → enhanced SHORT confidence (coins are reverting simultaneously)
```

**Why this is not a duplicate:**
- No prior RUN has measured whether multiple coins' Z-scores are moving in the same direction simultaneously
- Breadth measures static coin count; this measures directional agreement across coins
- All prior entry filters are per-coin; this is a cross-coin convergence filter

---

## Proposed Config Changes

```rust
// RUN70: Z-Score Convergence Filter
pub const Z_CONVERGENCE_ENABLE: bool = true;
pub const Z_CONVERGENCE_WINDOW: u32 = 2;  // bars to measure convergence over
pub const CONVERGENCE_THRESHOLD: f64 = 0.50;  // ≥50% of coins must be converging
```

**`coordinator.rs` — add convergence to MarketCtx:**
```rust
pub struct MarketCtx {
    // ... existing fields ...
    pub z_convergence_pct: f64,  // NEW: % of coins with rising z-score
    pub z_convergence_bars: u32,  // NEW: bars in current convergence streak
}
```

**`strategies.rs` — convergence gate:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, ctx: &MarketCtx) -> bool {
    // ... existing entry checks ...
    if config::Z_CONVERGENCE_ENABLE {
        if ctx.z_convergence_pct < config::CONVERGENCE_THRESHOLD {
            return false;  // insufficient market-wide convergence
        }
    }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN70.1 — Z-Convergence Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no convergence filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CONVERGENCE_THRESHOLD` | [0.40, 0.50, 0.60, 0.70] |
| `Z_CONVERGENCE_WINDOW` | [1, 2, 3] |

**Per coin:** 4 × 3 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `convergence_block_rate`: % of entries blocked by convergence filter
- `convergence_hit_rate`: % of convergence-filtered entries that are followed by profitable trades
- `WR_delta`: win rate change vs baseline
- `PF_delta`: profit factor change vs baseline

### RUN70.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `CONVERGENCE_THRESHOLD × Z_CONVERGENCE_WINDOW` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- Convergence hit rate ≥ 55%

### RUN70.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Z-Convergence Filter | Delta |
|--------|---------------|---------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| Convergence Block Rate | 0% | X% | — |
| Convergence Hit Rate | — | X% | — |
