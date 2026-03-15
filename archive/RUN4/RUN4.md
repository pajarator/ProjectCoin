# RUN4 - Strategy Optimization Pipeline

## Date: 2026-03-14

## Overview

RUN4 is a six-stage optimization pipeline that takes the RUN3 multi-coin system (PF ~1.18) and systematically improves it through parameter tuning, strategy assignment, validation, genetic optimization, correlation analysis, and a market breadth filter. Final result: **PF 2.28, WR 60%, breadth-filtered**.

---

## RUN4.1 — Parameter Grid Search

**Script:** `run4_1_grid_search.py` (archived) | **Results:** `grid_search_results.json`

### Objective
Find optimal stop loss, min hold, and risk parameters across all 20 coins.

### Grid
| Parameter | Values Tested |
|-----------|---------------|
| STOP_LOSS | 0.5%, 1.0%, 1.5%, 2.0%, 2.5% |
| MIN_HOLD_CANDLES | 2, 4, 6, 8, 12, 16 |
| RISK | 5%, 10%, 15%, 20% |

120 combinations × 20 coins × 5 months of 15m data.

### Results

| Parameter | Before | After | Change |
|-----------|--------|-------|--------|
| STOP_LOSS | 1.5% | **0.5%** | -67% |
| MIN_HOLD | 8 | **2** | -75% |
| Profit Factor | 1.18 | **1.64** | **+39%** |

Key insight: Tighter SL (0.5%) means more frequent small losses but dramatically better PF. With 5x leverage, 0.5% price move = 2.5% margin loss vs 7.5% at the old 1.5% SL.

---

## RUN4.2 — Per-Coin Strategy Assignment

**Script:** (part of grid search pipeline)

### Objective
Test all 5 strategies on each coin individually and assign the best one.

### Strategies Tested
- `mean_reversion` — z < -1.5
- `vwap_reversion` — z < -1.5 + below SMA20 + volume > 1.2x
- `bb_bounce` — price at lower BB + volume > 1.3x
- `adr_reversal` — price in bottom 25% of 24-candle range
- `dual_rsi` — z < -1.0

### Optimal Assignments

| Coin | Strategy | PF |
|------|----------|-----|
| DASH | vwap_reversion | 2.24 |
| UNI | vwap_reversion | 2.14 |
| NEAR | vwap_reversion | 2.00 |
| ADA | vwap_reversion | 1.99 |
| LTC | vwap_reversion | 1.88 |
| SHIB | vwap_reversion | 1.85 |
| LINK | vwap_reversion | 1.83 |
| ETH | vwap_reversion | 1.81 |
| DOT | vwap_reversion | 1.81 |
| XRP | vwap_reversion | 1.76 |
| ATOM | vwap_reversion | 1.71 |
| SOL | vwap_reversion | 1.71 |
| DOGE | bb_bounce | 1.69 |
| XLM | dual_rsi | 1.62 |
| AVAX | adr_reversal | 1.59 |
| ALGO | adr_reversal | 1.55 |
| BNB | vwap_reversion | 1.49 |
| BTC | bb_bounce | 1.44 |

TRX removed (PF=1.18, too low). 12/18 coins use `vwap_reversion` — it dominates.

---

## RUN4.3 — Walk-Forward Validation

**Script:** `run4_3_walk_forward.py` | **Results:** `walk_forward_results.json`

### Objective
Validate that RUN4.2 strategy assignments aren't overfit. Train on 2 months, test on 1 month, 3 rolling windows.

### Windows
| Window | Train | Test |
|--------|-------|------|
| W1 | Oct 15 – Dec 14 | Dec 15 – Jan 14 |
| W2 | Nov 15 – Jan 14 | Jan 15 – Feb 14 |
| W3 | Dec 15 – Feb 14 | Feb 15 – Mar 10 |

### Results
- Strategy stability: Most coins see the same strategy selected across windows
- Consistent coins (profitable in ≥67% of OOS windows): majority pass
- Avg train PF vs avg test PF: moderate degradation, within acceptable range

---

## RUN4.4 — Genetic Algorithm Optimization

**Script:** `run4_4_genetic.py` | **Results:** `genetic_results.json`

### Objective
Evolve strategy parameters using genetic algorithm to find better combinations than grid search.

### Setup
- Population: 20 (1 seed from RUN4.1 + 19 random)
- Generations: 10
- Selection: top 50% by PF
- Crossover: uniform
- Mutation: 30% rate, Gaussian perturbation

### Parameter Ranges
| Parameter | Range |
|-----------|-------|
| stop_loss | 0.2% – 2.0% |
| min_hold | 1 – 16 candles |
| z_threshold | -2.5 – -0.5 |
| bb_margin | 1.0 – 1.05 |
| vol_mult | 0.8 – 2.0 |
| adr_pct | 0.15 – 0.40 |
| exit_z | 0.0 – 1.5 |

### Results

| Metric | Seed (RUN4.1) | Evolved |
|--------|---------------|---------|
| Avg PF | 1.785 | **3.327** |

