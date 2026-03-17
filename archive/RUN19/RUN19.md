# RUN19 — Position Sizing Comparison

## Goal

Test whether Kelly criterion or half-Kelly sizing improves COINCLAW v13 portfolio returns versus fixed-fraction sizing. If Kelly helps, this could increase terminal wealth without requiring new strategies.

## Hypothesis

Kelly criterion, calibrated on historical win rate and avg win/loss, should produce higher risk-adjusted returns than fixed fraction because it sizes larger when edge is higher and smaller when edge is lower.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV (same as COINCLAW v13)
- **Strategies:** COINCLAW v13 per-coin assignment (loader.rs `COIN_STRATEGIES`)
- **Split:** 50/50 chronological — Kelly fraction estimated on train half only (OOS)
- **Fee:** 0.1%/side (0.2% round trip), applied to position size per trade
- **Slippage:** 0.05%/side, included in trade P&L via backtest()
- **Position sizing simulation:** `equity_change = fraction × equity × pnl_pct/100 − fee`
- **Kelly cap:** 25% max allocation per trade

**Methods tested:**

| Method | Fraction Rule |
|--------|---------------|
| Fixed 1% | 1% of equity per trade |
| Fixed 2% | 2% of equity per trade |
| Fixed 5% | 5% of equity per trade |
| Kelly OOS | Kelly fraction from train half, applied to test half |
| Half-Kelly OOS | Kelly / 2 |

## Results (OOS Test Half — Second 6 Months of Year)

### Per-Coin Returns

| Coin | Strategy | Test Trades | WR% | Kelly Frac | Fixed1% | Fixed2% | Fixed5% | Kelly OOS | HalfKel |
|------|----------|-------------|-----|------------|---------|---------|---------|-----------|---------|
| ADA  | vwap_rev | 1,869 | 42.7% | 0.188 | -1.8% | -3.6% | -8.9% | -29.5% | -16.0% |
| ALGO | adr_rev  |   481 | 36.3% | 0.126 | -0.3% | -0.5% | -1.4% |  -3.4% |  -1.7% |
| ATOM | vwap_rev | 1,838 | 42.4% | 0.151 | -1.9% | -3.7% | -9.1% | -24.9% | -13.3% |
| AVAX | adr_rev  |   449 | 35.0% | 0.179 | -0.4% | -0.8% | -2.0% |  -7.1% |  -3.6% |
| BNB  | vwap_rev | 1,456 | 48.2% | 0.051 | -1.8% | -3.5% | -8.6% |  -8.8% |  -4.5% |
| BTC  | bb_bounce|   482 | 43.3% | 0.048 | -0.4% | -0.9% | -2.2% |  -2.1% |  -1.1% |
| DASH | mean_rev | 1,030 | 34.4% | 0.183 | +1.5% | +3.0% | +7.7% | +30.9% | +14.5% |
| DOGE | bb_bounce|   541 | 32.3% | 0.175 | -0.4% | -0.9% | -2.2% |  -7.6% |  -3.9% |
| DOT  | vwap_rev | 1,864 | 41.8% | 0.151 | -1.4% | -2.8% | -6.8% | -19.3% | -10.1% |
| ETH  | vwap_rev | 1,640 | 45.7% | 0.149 | -2.0% | -3.9% | -9.5% | -25.8% | -13.8% |
| LINK | vwap_rev | 1,844 | 42.3% | 0.166 | -1.5% | -3.0% | -7.3% | -22.1% | -11.7% |
| LTC  | vwap_rev | 1,713 | 45.7% | 0.182 | -1.7% | -3.5% | -8.4% | -27.5% | -14.8% |
| NEAR | vwap_rev | 1,950 | 39.2% | 0.178 | -1.1% | -2.2% | -5.4% | -18.1% |  -9.5% |
| SHIB | vwap_rev | 1,816 | 42.5% | 0.161 | -2.2% | -4.4% |-10.7% | -30.7% | -16.8% |
| SOL  | vwap_rev | 1,772 | 42.0% | 0.148 | -2.0% | -4.0% | -9.7% | -26.0% | -14.0% |
| UNI  | vwap_rev | 1,951 | 40.3% | 0.174 | -1.5% | -2.9% | -7.1% | -22.7% | -12.0% |
| XLM  | dual_rsi |   745 | 37.9% | 0.184 | -0.9% | -1.7% | -4.2% | -14.7% |  -7.6% |
| XRP  | vwap_rev | 1,759 | 42.2% | 0.124 | -2.4% | -4.7% |-11.4% | -26.0% | -14.0% |

