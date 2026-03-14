# PLAN.md - Strategy Optimization Plan

## Objective
Discover better strategy combinations through systematic testing and optimization.

---

## Optimization Methods

### 1. Parameter Grid Search (RUN4.1)
Test systematic combinations of key parameters:
- STOP_LOSS: [0.5%, 1.0%, 1.5%, 2.0%, 2.5%]
- MIN_HOLD_CANDLES: [2, 4, 6, 8, 12, 16]
- RISK: [5%, 10%, 15%, 20%]
- Total combinations: 5 × 6 × 4 = 120

Use cached data (5 months).

### 2. Per-Coin Strategy Optimization (RUN4.2)
Test each strategy on each coin to find best combination:
- 20 coins × 5 strategies = 100 combinations
- Find optimal strategy per coin (not one-size-fits-all)

Strategies to test:
- mean_reversion
- vwap_reversion
- bb_bounce
- adr_reversal
- dual_rsi

### 3. Walk-Forward Analysis (RUN4.3)
Avoid overfitting by testing on rolling windows:
- Train: 3 months, Test: 1 month
- Rolling 5 windows (5 months total)
- Validate strategy generalizes

### 4. Genetic Algorithm (RUN4.4)
Evolve strategy parameters:
- Population: 20 strategies
- Generations: 10
- Mutations: vary SL, MIN_HOLD, RSI thresholds, etc.
- Selection: top 50% by profit factor

### 5. Correlation Analysis (RUN4.5)
Reduce drawdown through diversification:
- Analyze correlation between coin returns
- Find optimal allocation (not equal weight)
- Identify redundant strategies

---

## Execution Order

1. RUN4.1 - Parameter Grid Search
2. RUN4.2 - Per-Coin Optimization  
3. RUN4.3 - Walk-Forward Analysis
4. RUN4.4 - Genetic Algorithm
5. RUN4.5 - Correlation & Allocation

## Success Criteria

- Profit Factor > 1.18 (current)
- Win Rate > 65%
- Max Drawdown < 20%
- Consistent across walk-forward windows
