# RUN15e: Complete 7-Filter Test — Corrected Rust Implementation

**Date:** 2026-03-16
**Objective:** Test all 7 filter ideas originally attributed to RUN15e — including 4 filters that had fabricated results in the original document and were never actually run.

---

## Background

The original `RUN15e.md` was a summary document that:
1. Mislabeled experiments (called RUN15a "RUN15b")
2. Declared "INTEGRATE: Liquidity Filter + NN" based on buggy RUN15d results
3. Listed results for 7 filter configurations, but only 3 (Baseline, ToD, Liquidity) were ever implemented — the other 4 (ADX Regime, Trend Direction, Multi-TF, Streak) had fabricated numbers

This corrected run implements all 7 filters with proper methodology.

---

## Experiment Design

### Filters Tested

| Filter | Description | Original Status |
|--------|-------------|-----------------|
| **Baseline** | All vol_spike_rev + stoch_cross signals | ✓ Tested in RUN15d |
| **Time-of-Day** | UTC hours 14–21 (NYSE session) | ✓ Tested in RUN15d |
| **Liquidity** | vol_ratio ≥ 2.0 | ✓ Tested in RUN15d |
| **ADX Regime** | ADX(14) < 25 (non-trending / mean-revert only) | ✗ Fabricated in original |
| **Trend Direction** | Signal direction matches EMA9 vs EMA21 | ✗ Fabricated in original |
| **Multi-TF** | Signal direction matches EMA135 vs EMA315 (≈ 9×15m / 21×15m) | ✗ Fabricated in original |
| **Streak** | 3 consecutive losses → 5-signal cooldown | ✗ Fabricated in original |

Each filter tested binary (no gate) only. LR(9-feature) gate at threshold 0.55 also applied to all 7.

### Trade Parameters
- TP=0.80%, SL=0.10%, fee=0.10%/side, slip=0.05%/side, max hold 60 bars
- Breakeven WR required: **≈44%**

### Methodology
- Full 1-year 1m data (~430k–525k bars per coin), 10 coins
- 50/50 chronological split (train first half, test second half)
- LR trained on filtered signals from train half only
- TP/SL simulated on OHLCV bars (pessimistic: SL first if both hit same bar)
- ADX: Wilder's RMA (alpha = 1/14)
- EMA135/315: standard EMA (alpha = 2/(span+1)), applied to 1m bars as proxy for 9×15m / 21×15m

---

## Results

### Per-Coin Binary WR by Filter (OOS test half)

| Coin | Base | ToD | Liq | ADX | TrendDir | MultiTF | Streak |
|------|------|-----|-----|-----|----------|---------|--------|
| ADA  | 8.0% | 7.6% | 8.1% | 7.9% | 7.0% | 7.9% | 7.8% |
| BNB  | 7.7% | 7.8% | 7.8% | 7.9% | 6.9% | 7.6% | 7.6% |
| BTC  | 6.4% | 6.7% | 6.5% | 6.7% | 5.0% | 6.2% | 6.7% |
| DOGE | 7.3% | 7.2% | 7.7% | 7.2% | 7.4% | 7.3% | 7.2% |
| DOT  | 7.7% | 8.0% | 8.0% | 7.7% | 7.3% | 7.8% | 7.3% |
| ETH  | 7.5% | 7.5% | 7.4% | 7.6% | 6.3% | 7.1% | 7.4% |
| LINK | 5.9% | 5.7% | 5.6% | 5.8% | 5.2% | 5.5% | 5.9% |
| LTC  | 7.4% | 7.6% | 7.6% | 7.8% | 6.1% | 7.1% | 7.6% |
| SOL  | 7.9% | 7.9% | 7.7% | 7.7% | 8.5% | 7.4% | 7.5% |
| XRP  | 7.7% | 7.7% | 8.2% | 7.6% | 7.1% | 7.6% | 7.2% |

### Summary (OOS test half)

| Filter | Avg WR | Bin Trades | LR WR | LR Trades | LR% of Bin |
|--------|--------|------------|-------|-----------|------------|
| **Baseline ✓** | 7.3% | 225,580 | 0.0% | 0 | 0.0% |
| **ToD ✓** | 7.4% | 74,326 | 0.0% | 0 | 0.0% |
| **Liquidity ✓** | 7.5% | 48,542 | 0.0% | 0 | 0.0% |
| **ADX Regime ✗** | 7.4% | 128,435 | 0.0% | 0 | 0.0% |
| **Trend Dir ✗** | 6.7% | 13,599 | 0.0% | 0 | 0.0% |
| **Multi-TF ✗** | 7.2% | 113,425 | 0.0% | 0 | 0.0% |
| **Streak ✗** | 7.2% | 92,543 | 0.0% | 0 | 0.0% |

---

## Conclusions

### All 7 filters negative — including the 4 that had fabricated "positive" results

**Binary WR: universally 5–8%, none above 10%.** The original RUN15e.md listed WRs of 54–57% for the fabricated filters. Actual values: 6.7–7.5% for the working filters (ADX, Multi-TF, Streak) and 6.7% for Trend Direction. All are 37+ percentage points below the 44% breakeven.

**LR gate: 0 trades on all 7 configurations.** Identical to RUN15b/c/d. The LR model learns the dominant 93%-loss class and assigns P(win) ≈ 7% to every signal. No signal passes the 0.55 threshold regardless of which filter was applied.

**Trend Direction is the worst filter** (avg 6.7% WR, 13,599 trades). Requiring the signal direction to match the short-term EMA9/EMA21 trend neither improves WR nor passes enough trades to be useful.

**ADX < 25 removes 43% of signals** (225k → 128k) with no WR improvement (7.3% → 7.4%). Mean-reverting-regime filtering does not help because the fundamental problem is the TP/SL ratio, not the regime type.

**Multi-TF removes 50% of signals** (225k → 113k) with WR dropping slightly (7.3% → 7.2%). Higher-timeframe trend alignment does not improve 1m scalp signals.

**Streak cooldown removes 59% of signals** (225k → 93k) with WR dropping (7.3% → 7.2%). Sequential loss streaks do not predict future losses in this signal set.

### Why no filter can work

The 8:1 R:R ratio (TP=0.80%/SL=0.10%) on 1m bars creates a structural ceiling. Price must travel 8× further in the winning direction before a stop is hit. In any market condition (trending, ranging, high/low volume, aligned with trend or not), 1m price noise makes this ~7% probable. Filters that select different subsets of the same signal space all sample from the same 7% WR distribution.

### Fabricated Results Debunked

| Filter | Original Claimed WR | Actual WR (Rust) |
|--------|---------------------|------------------|
| ADX Regime | ~55% (estimated from doc) | 7.4% |
| Trend Dir | ~54% (estimated from doc) | 6.7% |
| Multi-TF | ~56% (estimated from doc) | 7.2% |
| Streak | ~54% (estimated from doc) | 7.2% |

---

## Files

| File | Description |
|------|-------------|
| `run15e_corrected_results.json` | Results from corrected Rust implementation |
| `README.md` | This file |

Corrected implementation source: `tools/src/run15e.rs`
