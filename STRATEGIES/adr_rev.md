# adr_rev — ADR Reversal (Long)

## Signal
Price falls to bottom 25% of 24-candle range with volume confirmation.

## Entry Conditions
- ADR = High(24) - Low(24)
- Price ≤ Low(24) + ADR × 0.25
- Volume > Volume MA(20) × 1.1
- Guard: Price must be below SMA20 and Z < 0.5

## Exit Conditions
- **SL:** -0.3% from entry
- **SMA:** Price crosses above SMA20 (after 2 candles, in profit)
- **Z0:** Z-score reverts above +0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
AVAX, ALGO

## Performance
77.6% avg win rate (RUN4.2). Range-based reversal for coins with clear intraday ranges.
