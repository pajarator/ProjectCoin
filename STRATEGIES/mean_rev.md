# mean_rev — Mean Reversion (Long)

## Signal
Pure z-score mean reversion — enters when price is significantly below the 20-period mean.

## Entry Conditions
- Z-score < -1.5
- Guard: Price must be below SMA20 and Z < 0.5

## Exit Conditions
- **SL:** -0.3% from entry
- **SMA:** Price crosses above SMA20 (after 2 candles, in profit)
- **Z0:** Z-score reverts above +0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
None currently assigned (available as fallback)

## Performance
72-87% win rate across coins (RUN1). Simplest long strategy — no volume or secondary indicator required. Best overall strategy from RUN1 discovery.
