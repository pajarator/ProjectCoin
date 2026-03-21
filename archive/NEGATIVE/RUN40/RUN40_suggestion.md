# RUN40 — BTC Dominance Scalp Filter: Cross-Coin Relative Strength Gate

## Hypothesis

**Named:** `btc_dominance_scalp_filter`

**Mechanism:** When BTC's z-score significantly exceeds the cross-coin average z-score (`btc_z >> avg_z`), capital is rotating into BTC and away from altcoins. A scalp LONG entry on an altcoin during this rotation faces a headwind: the altcoin's oversold condition may persist or worsen even if its own indicators suggest a reversal. Similarly, when BTC's z-score is far below the cross-coin average (`btc_z << avg_z`), BTC is underperforming — alts have relative strength and scalp LONG entries should have a higher success rate.

The existing `MarketCtx` already computes both `btc_z` and `avg_z` (via `compute_breadth_and_context`). This RUN adds a BTC-dominance gate to scalp entries using the z-score spread `btc_z − avg_z`.

**Why this is not a duplicate:**
- RUN21 (BTC RSI Fear/Greed) used BTC RSI as a sentiment proxy — this uses the *relative z-score spread* between BTC and the altcoin basket as a dominance filter
- RUN12 (scalp market mode) gated scalp direction by the 3-mode regime (Long/Short/ISO_SHORT) — this gates scalp entries by the cross-coin relative performance differential
- RUN27/28 (momentum persistence) measured per-coin forward returns — this uses BTC vs alts relative performance as a systemic filter
- The `btc_z − avg_z` spread is a different signal from BTC RSI or BTC direction alone

---

## Proposed Config Changes

```rust
// RUN40: BTC Dominance Scalp Filter
// btc_z > avg_z + THRESHOLD → BTC outperforming, block LONG scalps
// btc_z < avg_z − THRESHOLD → BTC underperforming, block SHORT scalps
pub const BTC_DOM_SCALE_LONG: f64 = 1.0;   // block LONG when btc_z > avg_z + 1.0σ
pub const BTC_DOM_SCALE_SHORT: f64 = 1.0;  // block SHORT when btc_z < avg_z − 1.0σ
```

**`strategies.rs` change — `scalp_entry_with_price` (add MarketCtx param):**
```rust
pub fn scalp_entry_with_price(
    ind: &Ind1m,
    price: f64,
    ctx_opt: Option<&MarketCtx>,  // new: None for backtesting without ctx
) -> Option<(Direction, &'static str)> {
    // ... existing checks ...
    if let Some(ctx) = ctx_opt {
        // BTC Dominance filter: block scalp entries against BTC's direction
        if ctx.btc_z_valid && ctx.avg_z_valid {
            let spread = ctx.btc_z - ctx.avg_z;
            // Block LONG when BTC is significantly outperforming
            if spread > config::BTC_DOM_SCALE_LONG { return None; }
            // Block SHORT when BTC is significantly underperforming
            if spread < -config::BTC_DOM_SCALE_SHORT { return None; }
        }
    }
    // ... rest of scalp logic ...
}
```

**`engine.rs` change — `check_scalp_entry` passes MarketCtx:**
The current `check_scalp_entry` doesn't have `MarketCtx`. The coordinator calls `compute_breadth_and_context` before `check_entry` but not before `check_scalp_entry`. Pass the ctx to `check_scalp_entry` so it can gate entries.

---

## Validation Method

### RUN40.1 — Grid Search: BTC Dominance Threshold (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Challenge:** `btc_z − avg_z` requires 1m data for BTC + all other coins simultaneously. Since scalp runs on 1m, the BTC dominance signal should also be computed on 1m.

**Simplification for backtest:** Compute `btc_z − avg_z` on 15m bars (same timeframe as the breadth computation) and apply the gate to 1m scalp entries on the *same bar*. This is slightly conservative (the actual filter would react faster on 1m) but avoids needing 1m BTC data for all coins in the backtester.

**Grid search:**
| Parameter | Values |
|-----------|--------|
| `BTC_DOM_SCALE_LONG` | [0.5, 1.0, 1.5, 2.0] |
| `BTC_DOM_SCALE_SHORT` | [0.5, 1.0, 1.5, 2.0] |

**Per coin:** 4 × 4 = 16 configs × 18 coins = 288 backtests

**Compare:**
1. Each config's scalp-only P&L vs baseline scalp (no BTC filter)
2. Primary metric: `delta_PnL = filtered_scalp_PnL − baseline_scalp_PnL`
3. Secondary: delta_WR%, trade count reduction, `% trades blocked`

**Also measure:** Does the filter block more SHORT scalps (when BTC is underperforming) or LONG scalps (when BTC is outperforming)? Asymmetric blocking may reveal directional bias.

### RUN40.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `BTC_DOM_SCALE_LONG × BTC_DOM_SCALE_SHORT` per coin (or universal best if per-coin degrades >40%)
2. Test: evaluate with those params on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline scalp
- Portfolio OOS scalp P&L ≥ baseline
- Average trade count reduction < 50% (filter shouldn't block everything)

### RUN40.3 — Combined Comparison

Side-by-side:

| Metric | Baseline Scalp (v16) | BTC-Dominance Scalp | Delta |
|--------|---------------------|---------------------|-------|
| Scalp WR% | X% | X% | +Ypp |
| Scalp PF | X.XX | X.XX | +0.XX |
| Scalp Total P&L | $X | $X | +$X |
| Scalp Max DD | X% | X% | -Ypp |
| Trades Blocked | — | N | — |
| BTC Spread at Blocked | — | avg σ | — |

---

## Why This Could Fail

1. **BTC dominance changes on different timescale than 1m scalp signals:** By the time `btc_z − avg_z` widens enough to block, the scalp opportunity may have already resolved. The filter reacts too slowly.
2. **BTC dominance doesn't predict altcoin reversion quality:** The fact that BTC is outperforming doesn't mean altcoin scalp LONGs will fail — it might just mean BTC is rising faster while alts are still quietly mean-reverting.
3. **Asymmetric effect:** The filter may work well for blocking LONGs but not SHORTs, or vice versa, reducing net benefit.

---

## Why It Could Succeed

1. **BTC dominance filter is the missing "market context" for scalp:** Scalp runs on 1m indicators only (RUN12 fixed the 3-mode regime direction issue, but didn't address cross-coin relative strength). Adding BTC dominance fills this gap.
2. **Leverages existing `MarketCtx.btc_z`:** No new data fetch needed — the field is already computed.
3. **Simple threshold, large effect:** A 1σ spread threshold blocks only the most extreme BTC outperformance/underperformance events, preserving most trade opportunities.

---

## Comparison to Baseline

| | Current Scalp (v16) | RUN40 BTC Dominance Scalp |
|--|--|--|
| BTC context | None | btc_z − avg_z gate |
| LONG scalp filter | Market mode only (RUN12) | Market mode + BTC dominance |
| SHORT scalp filter | Market mode only (RUN12) | Market mode + BTC dominance |
| Data required | 1m OHLCV | Same + BTC z-score (already computed) |
| Trade count | N | N × (1 − block_rate) |
