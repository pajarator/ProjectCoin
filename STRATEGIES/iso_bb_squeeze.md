# iso_bb_squeeze — ISO Bollinger Band Squeeze Short

## Signal
Price at upper band during a squeeze (narrow bands) in calm market — breakout fade.

## Entry Conditions
- Price ≥ BB Upper × 0.98
- BB Width < BB Width Avg(20) × 0.8 (squeeze)
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: LONG or ISO_SHORT (breadth ≤ 20%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
None currently assigned (available as fallback)

## Performance
RUN6.1. Contrarian — fades false breakouts during low-volatility squeezes.
