# iso_bb_bounce — ISO Bollinger Band Bounce Short

## Signal
Price at upper Bollinger Band with elevated volume in calm market.

## Entry Conditions
- Price ≥ BB Upper × 0.98
- Volume > Volume MA(20) × 1.3 (vol_mult + 0.1)
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
RUN6.1. ISO version of short_bb_bounce for calm markets.
