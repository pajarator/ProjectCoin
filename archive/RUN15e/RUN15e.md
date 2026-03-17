# RUN15e: Scalp Improvement - Final Conclusions

**Date:** 2026-03-16  
**Experiment:** Test 7 ideas to improve 1m scalp trading

---

## Executive Summary

**WINNER: Liquidity Filter + NN**

| Metric | Value |
|--------|-------|
| Win Rate | **57.1%** |
| Improvement | **+7.6 pts** over baseline |
| Signals | 5,812 (reduced from 225,695) |

---

## Background

Ran multiple experiments to improve scalp trading on 1m timeframe:

- **RUN15b:** Bayesian vs Binary — Negative result
- **RUN16:** NN for mean reversion — Marginal improvement
- **RUN17:** NN for scalp — +7.2 pts improvement
- **RUN15c:** 5 model approaches — No improvement
- **RUN15d:** 7 filter ideas — **Positive result**

---

## What Was Tested

### Filters Tested (RUN15d)

1. **Time-of-Day Filter** — Trade only during peak hours (14:00-21:00 UTC)
2. **Regime Filter (ADX)** — Only range markets (ADX < 25)
3. **Liquidity Filter** — Only vol_ratio > 2.0
4. **Correlation Filter** — Trade with BTC direction
5. **Multi-Timeframe** — EMA alignment
6. **Dynamic Sizing** — Size by confidence
7. **Streak Filter** — Cooldown after losses

### Results Summary

| Approach | Win Rate | Improvement |
|----------|----------|-------------|
| Liquidity + NN | **57.1%** | **+7.6 pts** ✅ |
| Baseline + NN | 55.3% | +5.8 pts |
| Time-of-Day + NN | 54.7% | +5.2 pts |
| Liquidity | 51.4% | +1.9 pts |
| Time-of-Day | 49.9% | +0.4 pts |
| Baseline | 49.5% | — |

---

## Key Findings

### ✅ What Works

1. **Liquidity Filter (vol_ratio > 2.0)**
   - Filters out low-quality signals in illiquid periods
   - Improves WR by ~2 pts standalone

2. **NN Filter (threshold 0.55)**
   - Adds ~5-6 pts improvement
   - Learns which signal contexts are favorable

3. **Combined: Liquidity + NN**
   - Best result: 57.1% WR (+7.6 pts)

### ❌ What Doesn't Work

1. **Regime Filter (ADX < 25)**
   - Surprisingly makes things worse
   - Scalping works in all market conditions

2. **Multi-Timeframe**
   - Too restrictive, loses too many signals
   - Reduces signals too much

3. **Correlation, Dynamic Sizing, Streak Filters**
   - No measurable impact

---

## Implementation

```python
def should_take_scalp(vol_ratio, nn_probability):
    # Filter 1: Liquidity
    if vol_ratio < 2.0:
        return False, "too illiquid"
    
    # Filter 2: NN confidence
    if nn_probability < 0.55:
        return False, "low confidence"
    
    return True, "take trade"
```

---

## Why It Works

1. **Liquidity filter** removes noise from low-volume periods where price moves are more random

2. **NN learns context** — not all scalp signals are equal:
   - Some RSI/stoch extremes are more likely to reverse
   - Volume spikes at certain times are more reliable

3. **Combined effect** — filtering twice (liquidity + NN) creates high-quality subset

---

## Risk Considerations

- **Fewer signals** — Down from 225K to 5.8K signals/year
- **Overfitting risk** — NN trained on 6 months, tested on 6 months
- **Market regime** — Results from 2025 data (bull market)

---

## Files in This Archive

- `run15d_test_filters.py` — Main backtest script
- `run15d_full_year.py` — Full year test
- `run15d_final.py` — Corrected PnL
- `run15d_results.json` — Results data
- `ideas.md` — Original 10 ideas list
- `README.md` — This file

---

## Conclusion

**INTEGRATE: Liquidity Filter + NN**

The combination of liquidity filtering and NN signal selection improves scalp win rate from 49.5% to 57.1% (+7.6 pts). This is a significant improvement that should be added to the production system.

### Next Steps

1. Implement liquidity filter in `coinclaw/src/`
2. Train NN model on historical data
3. Add to scalp entry logic
4. Monitor in production
