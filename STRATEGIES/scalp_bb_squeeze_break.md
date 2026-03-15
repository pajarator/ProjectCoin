# scalp_bb_squeeze_break — Scalp BB Squeeze Breakout

## Signal
Bollinger Band squeeze followed by a breakout with volume on 1m candles.

## Entry Conditions
- BB Width < BB Width Avg(20) × 0.4 (tight squeeze)
- Volume > Volume MA(20) × 2.0
- **Long:** Price breaks above BB Upper
- **Short:** Price breaks below BB Lower

## Exit Conditions
- **TP:** +0.2% from entry
- **SL:** -0.1% from entry

## Timeframe
1m

## Coins
All 18 (universal scalp overlay)

## Risk
5% per trade (vs 10% for regime), 5x leverage

## Performance (RUN9 live, first 13h)
- 0 TP / 1 SL — 0% win rate (1 trade only)
- Net P&L: -$0.05

## Notes
Barely fires due to the restrictive 0.4 squeeze factor. The condition requires extremely narrow bands AND a volume surge AND a band breakout all at once. May need more time to generate meaningful sample size.
