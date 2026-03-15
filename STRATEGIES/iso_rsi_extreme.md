# iso_rsi_extreme — ISO RSI Extreme Short

## Signal
Coin RSI is extremely overbought while the broader market RSI is calm.

## Entry Conditions
- RSI > 75
- Market Avg RSI < 55 (market is calm)
- Guard: Price must be above SMA20 and Z > -0.5
- Market mode: LONG or ISO_SHORT (breadth ≤ 20%)

## Exit Conditions
- **SL:** -0.3% (price rises above entry)
- **SMA:** Price drops below SMA20 (after 2 candles, in profit)
- **Z0:** Z-score drops below -0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
NEAR (PF=15.70), LTC (PF=10.01), SHIB (PF=4.46), ETH (PF=11.71), XRP (PF=3.29), SOL (PF=9.22), ALGO (PF=12.18), BTC (PF=6.18)

## Performance
RUN6.1. Highest PF strategy overall — several coins above PF 10. Works because RSI extremes in calm markets reliably revert. Most assigned ISO strategy (8 coins).
