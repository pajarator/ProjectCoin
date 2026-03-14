# ProjectCoin - Crypto Trading Strategy Discovery

Mission: Find a day trading strategy with ≥70% success rate.

## Structure

```
├── strategies/     # Trading strategy implementations
├── data/          # Raw and processed data
├── backtests/     # Backtest results
├── notebooks/     # Analysis notebooks
└── main.py        # Entry point
```

## Approach

1. **Data Collection** - Fetch historical crypto data (CCXT)
2. **Strategy Library** - Implement multiple indicator-based strategies
3. **Backtesting Engine** - Test strategies across various conditions
4. **Optimization** - Tune parameters for max win rate
5. **Validation** - Walk-forward testing to avoid overfitting

## Targets

- Timeframe: Day trading (1m-1h candles)
- Win rate: ≥70%
- Any crypto pair that shows promise
