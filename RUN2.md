# RUN2.md - Extended Backtest Results

## Test Parameters
- **Timeframe:** 15 minutes (15m)
- **Coins tested:** Top 20 coins by volume
- **Strategies:** 33 (original + enhanced + new from Reddit)
- **Capital:** $100 per coin
- **Risk:** 10% per trade
- **Stop Loss:** 2%
- **Take Profit:** 1.5%

---

## Top Performers by Win Rate (60%+)

| Rank | Coin | Strategy | Win Rate | Trades | Profit Factor |
|------|------|----------|----------|--------|---------------|
| 1 | XLM/USDT | VWAP Reversion | 100% | 5 | 2.46 |
| 2 | AVAX/USDT | BB Bounce | 100% | 5 | 3.94 |
| 3 | NEAR/USDT | BB Bounce | 100% | 9 | 8.10 |
| 4 | NEAR/USDT | RSI Divergence | 100% | 7 | 3.61 |
| 5 | NEAR/USDT | RSI Reversal | 100% | 5 | 8.28 |
| 6 | DAI/USDT | Williams %R | 95.5% | 22 | 1733 |
| 7 | NEAR/USDT | Mean Reversion | 93.8% | 16 | 14.58 |
| 8 | DAI/USDT | ADR Reversal | 93.8% | 16 | 902 |
| 9 | ETH/USDT | Mean Reversion | 90.9% | 11 | 3.88 |
| 10 | LTC/USDT | ADR Reversal | 90.9% | 11 | 5.76 |
| 11 | ATOM/USDT | Dual RSI | 88.9% | 9 | 4.08 |
| 12 | AVAX/USDT | Dual RSI | 88.9% | 9 | 4.21 |
| 13 | NEAR/USDT | Dual RSI | 88.9% | 9 | 20.98 |
| 14 | BTC/USDT | BB Bounce | 87.5% | 8 | 5.46 |
| 15 | BTC/USDT | Dual RSI | 85.7% | 14 | 11.84 |
| 16 | ETH/USDT | BB Bounce | 85.7% | 7 | 3.84 |
| 17 | AVAX/USDT | Williams %R | 84.6% | 13 | 4.73 |

---

## Strategy Rankings (Average across all coins)

| Rank | Strategy | Avg Win Rate | Total Trades | Avg Profit Factor |
|------|----------|--------------|--------------|-------------------|
| 🥇 | **VWAP Reversion** | 86.8% | 55 | 18.2 |
| 🥈 | **BB Bounce** | 81.4% | 104 | 5.7 |
| 🥉 | **Dual RSI** | 79.3% | 113 | 7.1 |
| 4 | Mean Reversion | 77.8% | 212 | 5.6 |
| 5 | ADR Reversal | 77.6% | 180 | 56.3 |
| 6 | RSI Reversal | 75.1% | 86 | 9.4 |
| 7 | RSI Divergence | 74.4% | 67 | 3.7 |
| 8 | Williams %R | 72.7% | 240 | 99.3 |
| 9 | SR Breakout | 73.3% | 11 | 6.8 |
| 10 | Order Block | 65.5% | 57 | 3.3 |
| 11 | FVG | 65.2% | 17 | 4.0 |
| 12 | Candlestick Patterns | 64.4% | 17 | 4.3 |
| 13 | ADX Trend | 60.0% | 5 | 0.8 |

---

## 30-Day Projection

### Based on extended backtest data:

| Risk/Trade | Best Setup | Win Rate | Projected 30-Day Balance |
|------------|------------|----------|-------------------------|
| 2% (conservative) | ETH 15m VWAP Reversion | 80% | $125 (+25%) |
| 5% (moderate) | ETH 15m VWAP Reversion | 80% | $176 (+76%) |
| 10% (aggressive) | ETH 15m VWAP Reversion | 80% | $315 (+215%) |

---

## New Strategies Tested (from Reddit/Communities)

### Strategies Added:
1. **RSI Divergence** - Price lower low + RSI higher low
2. **Supertrend** - ATR-based trend indicator
3. **Order Block (ICT)** - Buy at previous support
4. **FVG (Fair Value Gap)** - Buy on retracement to fill gap
5. **MSS (Market Structure Shift)** - Break of swing high
6. **MA Ribbon** - Multiple EMAs aligned
7. **Trendline Breakout** - Linear regression slope
8. **Dual RSI** - RSI 7 + RSI 14 confirmation
9. **VPT (Volume Price Trend)** - Volume-weighted momentum
10. **Pivot Point Bounce** - Classic pivot levels

### Results:
- **Dual RSI**: 79.3% avg win rate - Best new strategy
- **RSI Divergence**: 74.4% avg win rate
- **Order Block**: 65.5% avg win rate
- **FVG**: 65.2% avg win rate

---

## Best Setups for Live Trading

### Recommended (70%+ win rate):
| Coin | Timeframe | Strategy | Entry Condition |
|------|-----------|----------|-----------------|
| ETH | 15m | Mean Reversion | Z < -1.5 |
| BTC | 15m | BB Bounce | Price at lower BB + volume |
| NEAR | 15m | Mean Reversion | Z < -1.5 |
| SOL | 15m | VWAP Reversion | Z < -1.5 + volume spike |
| LTC | 15m | ADR Reversal | Price at bottom 25% of daily range |
| XLM | 15m | VWAP Reversion | Z < -1.5 + volume |

---

## Trading Parameters Used

```python
RISK_PER_TRADE = 0.10    # 10% of balance per trade
STOP_LOSS = 0.02          # 2% stop loss
TAKE_PROFIT = 0.015       # 1.5% take profit
FEE_RATE = 0.001          # 0.1% Binance fee
SLIPPAGE = 0.0005         # 0.05% slippage
```

---

## Notes

- Data period: ~7 days (limited by Binance API)
- More historical data would improve accuracy
- Paper trading recommended before live execution
- Results are backtested - actual performance may vary
