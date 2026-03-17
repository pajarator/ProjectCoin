# RUN15d: 10 Ideas to Improve Scalping

**Date:** 2026-03-16

Based on trading research and common scalp trading techniques:

---

## 10 Ideas to Test

### 1. Time-of-Day Filter
**Idea:** Only scalp during specific hours (e.g., when markets are most liquid)
- Scalp only during NYSE open (9:30-11:30 AM ET) or peak volume hours
- Avoid low-liquidity periods (pre-market, late night)

### 2. Market Regime Filter
**Idea:** Only scalp in ranging markets, not trending
- Use ADX < 25 as range filter
- Avoid scalping when strong trend detected (ADX > 40)

### 3. Liquidity Filter
**Idea:** Filter by spread and volume
- Only trade when bid-ask spread < 0.05%
- Minimum volume threshold (e.g., > $1M 24h volume)

### 4. Correlation Filter
**Idea:** Trade with market direction
- Only long when BTC/ETH is going up
- Use BTC 15m trend as market bias

### 5. Multi-Timeframe Confirmation
**Idea:** 1m signal confirmed by 5m or 15m
- 1m gives entry, 5m confirms direction
- Reduces false signals

### 6. TWAP Orders
**Idea:** Split orders over time to reduce slippage
- Enter in 3 chunks over 1 minute
- Reduces market impact

### 7. Dynamic Position Sizing
**Idea:** Size based on confidence (NN probability)
- High prob = larger position
- Low prob = smaller position

### 8. Streak Filter
**Idea:** After X consecutive losses, pause
- Cooldown after 3 losing trades
- Prevents revenge trading

### 9. Best Coin Selection
**Idea:** Only trade coins with best recent performance
- Rank by recent WR (last 100 trades)
- Only trade top 5 coins

### 10. Transaction Cost Filter
**Idea:** Factor fees into signal selection
- Require higher probability when fees are higher
- Dynamic threshold based on fee tier

---

## Priority to Test

| # | Idea | Expected Impact | Complexity |
|---|------|----------------|------------|
| 4 | Correlation Filter | High | Low |
| 2 | Regime Filter | High | Low |
| 1 | Time-of-Day Filter | Medium | Low |
| 9 | Best Coin Selection | Medium | Low |
| 6 | Multi-Timeframe | Medium | Medium |

---

## Next Steps

Pick 3-5 of these to backtest in RUN15d. Start with the low-complexity, high-expected-impact ones:
1. Correlation Filter (trade with BTC direction)
2. Regime Filter (ADX-based)
3. Time-of-Day Filter
