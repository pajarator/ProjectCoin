# RUN56 — SMA Cross-Back Depth Filter: Exit Quality Gate for Signal Exits

## Hypothesis

**Named:** `sma_cross_depth_filter`

**Mechanism:** The current SMA exit fires when price crosses back above/below SMA20 after being on the opposite side. However, it doesn't distinguish between:
1. **Deep penetration:** Price ran well past SMA20 (e.g., +1% above for a LONG) before reverting — a strong, high-quality exit signal
2. **Shallow touch:** Price barely crossed SMA20 (e.g., 0.05% penetration) before reversing — a weak, possibly false exit signal

Shallow SMA crosses often occur during ranging markets where price oscillates around SMA20 without establishing a clear trend. Exiting on these shallow touches takes profit too early, before the mean-reversion move has fully developed.

The fix: require SMA cross-back to exceed a minimum penetration depth before the SMA exit fires:
```
sma_depth = |price_at_cross - sma20_at_cross| / sma20_at_cross
if sma_depth < MIN_SMA_DEPTH: block SMA exit (wait for Z0 exit or deeper SMA touch)
```

**Why this is not a duplicate:**
- No prior RUN measured SMA penetration depth as an exit quality gate
- All prior exit tests used fixed thresholds (z > 0.5, held >= N bars) without measuring *how far* the cross occurred
- SMA exit has never been quality-filtered in any prior RUN

---

## Proposed Config Changes

```rust
// RUN56: SMA Cross-Back Depth Filter
pub const SMA_DEPTH_FILTER_ENABLE: bool = true;
pub const MIN_SMA_DEPTH_PCT: f64 = 0.003;  // price must penetrate SMA by ≥0.3% before exit fires
```

**`state.rs` — add SMA penetration tracking to CoinState:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub crossed_sma: bool,           // true when price crossed SMA20 this bar
    pub sma_cross_depth_pct: f64,   // penetration depth when crossed
    pub crossed_sma_bars_ago: u32,   // bars since SMA cross
}
```

**`engine.rs` — SMA exit requires minimum depth:**
```rust
fn check_sma_exit(state: &mut SharedState, ci: usize, price: f64) -> bool {
    let ind = match &state.coins[ci].ind_15m {
        Some(i) => i.clone(),
        None => return false,
    };
    let pos = match &state.coins[ci].pos {
        Some(p) => p.clone(),
        None => return false,
    };
    let held = state.coins[ci].candles_held;
    let pnl = /* compute pnl_pct */;

    if held >= config::MIN_HOLD_CANDLES && pnl > 0.0 {
        match pos.dir.as_str() {
            "long" => {
                // Current: if price < SMA20 → SMA exit
                // New: if price < SMA20 AND penetration depth >= MIN_SMA_DEPTH
                if price < ind.sma20 {
                    if config::SMA_DEPTH_FILTER_ENABLE {
                        // Measure how far price crossed below SMA20
                        let penetration = (ind.sma20 - price) / ind.sma20;
                        if penetration < config::MIN_SMA_DEPTH_PCT {
                            return false;  // shallow touch, ignore SMA exit
                        }
                    }
                    close_position(state, ci, price, "SMA", TradeType::Regime);
                    return true;
                }
            }
            "short" => {
                if price > ind.sma20 {
                    if config::SMA_DEPTH_FILTER_ENABLE {
                        let penetration = (price - ind.sma20) / ind.sma20;
                        if penetration < config::MIN_SMA_DEPTH_PCT {
                            return false;
                        }
                    }
                    close_position(state, ci, price, "SMA", TradeType::Regime);
                    return true;
                }
            }
        }
    }
    false
}
```

---

## Validation Method

### RUN56.1 — SMA Depth Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — SMA exit fires on any cross, no depth requirement

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MIN_SMA_DEPTH_PCT` | [0.002, 0.003, 0.004, 0.005, 0.006] |

5 configs × 18 coins = 90 backtests (fast grid)

**Key metrics:**
- `shallow_cross_rate`: % of SMA exits that would be blocked by the filter (cross depth < threshold)
- `avg_cross_depth`: average penetration depth of SMA exits
- `WR_delta`: win rate change vs baseline for trades that now hold longer
- `PF_delta`: profit factor change vs baseline
- `avg_held_bars_delta`: how many additional bars trades hold before exiting

**Also measure:** What happens to trades that would have exited via shallow SMA but don't? Do they eventually hit a deeper SMA exit, Z0 exit, or SL? This determines whether the filter simply delays exits or actually prevents premature exits.

### RUN56.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `MIN_SMA_DEPTH_PCT` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Average held bars increases (expected — deeper SMA cross takes longer to develop)
- Portfolio OOS P&L ≥ baseline

### RUN56.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | SMA Depth Filter | Delta |
|--------|---------------|-----------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Avg Held Bars | X | X | +N |
| SMA Exit Rate | X% | X% | −Ypp |
| Z0 Exit Rate | X% | X% | +Ypp |
| Shallow Cross Rate | — | X% | — |
| Avg Cross Depth | X% | X% | +Y% |

---

## Why This Could Fail

1. **Depth requirement delays all exits:** If the minimum depth is too high, the filter may block even genuine SMA exits, causing trades to hold through adverse moves until the depth is met — or until they hit SL.
2. **Price may not always deeply penetrate SMA:** In low-vol, low-momentum markets, mean-reversion moves may only reach SMA20 without deeply crossing. Requiring deep penetration may prevent exits that would have been good trades.
3. **Z0 exit as backup is already in place:** If shallow SMA is blocked, the Z0 exit (`z > 0.5`) catches the trade at the same point as the shallow SMA would have. The filter may not change outcomes substantially.

---

## Why It Could Succeed

1. **Shallow SMA crosses are noise:** In ranging markets, price oscillating around SMA20 produces many shallow touches that fire the SMA exit prematurely. Requiring depth filters this noise.
2. **Deeper = more conviction:** A deep penetration of SMA20 shows the market made a genuine attempt to trend. The subsequent reversion is more likely to complete, making the SMA exit more reliable.
3. **Trivial implementation:** One new config parameter, one depth check in the SMA exit branch.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN56 SMA Depth Filter |
|--|--|--|
| SMA exit trigger | Any cross | Deep cross (≥0.3% penetration) |
| Exit quality | All equal | Depth-qualified |
| Shallow crosses | Always exit | Blocked, held for Z0 |
| Avg held bars | X | +2–4 bars |
| Expected WR% | X% | +1–3pp |
| Expected PF | X.XX | +0.03–0.08 |
| Implementation | check_exit SMA branch | Same + depth check |
