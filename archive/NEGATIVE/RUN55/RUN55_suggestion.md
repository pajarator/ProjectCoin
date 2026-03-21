# RUN55 — BTC-Altcoin Breadth Divergence Filter for ISO Shorts

## Hypothesis

**Named:** `btc_alt_breadth_divergence`

**Mechanism:** When BTC's z-score diverges significantly from the average altcoin z-score (`btc_z >> avg_z` or `btc_z << avg_z`), capital is rotating into or out of BTC. This cross-market rotation affects ISO short timing:
- **BTC Z >> Alt Z:** BTC is overbought while alts are less so — good for ISO shorts (BTC topping while alts hold)
- **BTC Z << Alt Z:** BTC is oversold while alts are less so — bad for ISO shorts (BTC dragging everything down)

The existing `MarketCtx` already computes `btc_z_valid` and `avg_z_valid`. This RUN adds a BTC-altcoin breadth divergence gate to ISO short entries.

**Divergence signal:**
```
divergence = btc_z - avg_z
ISO_SHORT timing enhanced:
  - divergence > DIVERGENCE_THRESHOLD → BTC overbought vs alts → ISO shorts more likely to succeed
  - divergence < -DIVERGENCE_THRESHOLD → BTC oversold vs alts → suppress ISO shorts
```

**Why this is not a duplicate:**
- RUN40 (BTC dominance scalp filter) applied BTC Z-spread to scalp entries only
- This RUN applies BTC Z-spread to ISO short timing, not scalp
- No prior RUN tested cross-coin breadth divergence as an ISO short filter
- RUN6 (ISO short discovery) and RUN34 (ISO drawdown mitigation) didn't use cross-coin signals

---

## Proposed Config Changes

```rust
// RUN55: BTC-Altcoin Breadth Divergence Filter
pub const DIVERGENCE_ENABLE: bool = true;
pub const DIVERGENCE_THRESHOLD: f64 = 1.0;   // btc_z must be > avg_z + 1.0σ to enhance ISO shorts
pub const DIVERGENCE_SUPPRESS_THRESHOLD: f64 = -1.0;  // btc_z < avg_z - 1.0σ → suppress ISO shorts
```

**`strategies.rs` — extend `iso_short_entry` with divergence gate:**
```rust
pub fn iso_short_entry(
    ind: &Ind15m,
    strat: IsoShortStrat,
    ctx: &MarketCtx,
) -> bool {
    // Existing entry logic first
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }

    let base_entry = match strat {
        // ... existing match arms unchanged ...
    };

    if !base_entry { return false; }

    // BTC-Altcoin Breadth Divergence gate (only for enhanced timing)
    if config::DIVERGENCE_ENABLE && ctx.btc_z_valid && ctx.avg_z_valid {
        let divergence = ctx.btc_z - ctx.avg_z;
        // Suppress when BTC is significantly oversold vs alts
        if divergence < config::DIVERGENCE_SUPPRESS_THRESHOLD {
            return false;
        }
        // Note: positive divergence (BTC overbought) doesn't block — it enhances timing
        // but doesn't prevent ISO short from firing
    }

    true
}
```

---

## Validation Method

### RUN55.1 — Divergence Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BTC-Altcoin divergence filter on ISO shorts

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `DIVERGENCE_THRESHOLD` | [0.5, 1.0, 1.5, 2.0] |
| `DIVERGENCE_SUPPRESS_THRESHOLD` | [-0.5, -1.0, -1.5, -2.0] |

**Per coin:** 4 × 4 = 16 configs × 18 coins = 288 backtests

**Also test:** Does the divergence filter help for specific ISO short strategies more than others? (IsoDivergence already uses BTC Z, so it may not benefit as much)

**Key metrics:**
- `suppression_rate`: % of ISO shorts suppressed by the filter
- `enhancement_rate`: % of ISO shorts that have positive divergence (would time them better)
- `WR_delta`: ISO short win rate change vs baseline
- `PF_delta`: ISO short profit factor change vs baseline
- `false_suppress_rate`: % of suppressed ISO shorts that would have been winners

### RUN55.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `DIVERGENCE_THRESHOLD × SUPPRESS_THRESHOLD` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS ISO P&L delta vs baseline
- False suppress rate < 25%
- Portfolio ISO short P&L ≥ baseline

### RUN55.3 — Combined Comparison

Side-by-side ISO short performance:

| Metric | Baseline ISO Shorts (v16) | Divergence-Filtered | Delta |
|--------|--------------------------|---------------------|-------|
| ISO WR% | X% | X% | +Ypp |
| ISO PF | X.XX | X.XX | +0.XX |
| ISO P&L | $X | $X | +$X |
| ISO Max DD | X% | X% | -Ypp |
| ISO Trade Count | N | M | −K (−X%) |
| Suppression Rate | 0% | X% | — |
| False Suppress Rate | — | X% | — |
| Avg Divergence at Entry | X | X | +Y |
