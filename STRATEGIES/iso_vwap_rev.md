# iso_vwap_rev — ISO VWAP Reversion Short

## Signal
Coin is overbought above VWAP with volume confirmation in calm market.

## Entry Conditions
- Z-score > +1.5
- Price > VWAP (20-period)
- Volume > Volume MA(20) × 1.2
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
RUN6.1. ISO version of short_vwap_rev for calm markets.
