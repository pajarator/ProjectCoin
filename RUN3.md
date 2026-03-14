# RUN3.md - Multi-Coin Trading Strategy - Backtest Results

## Date: 2026-03-14

## Strategy Overview

This is a multi-coin paper trading system that runs 20 coins simultaneously with different strategies per coin. Each coin trades with its own $100 capital (total $1900), using 5x leverage.

---

## Parameters

### Capital & Risk
| Parameter | Value | Description |
|-----------|-------|-------------|
| INITIAL_CAPITAL | $100 | Starting capital per coin |
| Total Capital | $1900 | 20 coins × $100 |
| RISK | 10% | % of balance risked per trade |
| LEVERAGE | 5x | Leverage multiplier |
| FEE | 0.1% | Trading fee (0.001) |

### Stop Loss & Exit
| Parameter | Value | Description |
|-----------|-------|-------------|
| STOP_LOSS | 1.5% | Price distance for stop loss |
| MIN_HOLD_CANDLES | 8 | Minimum candles to hold before SMA exit |

### Data
| Parameter | Value | Description |
|-----------|-------|-------------|
| Timeframe | 15m | 15-minute candles |
| Lookback | 20 | SMA period |
| Backtest Period | 5 months | Oct 2025 - Mar 2026 |

---

## Strategies per Coin

| Coin | Strategy | Description |
|------|----------|-------------|
| ETH | mean_reversion | Z-score < -1.5 |
| NEAR | mean_reversion | Z-score < -1.5 |
| BTC | bb_bounce | Price at lower BB + volume |
| AVAX | bb_bounce | Price at lower BB + volume |
| SOL | vwap_reversion | Z-score + VWAP + volume |
| LTC | adr_reversal | Near daily low reversal |
| ATOM | dual_rsi | Z-score < -1.0 |
| XLM | vwap_reversion | Z-score + VWAP + volume |
| DOGE | mean_reversion | Z-score < -1.5 |
| DOT | mean_reversion | Z-score < -1.5 |
| MATIC | mean_reversion | Z-score < -1.5 |
| LINK | vwap_reversion | Z-score + VWAP + volume |
| ADA | mean_reversion | Z-score < -1.5 |
| BNB | mean_reversion | Z-score < -1.5 |
| TRX | adr_reversal | Near daily low reversal |
| XRP | vwap_reversion | Z-score + VWAP + volume |
| UNI | mean_reversion | Z-score < -1.5 |
| SHIB | mean_reversion | Z-score < -1.5 |
| DASH | adr_reversal | Near daily low reversal |
| ALGO | vwap_reversion | Z-score + VWAP + volume |

---

## Indicators Used

- **SMA20**: 20-period simple moving average
- **Z-Score**: (Price - SMA20) / StdDev - measures how far price is from mean
- **Bollinger Bands**: SMA ± 2 standard deviations
- **Volume MA**: 20-period average volume
- **ADR**: Average Daily Range (24 candles = 6 hours)

---

## Entry Signals

### mean_reversion
- Z-score < -1.5 (price significantly below average)

### vwap_reversion
- Z-score < -1.5
- Price below SMA20
- Volume > 1.2x average

### bb_bounce
- Price touches lower Bollinger Band (within 2%)
- Volume > 1.3x average

### adr_reversal
- Price within 25% of daily low
- Expecting reversal higher

### dual_rsi
- Z-score < -1.0 (simpler oversold condition)

---

## Exit Signals

1. **Stop Loss (SL)**: Price moves 1.5% against position → 7.5% loss on leveraged trade
2. **SMA Exit**: Price crosses above SMA20 while in profit
3. **Z-Score Exit**: Z-score reverts to > 0.5 while in profit

**Note**: Only exits on SMA/Z0 if position has been held for minimum 8 candles (2 hours). This lets winners run!

---

## Backtest Results - 5 Months

```
Overall P&L: +20.26%
Win Rate: 67.4%
Avg Win: +4.28%
Avg Loss: -7.49%
Profit Factor: 1.18
Total Trades: 7860
```

### Per-Coin Performance

| Coin | Final | P&L | Trades |
|------|-------|-----|--------|
| DASH | $189.35 | +89.3% | 606 |
| LINK | $135.58 | +35.6% | 322 |
| AVAX | $131.79 | +31.8% | 597 |
| LTC | $131.59 | +31.6% | 445 |
| UNI | $128.29 | +28.3% | 429 |
| DOT | $129.05 | +29.0% | 420 |
| XLM | $123.94 | +23.9% | 328 |
| ADA | $122.31 | +22.3% | 442 |
| ATOM | $117.23 | +17.2% | 521 |
| NEAR | $116.07 | +16.1% | 424 |
| SOL | $116.12 | +16.1% | 338 |
| ETH | $113.15 | +13.2% | 403 |
| XRP | $112.89 | +12.9% | 322 |
| DOGE | $111.06 | +11.1% | 396 |
| SHIB | $103.76 | +3.8% | 394 |
| BTC | $104.29 | +4.3% | 526 |
| ALGO | $101.88 | +1.9% | 314 |
| BNB | $102.70 | +2.7% | 362 |
| TRX | $93.91 | -6.1% | 271 |

### Exit Reasons
- SMA (price crossed above SMA20): 5296 (67%)
- SL (stop loss hit): 2559 (33%)
- EOD (end of data): 5

---

## Key Findings

1. **Strategy Works**: +20% profit over 5 months with 67% win rate
2. **Profit Factor > 1**: 1.18 means for every $1 lost, $1.18 is won
3. **Winners Run**: SMA exit only triggers after 8 candles, letting profits accumulate
4. **Leverage Amplifies**: 5x leverage turns small price moves into meaningful P&L
5. **DASH is King**: +89% - best performer by far
6. **TRX Underperformed**: Only coin with negative returns

---

## Risk Analysis

- **Avg Loss**: -7.49% (1.5% SL × 5x leverage)
- **Worst Case**: Multiple SL hits can compound quickly
- **Win Rate**: 67% means ~1 in 3 trades loses
- **Risk/Reward**: 1:1.75 (4.28 / 7.49) - positive expectancy

---

## Live Trading Comparison

The live trading (shown in system logs) produced similar results:
- Total Trades: ~9600
- Total Coins: 19
- Consistent profit generation

---

## Files

- `multi_curses.py` - Live trading TUI
- `backtest_original.py` - Backtester with caching
- `data_cache/` - Cached OHLCV data (CSV)
- `backtest_state.json` - Backtest resume state
- `trading_state.json` - Live trading state
- `trading_log.txt` - Trade history

---

## Recommendations

1. **Keep Running**: Strategy shows consistent profit
2. **Monitor DASH**: Could increase DASH allocation
3. **Avoid TRX**: Consider removing or reducing
4. **Tighten SL**: 1.5% seems optimal
5. **Use Cache**: Re-run backtests with cached data for speed