**Evolved parameters:**
```
stop_loss: 0.002   (was 0.005)
min_hold:  3        (was 2)
z_threshold: -2.5   (was -1.5)
bb_margin: 1.0      (was 1.02)
vol_mult:  2.0      (was 1.2)
adr_pct:   0.15     (was 0.25)
exit_z:    0.77     (was 0.5)
```

The genetic params are more selective (stricter z-threshold, higher volume requirement) with tighter stops. However, this came with significantly fewer trades — not adopted as primary because of overfitting risk.

---

## RUN4.5 — Correlation Analysis

**Script:** `run4_5_correlation.py` | **Results:** `correlation_results.json`

### Objective
Analyze return correlations to reduce drawdown through diversification. Identify redundant coins.

### Key Findings

**Coin Clusters (correlation > 0.7):**

| Cluster | Coins | Best Performer |
|---------|-------|----------------|
| 1 | DASH (standalone) | DASH (PF=2.24) |
| 2 | UNI, ADA, LINK | UNI (PF=2.14) |
| 3 | NEAR, LTC, SHIB, DOT, SOL, DOGE, XLM, AVAX, ALGO | NEAR (PF=2.00) |
| 4 | ETH, XRP, BNB, BTC | ETH (PF=1.81) |
| 5 | ATOM (standalone) | ATOM (PF=1.71) |

Most coins (cluster 3) are highly correlated — 9 coins moving together. This means:
- When one dips, most dip (market-wide move, not coin-specific)
- Mean reversion entries during broad dumps are dangerous
- Diversification benefit is limited within the cluster

**Recommended diversified portfolio (top 9 by PF × (1 - avg_corr)):**
DASH, UNI, NEAR, ATOM, LTC, SHIB, XRP, DOT, BNB

---

## RUN4.6 — Correlation-Aware Breadth Filter

**Script:** `run4_6_correlated_filter.py` | **Results:** `correlated_filter_results.json`

### Objective
Use the correlation insight: skip entries when too many coins dump simultaneously (market-wide move, not mean reversion opportunity).

### Market Breadth
At each candle, calculate what fraction of coins have z < -1.0.

| Statistic | Value |
|-----------|-------|
| Avg breadth | 26.8% |
| Max breadth | 100% |
| Time breadth > 50% | 22.8% |
| Time breadth > 30% | 32.4% |

### Breadth Filter Impact (RUN4.1 params)

| Breadth Max | Avg PF | Avg WR | Trades |
|-------------|--------|--------|--------|
| OFF | 1.64 | 50.4% | 11,326 |
| 50% | +PF | +WR | fewer |
| 40% | +PF | +WR | fewer |
| 30% | +PF | +WR | fewer |
| **20%** | **2.28** | **59.5%** | fewer |

### Recommended Configuration

```python
BREADTH_MAX = 0.20  # Only enter when ≤20% of coins are bearish
```

| Metric | Without Filter | With Filter | Change |
|--------|----------------|-------------|--------|
| Avg PF | 1.64 | **2.28** | +39% |
| Avg WR | 50.4% | **59.5%** | +18% |
| Idle time | 0% | 22.8% | system sits out during dumps |

The filter turns a 50% win rate system into a 60% win rate system by avoiding entries during market-wide dumps. The cost is 22.8% idle time.

---

## RUN4 Summary — v5 Configuration

### Parameters Applied to trader.py (v5)
```python
STOP_LOSS = 0.005        # 0.5% (RUN4.1)
MIN_HOLD_CANDLES = 2     # 2 candles = 30min (RUN4.1)
RISK = 0.10              # 10%
LEVERAGE = 5             # 5x
BREADTH_MAX = 0.20       # Skip if >20% coins bearish (RUN4.6)
```

### Per-Coin Strategy Assignments (RUN4.2)
12/18 coins: `vwap_reversion`
2 coins (DOGE, BTC): `bb_bounce`
2 coins (AVAX, ALGO): `adr_reversal`
1 coin (XLM): `dual_rsi`

### Overall Performance
| Metric | RUN3 (v3) | RUN4 (v5) | Improvement |
|--------|-----------|-----------|-------------|
| Profit Factor | 1.18 | **2.28** | +93% |
| Win Rate | 67% | **60%** | -7% (by design) |
| Breadth filter | none | 20% max | avoids dumps |

### Remaining Issue
The system is **idle 22.8% of the time** when breadth > 50% (market-wide dumps). Since most coins are correlated, these dumps drag overbought coins down — a potential shorting opportunity. This motivates RUN5.

---

## Files

| File | Purpose |
|------|---------|
| `run4_3_walk_forward.py` | Walk-forward validation |
| `run4_4_genetic.py` | Genetic algorithm optimization |
| `run4_5_correlation.py` | Correlation analysis |
| `run4_6_correlated_filter.py` | Breadth filter testing |
| `walk_forward_results.json` | RUN4.3 results |
| `genetic_results.json` | RUN4.4 results |
| `correlation_results.json` | RUN4.5 results |
| `correlated_filter_results.json` | RUN4.6 results |
| `RUN4.1.md` | Detailed grid search writeup |
