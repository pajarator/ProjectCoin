# short_adr_rev — Short ADR Reversal

## Signal
Price rises to top 25% of 24-candle range with volume confirmation.

## Entry Conditions
- ADR = High(24) - Low(24)
- Price ≥ High(24) - ADR × 0.25
- Volume > Volume MA(20) × 1.1
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: SHORT (breadth ≥ 50%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
UNI, NEAR, ETH, ATOM, SOL, ALGO, BTC

## Performance
RUN5.2 validated. Most widely used short strategy — 7 of 18 coins.
