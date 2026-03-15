# short_mean_rev — Short Mean Reversion

## Signal
Pure z-score mean reversion short — enters when price is significantly above the 20-period mean during market dumps.

## Entry Conditions
- Z-score > +1.5
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: SHORT (breadth ≥ 50%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
DASH, LTC, XLM

## Performance
RUN5.2 validated. Simplest short strategy — mirrors mean_rev for the short side.
