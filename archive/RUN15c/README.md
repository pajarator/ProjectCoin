# RUN15c: ML Model Comparison for Scalp Signal Filter

**Date:** 2026-03-16
**Objective:** Test whether more complex ML models (GBM, RF) or more features beat Logistic Regression as a scalp signal filter.

---

## Experiment Design

### Question
Original RUN15c asked: "Can GBM, RF, or 17 features beat Logistic Regression at filtering vol_spike_rev + stoch_cross signals?"

### Models Tested (separate long/short model per coin)
| Model | Features | Params |
|-------|----------|--------|
| LR(9)  | 9 basic | L2 λ=10, lr=0.05, 500 iterations |
| LR(17) | 17 extended | same |
| RF     | 9 basic (unscaled) | 50 trees, depth=5, sqrt(9)=3 feats/split |
| GBM    | 9 basic (unscaled) | 50 estimators, lr=0.1, depth=3 |

### Extended Features (17 total = basic 9 + 8 new)
Basic 9: `rsi14, vol_ratio, stoch_k, stoch_d, stoch_cross, bb_pos, roc3, body3, hour`

New 8: `ema9_rel, ema21_rel, ema_diff, macd_hist, atr_pct, hl_pct, candle_str, vol_steep`

### Trade Parameters
- TP=0.80%, SL=0.10%, fee=0.10%/side, slip=0.05%/side
- Max hold: 60 bars (1 hour)
- Breakeven WR required: **≈44%**

### Methodology
- Full 1-year 1m data (~430k–525k bars per coin)
- 50/50 chronological split (train first half, test second half)
- Single pre-specified threshold: 0.55 (no test-set threshold selection)
- TP/SL simulated on OHLCV bars (pessimistic: SL checked first if both hit same bar)

---

## Corrected Results

### Per-Coin Results

| Coin | Binary WR | Binary Trades | LR9 Trades | LR17 Trades | RF Trades | GBM Trades |
|------|-----------|--------------|------------|-------------|-----------|------------|
| BTC  | 6.4%  | 25,576 | 0 | 0 | 0  | 2  |
| ETH  | 7.5%  | 21,803 | 0 | 0 | 0  | 16 |
| BNB  | 7.7%  | 24,498 | 0 | 0 | 0  | 0  |
| SOL  | 7.9%  | 22,382 | 0 | 0 | 8  | 41 |
| ADA  | 8.0%  | 22,614 | 0 | 0 | 0  | 3  |
| XRP  | 7.7%  | 22,986 | 0 | 0 | 0  | 5  |
| DOGE | 7.3%  | 22,597 | 0 | 0 | 0  | 2  |
| LTC  | 7.4%  | 23,130 | 0 | 0 | 0  | 14 |
| LINK | 5.9%  | 17,761 | 0 | 0 | 6  | 13 |
| DOT  | 7.7%  | 22,233 | 0 | 0 | 0  | 11 |

### Summary

| Model | Avg WR | Avg PF | Total Trades | vs Binary |
|-------|--------|--------|-------------|-----------|
| **Binary** | **7.3%** | **0.092** | **225,580** | — |
| LR(9)   | 0.0% | 0.000 | 0       | 0 trades at threshold |
| LR(17)  | 0.0% | 0.000 | 0       | 0 trades at threshold |
| RF      | 1.7% | 0.031 | 14      | 14 noise trades |
| GBM     | 2.9% | 0.032 | 107     | 107 noise trades (still losing) |

---

## Conclusions

### Strong negative result

**All 4 models fail.** The fundamental problem identified in RUN15b persists: binary WR = 7.3% is 37 percentage points below the 44% breakeven. No ML model can fix a signal with no underlying edge.

**LR(9) and LR(17): 0 trades.** Both models correctly learn the dominant class (93% losses) and assign P(win) ≈ 7% to every signal — far below the 0.55 threshold. No signal passes the filter.

**RF: 14 noise trades at 1.7% WR.** The bootstrap sampling can occasionally produce tree splits that assign slightly higher probability to a handful of test signals. These 14 trades at 1.7% WR are random noise — the trades that do pass are losing at an even higher rate than binary.

**GBM: 107 noise trades at 2.9% WR.** GBM fits residuals sequentially. Starting from a base prediction of 7.3% (class mean), early trees reduce residuals for the training wins. The cumulative predictions for a small number of test samples drift above 0.55 by the end of training. These 107 trades still achieve only 2.9% WR — far below breakeven.

### Why the original "LogReg wins" conclusion was accidentally correct

The original RUN15c found all 5 approaches clustered around 53% WR. This was true in the 3-bar-lookahead frame (coin-flip) — within that frame, model complexity doesn't matter because the target is essentially random. The corrected experiment confirms this insight (model complexity doesn't help) but for the correct reason: the underlying signals have no real edge, so no model can learn to select the winners.

### Bugs in original implementation (inherited from RUN15b)

1. **3-bar lookahead target** — `future_return = c.pct_change(3).shift(-3)` (coin-flip ~49%)
2. **21-day window** — `df.tail(30000)`; only ~10 days train, ~10 days test
3. **Threshold selected on test data** — compared [0.50, 0.55, 0.60] on test set
4. **Mixed-threshold aggregation** — WR reported as `sum(wins across all thresholds) / sum(signals across all thresholds)` — nonsensical average
5. **No fees or slippage**

### Recommendation

**No COINCLAW changes.** The question "which ML model is best for scalp filtering?" is moot when the underlying signals (vol_spike_rev + stoch_cross on 1m with TP=0.80%/SL=0.10%) have 7.3% actual WR. Model selection cannot solve a fundamentally unviable signal.

---

## Files

| File | Description |
|------|-------------|
| `run15c_scalp_improvements.py` | Original buggy Python implementation |
| `run15c_scalp_improvements2.py` | Second original Python script |
| `run15c_results.json` | Results from original buggy run |
| `run15c_corrected_results.json` | Results from corrected Rust implementation |
| `README.md` | This file (updated with corrected conclusions) |

Corrected implementation source: `tools/src/run15c.rs`
