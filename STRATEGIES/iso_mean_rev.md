# iso_mean_rev — ISO Mean Reversion Short

## Signal
Pure z-score overbought in calm market.

## Entry Conditions
- Z-score > +1.5
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
RUN6.1. Simplest ISO short — no market context required.
