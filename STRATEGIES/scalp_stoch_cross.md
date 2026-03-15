# scalp_stoch_cross — Scalp Stochastic Cross

## Signal
Stochastic %K crosses %D at extreme levels on 1m candles.

## Entry Conditions
- **Long:** %K crosses above %D, both below 5 (oversold)
- **Short:** %K crosses below %D, both above 95 (overbought)
- Stochastic: 14-period %K, 3-period %D smoothing

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
- 81 TP / 96 SL — 45.8% win rate
- Net P&L: +$1.41
- Dominant scalp signal — 78% of all scalp trades

## Notes
Most active scalp strategy by far. The extreme threshold (5/95) keeps it selective. Net positive despite sub-50% WR because wins ($0.05-0.06) are larger than losses ($0.03).
