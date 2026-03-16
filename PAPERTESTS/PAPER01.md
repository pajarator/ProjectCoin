# PAPER01 — COINCLAW v9 Paper Trading Results

## Overview

| Metric | Value |
|--------|-------|
| Version | COINCLAW v9 |
| Period | 2026-03-14 21:26 → 2026-03-15 18:10 (20.7 hours) |
| Starting Capital | $100.00 per coin (18 coins, $1,800 total) |
| Final Balance | $1,803.88 |
| Net P&L | **+$3.88 (+0.22%)** |
| Total Trades | 478 (451 scalp + 27 regime) |

## Summary

COINCLAW v9 was net positive over ~21 hours of live paper trading, but nearly all profit came from **regime trades**. Scalps were barely breakeven despite high volume.

| Type | Trades | Win Rate | P&L | Profit Factor | Avg Win | Avg Loss |
|------|--------|----------|-----|---------------|---------|----------|
| **Scalp** | 451 | 40.4% | +$0.72 | 1.07 | $0.058 | -$0.036 |
| **Regime** | 27 | 55.6% | +$2.58 | 2.24 | $0.311 | -$0.173 |
| **Combined** | 478 | 41.2% | +$3.30 | — | — | — |

Scalps generated 94% of trade volume but only 22% of profit. Regime trades were the real earner with 2.24x profit factor.

## Scalp Strategy Breakdown

| Strategy | Trades | Win Rate | P&L | Profit Factor |
|----------|--------|----------|-----|---------------|
| scalp_stoch_cross | 348 | 41.4% | +$0.91 | 1.12 |
| scalp_vol_spike_rev | 98 | 35.7% | -$0.31 | 0.86 |
| scalp_bb_squeeze_break | 5 | 60.0% | +$0.12 | 2.50 |

- **stoch_cross** carried the scalp P&L — positive but thin margin (PF 1.12)
- **vol_spike_rev** was net negative — 35.7% WR at TP=0.20%/SL=0.10% is unprofitable
- **bb_squeeze_break** had only 5 trades — too few to draw conclusions

## Scalp Exit Analysis

All exits were either TP or SL — no signal exits or timeouts:

| Exit Reason | Count | P&L |
|-------------|-------|-----|
| TP (take profit) | 182 | +$10.51 |
| SL (stop loss) | 269 | -$9.79 |

The system won $10.51 on 182 TP hits and lost $9.79 on 269 SL hits. Net +$0.72. This confirms the core problem identified in RUN10: at TP=0.20%/SL=0.10%, you need >60% WR to profit meaningfully, and the actual WR was 40.4%.

## Regime Exit Analysis

| Exit Reason | Count | Wins | P&L |
|-------------|-------|------|-----|
| SMA crossback | 15 | 15 (100%) | +$4.66 |
| Stop loss | 12 | 0 (0%) | -$2.08 |

SMA crossback exits were perfect — every single one was profitable. Regime SL losses were larger per-trade ($0.17 avg) than scalp SL losses ($0.036 avg) due to 10% position size vs 5%.

## Scalp Direction Analysis

| Direction | Trades | Win Rate | P&L |
|-----------|--------|----------|-----|
| Long | 229 | 41.0% | +$0.80 |
| Short | 222 | 39.6% | -$0.08 |

Longs slightly outperformed shorts. Both directions had similar trade counts, suggesting no strong directional bias in market conditions.

## Per-Coin Results

| Coin | Balance | P&L | Trades | WR% | Scalp P&L | Regime P&L |
|------|---------|-----|--------|-----|-----------|------------|
| DASH | $101.32 | +$1.32 | 20 | 45.0% | +$0.03 | +$1.27 |
| NEAR | $100.96 | +$0.96 | 23 | 43.5% | -$0.07 | +$1.02 |
| XLM | $100.95 | +$0.95 | 26 | 53.8% | +$0.38 | +$0.56 |
| XRP | $100.82 | +$0.82 | 38 | 55.3% | +$0.52 | +$0.25 |
| ATOM | $100.51 | +$0.51 | 28 | 50.0% | +$0.22 | +$0.25 |
| ADA | $100.36 | +$0.36 | 19 | 42.1% | +$0.05 | +$0.30 |
| LTC | $100.23 | +$0.23 | 18 | 50.0% | +$0.19 | $0.00 |
| DOGE | $100.20 | +$0.20 | 28 | 42.9% | +$0.11 | +$0.08 |
| SHIB | $100.17 | +$0.17 | 19 | 42.1% | +$0.23 | $0.00 |
| DOT | $100.05 | +$0.05 | 28 | 39.3% | -$0.11 | +$0.09 |
| AVAX | $100.03 | +$0.03 | 29 | 44.8% | +$0.16 | -$0.20 |
| BTC | $100.01 | +$0.01 | 30 | 46.7% | +$0.11 | -$0.13 |
| BNB | $99.78 | -$0.22 | 35 | 31.4% | -$0.25 | $0.00 |
| ETH | $99.84 | -$0.16 | 21 | 33.3% | -$0.18 | $0.00 |
| LINK | $99.92 | -$0.08 | 25 | 36.0% | +$0.00 | -$0.16 |
| SOL | $99.85 | -$0.15 | 17 | 35.3% | -$0.17 | $0.00 |
| UNI | $99.34 | -$0.66 | 31 | 25.8% | -$0.26 | -$0.41 |
| ALGO | $99.53 | -$0.47 | 43 | 30.2% | -$0.24 | -$0.34 |

**Best performers:** DASH (+1.32%), NEAR (+0.96%), XLM (+0.95%) — all driven by regime wins.
**Worst performers:** UNI (-0.66%), ALGO (-0.47%) — both negative on scalps AND regime.

## Trading Frequency

| Metric | Value |
|--------|-------|
| Scalp trades/hour | 21.7 |
| Regime trades/hour | 1.3 |
| Avg scalp hold time | ~2-5 minutes |

The system was extremely active — nearly 22 scalp trades per hour across 18 coins.

## Biggest Single-Trade Losses

All 10 largest losses were regime SL hits, not scalps:

| Coin | Type | P&L | PnL% |
|------|------|-----|------|
| UNI | regime | -$0.26 | -2.6% |
| DOT | regime | -$0.21 | -2.1% |
| DOT | regime | -$0.18 | -1.8% |
| ALGO | regime | -$0.17 | -1.7% |
| ALGO | regime | -$0.17 | -1.7% |

## Key Takeaways

1. **Scalping at TP=0.20%/SL=0.10% is marginal** — PF of 1.07 means you're one bad streak away from negative. This confirms RUN10's finding that narrow TP/SL doesn't work.

2. **vol_spike_rev was correctly disabled mid-session** — 35.7% WR at these tight levels makes it a net loser. (RUN10 showed it becomes profitable at TP=0.80% with F6 filter.)

3. **Regime trades are the profit engine** — 55.6% WR with 2.24 PF. SMA crossback exits are excellent (15/15 wins).

4. **No fees were charged** — this was paper trading against Binance data with no fee deduction. In production, scalp profits would be further reduced by fees (unless using Bitfinex 0% taker).

5. **UNI and ALGO are consistently weak** — both negative on scalps and regime. May warrant different strategy assignments or exclusion.

## v9 → v10 Changes (applied after this test)

Based on RUN10 analysis, COINCLAW v10 addresses the scalp profitability problem:
- TP widened from 0.20% → **0.80%** (8:1 reward/risk ratio)
- F6 filter gate added (counter-momentum + active candles)
- vol_spike_rev re-enabled (profitable at wide TP + F6)
- bb_squeeze_break removed (zero contribution after F6 filter)
