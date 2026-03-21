# RUN61 — RSI Divergence Confirmation for LONG Entries

## Hypothesis

**Named:** `rsi_divergence_confirmation`

**Mechanism:** Classic technical analysis identifies "bullish divergence" as a high-probability reversal signal: when price makes a lower low but RSI makes a higher low, the downward momentum is weakening and a reversal is likely. COINCLAW currently doesn't use RSI trend direction as an entry confirmation for LONG trades.

**For LONG entries:**
```
// Classic bullish divergence check:
// Price at entry bar is lower than price N bars ago
// BUT RSI at entry bar is higher than RSI N bars ago
// → momentum shifting up despite flat/bearish price action

price_trend = ind.p - ind.p_N  // negative = lower low
rsi_trend = ind.rsi - ind.rsi_N  // positive = higher low in RSI

if price_trend < -PRICE_DECLINE_THRESHOLD  // price made a lower low
   AND rsi_trend > RSI_RECOVERY_THRESHOLD  // RSI made a higher low
   AND rsi_trend > 0:  // RSI itself is recovering
   → bullish divergence confirmed → enhanced LONG entry confidence
```

**Why this is not a duplicate:**
- No prior RUN has tested RSI divergence as an entry filter
- No prior RUN used RSI trend direction (as opposed to RSI level) as a confirmation signal
- All prior RSI tests used the RSI *level* (e.g., RSI < 30) not the RSI *trend*
- This is a classic technical pattern (divergence) not yet encoded in COINCLAW

---

## Proposed Config Changes

```rust
// RUN61: RSI Divergence Confirmation
pub const RSI_DIVERGENCE_ENABLE: bool = true;
pub const RSI_DIVERGENCE_LOOKBACK: u32 = 6;   // compare current bar to N bars ago
pub const PRICE_DECLINE_THRESHOLD: f64 = 0.005; // price must be ≥0.5% below lookback bar
pub const RSI_RECOVERY_THRESHOLD: f64 = 2.0;   // RSI must be ≥2 points above lookback RSI
pub const RSI_DIVERGENCE_MODE: u8 = 1;         // 1=confirm_long_only, 2=confirm_both
```

**`indicators.rs` — add RSI trend fields:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub rsi_delta: f64,        // rsi_current - rsi_prev
    pub rsi_6b: f64,         // RSI 6 bars ago
}
```

**`strategies.rs` — add divergence check:**
```rust
fn has_bullish_divergence(ind: &Ind15m) -> bool {
    if !config::RSI_DIVERGENCE_ENABLE { return true; }
    if ind.p.is_nan() || ind.rsi.is_nan() || ind.rsi_6b.is_nan() { return true; }

    let price_change = ind.p - ind.rsi_6b;  // using rsi_6b as proxy for p_6b (need separate field)
    // Better: compare to price N bars ago — need to track price history
    // For backtest simplicity: use ROC (rate of change) as proxy
    let price_roc = /* roc over RSI_DIVERGENCE_LOOKBACK bars */;

    // Price making lower low: price_roc < -PRICE_DECLINE_THRESHOLD
    // RSI making higher low: rsi > rsi_N + RSI_RECOVERY_THRESHOLD

    let price_lower = price_roc < -config::PRICE_DECLINE_THRESHOLD;
    let rsi_recovering = ind.rsi > (ind.rsi_6b + config::RSI_RECOVERY_THRESHOLD);

    price_lower && rsi_recovering
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    // ... existing entry checks ...
    // NEW: divergence confirmation
    if config::RSI_DIVERGENCE_MODE >= 1 {
        if !has_bullish_divergence(ind) { return false; }
    }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN61.1 — RSI Divergence Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no RSI divergence check

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `RSI_DIVERGENCE_LOOKBACK` | [4, 6, 8, 10] |
| `PRICE_DECLINE_THRESHOLD` | [0.003, 0.005, 0.008] |
| `RSI_RECOVERY_THRESHOLD` | [1.0, 2.0, 3.0] |

**Per coin:** 4 × 3 × 3 = 36 configs × 18 coins = 648 backtests

**Key metrics:**
- `divergence_detection_rate`: % of bars where bullish divergence is detected
- `divergence_hit_rate`: % of divergence signals that are followed by a profitable LONG trade within N bars
- `WR_delta`: win rate change vs baseline for LONG trades
- `PF_delta`: profit factor change vs baseline
- `divergence_block_rate`: % of LONG entries blocked by divergence filter

### RUN61.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `LOOKBACK × PRICE_THRESHOLD × RSI_THRESHOLD` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta for LONG trades
- Divergence hit rate ≥ 55%
- Portfolio OOS P&L ≥ baseline

### RUN61.3 — Combined Comparison

Side-by-side LONG trades:

| Metric | Baseline LONGs (v16) | Divergence-Confirmed LONGs | Delta |
|--------|---------------------|-------------------------|-------|
| LONG WR% | X% | X% | +Ypp |
| LONG PF | X.XX | X.XX | +0.XX |
| LONG P&L | $X | $X | +$X |
| LONG Trade Count | N | M | −K |
| Divergence Block Rate | 0% | X% | — |
| Divergence Hit Rate | — | X% | — |

---

## Why This Could Fail

1. **Divergence is subjective:** Classic chart pattern definitions vary. The exact parameters (lookback N, price threshold, RSI threshold) are arbitrary and may not generalize.
2. **Divergence detection is rare:** With strict thresholds, divergence signals may be too infrequent to be useful. With loose thresholds, they may fire on noise.
3. **RSI trend is already captured by z-score:** The z-score already captures deviations from mean. RSI divergence may be redundant with z-score information.

---

## Why It Could Succeed

1. **One of the most reliable technical patterns:** Bullish divergence is a well-documented reversal signal with high predictive value across markets.
2. **Addresses momentum shift:** COINCLAW currently only checks RSI *level*, not RSI *direction*. Divergence confirms momentum is shifting before entry.
3. **Independent signal:** RSI trend direction is orthogonal to z-score level — they measure different things (momentum vs deviation from mean).

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN61 RSI Divergence Confirmation |
|--|--|--|
| Entry filter | z-level, RSI level | + RSI divergence trend |
| LONG entry | Standard | Enhanced with divergence |
| Divergence check | None | Price lower low + RSI higher low |
| Expected LONG WR% | ~38% | +3–6pp |
| Expected LONG PF | ~0.85 | +0.05–0.15 |
| Block rate | 0% | 15–30% |
