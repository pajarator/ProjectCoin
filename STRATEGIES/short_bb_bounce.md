# short_bb_bounce — Short Bollinger Band Bounce

## Signal
Price touches or pierces the upper Bollinger Band with volume surge during market dumps.

## Entry Conditions
- Price ≥ BB Upper × 0.98
- Volume > Volume MA(20) × 1.3
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: SHORT (breadth ≥ 50%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
ADA, LINK, XRP, DOGE, AVAX

## Performance
RUN5.2 validated. Mirrors bb_bounce for the short side.
