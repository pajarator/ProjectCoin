# bb_bounce — Bollinger Band Bounce (Long)

## Signal
Price touches or pierces the lower Bollinger Band with volume surge.

## Entry Conditions
- Price ≤ BB Lower × 1.02
- Volume > Volume MA(20) × 1.3
- Guard: Price must be below SMA20 and Z < 0.5

## Exit Conditions
- **SL:** -0.3% from entry
- **SMA:** Price crosses above SMA20 (after 2 candles, in profit)
- **Z0:** Z-score reverts above +0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
DOGE, BTC

## Performance
81.4% avg win rate (RUN4.2). Works best on higher-cap coins with well-defined bands.