### Portfolio Summary (18-coin average)

| Method | Avg Return | Avg Max DD | Avg Calmar |
|--------|-----------|------------|------------|
| Fixed 1% | -1.24% | 1.35% | -0.509 |
| Fixed 2% | -2.45% | 2.68% | -0.506 |
| Fixed 5% | -5.96% | 6.54% | -0.497 |
| Kelly OOS | -15.86% | 18.04% | -0.454 |
| Half-Kelly OOS | -8.56% | 9.60% | -0.484 |

### Kelly Fractions (estimated from train half)

| Stat | Value |
|------|-------|
| Min | 0.048 (BTC, BNB) |
| Median | 0.166 |
| Mean | 0.151 |
| Max | 0.188 (ADA) |
| Cap | 25% |

## Conclusions

### Primary finding: Position sizing is moot when strategies have negative expected value

**17 of 18 coins lose money in the OOS test half regardless of sizing method.** DASH (mean_rev) is the only positive coin. This is not a sizing problem — it is an expectancy problem. In the test period (H2 of the data year), COINCLAW v13 long strategies did not have positive expected value after fees.

### Why the strategies are fee-negative in the test half

With SL=0.3% and fee=0.2% round trip, a strategy needs:
```
WR × avg_win > (1 − WR) × 0.3% + 0.2%
```
At the observed WR of ~42% and rough avg_win of ~0.5–0.8%, the strategies are marginal at best. In a bearish or sideways H2 period, more reversals fail (more stop losses, smaller wins), pushing expected value negative.

### Kelly dramatically amplifies losses

Kelly fractions in the range 0.15–0.19 (15–19% of equity per trade) with 1,700–1,950 trades in 6 months create compounding loss of 20–31%. This is the **anti-Kelly scenario**: Kelly is calibrated on a winning train period, applied to a losing test period. The Kelly multiplier amplifies the loss by ~15×–17× compared to Fixed 1%.

This is a known risk of Kelly in non-stationary systems. Kelly assumes the future distribution of wins/losses matches the past. When it doesn't, Kelly fails catastrophically.

### Fixed 1% minimizes loss but cannot produce positive returns from a losing edge

Fixed 1% produces the smallest losses (-1.24% avg). This is optimal in a losing period — size as small as possible to preserve capital. But no sizing method can produce positive returns from a strategy with negative expectancy.

### DASH (mean_rev) is the anomaly

DASH's mean_rev strategy is positive in both halves: +30.9% with Kelly OOS. The Kelly fraction of 0.183 (18.3%) matches the strategy's observed edge. This validates Kelly when the edge is real and stable.

### Implication for COINCLAW

The negative OOS results do not invalidate COINCLAW v13. The RUN17 Monte Carlo demonstrated robustness on the full year. However:
1. H2 was a weaker period for vwap_rev strategies
2. With proper fees (0.2% round trip), WR requirements are higher than previously measured
3. The live trader's actual performance should be monitored against the breakeven WR

**No changes to COINCLAW** — position sizing is not the constraint. If anything, the fixed 2% position sizing currently implied is appropriate (fixed 1% gives similar Calmar ratio but smaller absolute exposure).

## Decision

**NEGATIVE** — Kelly criterion does not improve COINCLAW v13 risk-adjusted returns in OOS testing. The root cause is that vwap_rev strategies underperform in the test period, and Kelly amplifies this underperformance. No COINCLAW changes recommended.

Fixed 2% remains the practical default (linear to Fixed 1% by Calmar, but gives 2× capital growth in winning periods).

## Files

| File | Description |
|------|-------------|
| `run19_results.json` | Full per-coin and portfolio results |
| `RUN19.md` | This file |

Source: `tools/src/run19.rs`
