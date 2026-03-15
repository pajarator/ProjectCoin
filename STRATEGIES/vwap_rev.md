# vwap_rev — VWAP Reversion (Long)

## Signal
Price dips below VWAP with strong z-score and volume confirmation.

## Entry Conditions
- Z-score < -1.5
- Price < VWAP (20-period)
- Volume > Volume MA(20) × 1.2
- Guard: Price must be below SMA20 and Z < 0.5

## Exit Conditions
- **SL:** -0.3% from entry
- **SMA:** Price crosses above SMA20 (after 2 candles, in profit)
- **Z0:** Z-score reverts above +0.5 (after 2 candles, in profit)

## Timeframe
15m

## Coins
DASH, UNI, NEAR, ADA, LTC, SHIB, LINK, ETH, DOT, XRP, ATOM, SOL, BNB

## Performance
86.8% avg win rate (RUN4.2). Most widely used long strategy — 13 of 18 coins.
