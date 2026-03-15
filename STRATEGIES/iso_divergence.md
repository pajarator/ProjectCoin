# iso_divergence — ISO Divergence Short

## Signal
Coin is overbought while BTC is flat or down — divergence suggests the coin will revert.

## Entry Conditions
- Z-score > +1.5
- BTC Z-score < 0 (BTC flat/down)
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: LONG or ISO_SHORT (breadth ≤ 20%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
DASH (PF=3.75), ADA (PF=2.64), DOGE (PF=5.30), BNB (PF=2.60)

## Performance
RUN6.1. Exploits BTC-altcoin divergence in calm markets. Works best on coins that tend to overshoot when BTC is flat.
