# RUN15b: NN Scalp Filter — 1m Data, 10 Coins

**Date:** 2026-03-16
**Objective:** Test whether a Logistic Regression filter on 9 features can improve the win rate of COINCLAW scalp signals (vol_spike_rev + stoch_cross) on 1-minute data.

---

## Experiment Design

### Signals Tested
- **vol_spike_rev:** Volume > 3.5× MA + RSI < 20 (long) or RSI > 80 (short)
- **stoch_cross:** Stochastic K crossing D at extreme levels (K < 20 or K > 80)

### Model
- Logistic Regression trained on 9 features: RSI(14), vol_ratio, stoch_k, stoch_d, stoch_cross direction, bb_position, roc_3, avg_body_3, hour_of_day
- Separate long/short models per coin
- Train: first 50% of 1-year data; Test: second 50% (strict OOS)

### Evaluation
- **Actual COINCLAW scalp parameters:** TP=0.80%, SL=0.10%
- **Fees:** 0.1% per side (taker)
- **Slippage:** 0.05% per side
- **Max hold:** 60 bars (1 hour) — exit at close if TP/SL not hit
- **TP/SL simulation:** use high/low per bar; pessimistic (SL checked first when both hit in same bar)

---

## Corrected Results (Rust Implementation)

### Binary (All Signals — No Filter)

| Coin | Signals | Avg WR | PF |
|------|---------|--------|-----|
| BTC | ~18,400 | 7.4% | ~0.09 |
| ETH | ~20,500 | 7.5% | ~0.09 |
| BNB | ~17,900 | 7.2% | ~0.09 |
| SOL | ~19,800 | 7.3% | ~0.09 |
| ADA | ~21,000 | 7.2% | ~0.09 |
| XRP | ~20,200 | 7.4% | ~0.09 |
| DOGE | ~19,700 | 7.3% | ~0.09 |
| LTC | ~19,000 | 7.3% | ~0.09 |
| LINK | ~20,200 | 7.1% | ~0.09 |
| DOT | ~19,700 | 7.2% | ~0.09 |
| **AVG** | **~196k total** | **7.3%** | **~0.09** |

### LR Filter (LR>55% and LR>60%)

**0 trades on all 10 coins at both thresholds.**

The LR model learns that every signal is a loss (WR ≈7%). It never predicts P(win) > 0.5, let alone > 0.55 or > 0.60. The model has nothing to learn — it correctly identifies that these signals have no edge.

---

## Breakeven Analysis

With TP=0.80%, SL=0.10%, fee=0.1%/side, slip=0.05%/side:
- Net win per trade: +0.80% − 0.30% = **+0.50%**
- Net loss per trade: −0.10% − 0.30% = **−0.40%**
- **Breakeven WR = 0.40 / (0.50 + 0.40) ≈ 44%**

Actual WR: **7.3%** — 37 percentage points below breakeven.

---

## Bugs in Original Implementation

### Bug 1: 3-bar lookahead target (critical)
```python
future_return = c.pct_change(3).shift(-3)  # did price go up in 3 minutes?
```
This measures 3-bar price direction — approximately a coin-flip (~49% "up"). COINCLAW scalps use TP=0.80%/SL=0.10% which resolve over variable bar counts, often reaching SL within 1-5 bars. The 3-bar target is unrelated to actual scalp trade outcomes.

**Impact:** Inflated baseline WR from real ~7% to ~49% (coin-flip). Made all results meaningless.

### Bug 2: 21-day sample window
```python
df = df.tail(30000)  # ~21 days of 1m data
```
Train period = ~10 days, test period = ~10 days. All results are statistically unreliable.

### Bug 3: Threshold selected on test data
Thresholds [0.50, 0.55, 0.60] compared on the test set; best reported. Classic in-sample bias on threshold selection.

### Bug 4: No fees or slippage
Wins and losses computed on raw price change. At 1m with TP=0.80% / SL=0.10%, fees and slippage (0.30% total) consume 60% of TP and 300% of SL. This is not optional to model.

### Bug 5: stoch_d NaN propagation (Rust fixed)
`rolling_mean(stoch_k, 3)` used a rolling-sum accumulator. When stoch_k is NaN for the first 13 bars (indicator warmup), the running sum `s` becomes NaN and never recovers (`NaN - NaN = NaN` when subtracting the old window element). This made stoch_d all-NaN, causing all bars to be skipped in the feature validity check, producing zero signals.

**Fix:**
```rust
let mut stoch_d = vec![f64::NAN; n];
for i in 2..n {
    if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
        stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
    }
}
```

---

## Original Claimed Results (INVALID)

| Metric | Baseline | NN Filter | Delta |
|--------|----------|-----------|-------|
| Win Rate | 49.3% | 56.5% | +7.2 pts |
| Signals | 21,831 | 6,268 | −71% |

These numbers reflect 3-bar price direction on 21 days of data without fees. The 49.3% baseline is a coin-flip. The 56.5% "improvement" is the NN learning to pick slightly-favorable coin-flips — not actual profitability.

---

## Conclusions

### Negative result — scalp signals have no edge at COINCLAW parameters

1. **Binary WR = 7.3%** with TP=0.80%/SL=0.10%. Breakeven requires ≈44%. The gap is 37 percentage points — not a tuning problem, a fundamental mismatch between signal resolution (1m) and the TP/SL ratio.

2. **The TP=0.80%/SL=0.10% ratio (8:1 R:R) requires very high WR to be profitable.** With 8:1 R:R and fees, the breakeven is near 44%. Most scalp signals on 1m do not reach TP before SL — price reverts in the wrong direction or grinds against the position.

3. **The LR filter cannot help** when the underlying signals have no edge. The model correctly assigns near-zero win probability to every signal.

4. **The original 49.3% baseline was meaningless** — it measured whether the coin went up or down in the next 3 minutes, which is essentially random. Any NN "improving" a coin-flip is also finding patterns in noise.

### Why the original conclusion was accidentally partially correct

"NN filter improves scalp WR" — directionally plausible for a different outcome metric (e.g., 3-bar direction). But this conclusion is irrelevant to profitability, and the corrected experiment shows the underlying scalp signals cannot support the COINCLAW scalp overlay at all.

### Recommendation

**No COINCLAW changes.** If scalp improvement is desired, the parameters would need to change fundamentally: either tighter TP/SL ratio (reducing breakeven WR to ~35%) or a different entry strategy on higher timeframe (5m/15m) data where signals can breathe to reach TP.

---

## Files

| File | Description |
|------|-------------|
| `run17_nn_scalp.py` | Original buggy Python implementation |
| `run17_results.json` | Results from original buggy run |
| `run15b_corrected_results.json` | Results from corrected Rust implementation |
| `README.md` | This file (updated with corrected conclusions) |

Corrected implementation source: `tools/src/run15b.rs`
