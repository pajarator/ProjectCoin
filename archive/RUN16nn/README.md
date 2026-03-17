# RUN16: Neural Network Mean Reversion Enhancement

**Date:** 2026-03-16  
**Objective:** Improve mean reversion strategy using a neural network that filters signals

---

## Experiment Design

### Hypothesis
Instead of using hard thresholds (z < -1.5), can a neural network learn which mean reversion signals are more likely to succeed based on contextual features?

### Approach
1. **Input Features:** 8 core indicators (z-score, RSI, BB position, volume ratio, stochastic, momentum, hour, day)
2. **Training:** Walk-forward validation (train on rolling 2-month windows, test on next month)
3. **Model:** Logistic Regression (simpler, more robust than MLP)
4. **Filtering:** Only take high-confidence predictions (P > 0.55 or P < 0.45)

---

## Results Summary

| Metric | Neural Network | Baseline | Delta |
|--------|---------------|----------|-------|
| **Win Rate** | 53.2% | 52.4% | **+0.8 pts** |
| **Signals** | 12,406 | 75,823 | **-84%** |

### Per-Coin Results

| Coin | NN WR% | Base WR% | Delta |
|------|--------|----------|-------|
| ETH | **58.4%** | 52.4% | +6.0 |
| BNB | **56.9%** | 52.4% | +4.5 |
| LTC | **54.9%** | 52.4% | +2.5 |
| DASH | 54.2% | 52.4% | +1.8 |
| XRP | 54.2% | 52.4% | +1.8 |
| DOT | 54.2% | 52.4% | +1.8 |
| ATOM | 53.8% | 52.4% | +1.4 |
| LINK | 52.6% | 52.4% | +0.2 |
| ADA | 52.5% | 52.4% | +0.1 |
| NEAR | 53.3% | 52.4% | +0.9 |
| SOL | 53.2% | 52.4% | +0.8 |
| BTC | 51.6% | 52.4% | -0.8 |
| AVAX | 51.9% | 52.4% | -0.5 |
| UNI | 51.0% | 52.4% | -1.4 |
| DOGE | 49.8% | 52.4% | -2.6 |
| SHIB | 48.9% | 52.4% | -3.5 |

---

## Key Findings

1. **Modest improvement:** NN improves win rate by 0.8 percentage points on average

2. **Huge signal reduction:** 84% fewer signals (more selective trading)

3. **Best performers:**
   - ETH: +6.0 pts improvement (58.4% WR)
   - BNB: +4.5 pts improvement (56.9% WR)
   - LTC: +2.5 pts improvement (54.9% WR)

4. **Worst performers:**
   - SHIB: -3.5 pts (48.9% WR - worse than baseline)
   - DOGE: -2.6 pts (49.8% WR)

5. **Why the improvement is small:**
   - Mean reversion is inherently noisy
   - Markets in 2025 were strongly bullish (trend-following outperformed)
   - NN cannot reliably predict short-term price movements

---

## Conclusions

### Negative Result (Marginal)
The neural network filter provides only a **+0.8 percentage point improvement** in win rate. While statistically meaningful, this is not a transformative improvement.

### Practical Value
- **More selective trading:** 84% signal reduction means fewer trades
- **Better for some coins:** ETH, BNB, LTC show meaningful improvements
- **Worse for others:** SHIB, DOGE perform worse with NN filter

### Recommendation
The current binary mean reversion strategy (z < -1.5) remains the best approach. The NN adds complexity without proportional benefit. However, for coins like ETH and BNB, the NN filter could be optionally enabled.

---

## Files in This Archive

- `run16_nn_backtest.py` — v1: Direct NN prediction (failed)
- `run16_nn_backtest_v2.py` — v2: NN as filter (marginal)
- `run16_nn_backtest_v3.py` — v3: Walk-forward validation (best results)
- `run16_v3_results.json` — Detailed results
- `README.md` — This file

---

## How to Run

```bash
cd /home/scamarena/ProjectCoin
python3 run16_nn_backtest_v3.py
```

Results will be saved to `archive/RUN16/`.
