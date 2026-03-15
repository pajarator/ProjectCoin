# iso_relative_z — ISO Relative Z-Score Short

## Signal
Coin's z-score is an outlier vs the market average — significantly more overbought than peers.

## Entry Conditions
- Z-score > Market Avg Z + 1.5 (z_spread)
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: LONG or ISO_SHORT (breadth ≤ 20%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
UNI (PF=3.73), LINK (PF=10.93), DOT (PF=2.85), ATOM (PF=2.52), XLM (PF=7.80), AVAX (PF=5.36)

## Performance
RUN6.1. Relative-value strategy — doesn't need the coin to be absolutely overbought, just overbought vs peers. Most widely used ISO strategy (6 coins).
