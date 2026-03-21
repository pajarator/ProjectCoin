# RUN21 — Sentiment Regime Filter (BTC RSI as Fear/Greed Proxy)

## Goal

Test whether COINCLAW mean reversion entries succeed more often when the broader crypto market is in "fear" mode (oversold). The "buy the fear" hypothesis: extreme bearishness overshoots price to the downside, creating cleaner reversion targets with higher win rates.

## Background

Originally planned to use Crypto Fear & Greed Index from alternative.me (daily, free API). No cached data was available. Reframed as BTC RSI(14) — a cleaner proxy since F&G itself is ~40% derived from price momentum and RSI.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV, COINCLAW v13 strategy per coin
- **Sentiment proxy:** BTC RSI(14) and BTC z-score(50) computed at each bar
- **Split:** 50/50 chronological — evaluated on OOS test half only
- **Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side

**Regime definitions (BTC RSI14):**

| Regime | Condition | Bars in Test Half |
|--------|-----------|-------------------|
| Ext Fear | RSI < 30 | 10.6% |
| Fear | RSI < 40 | 26.6% |
| Neutral | RSI 40–60 | ~46% |
| Greed | RSI > 60 | 27.0% |
| Ext Greed | RSI > 70 | ~8% |
| Z-Fear | z50 < -1.0 | 26.7% |

## Results (OOS Test Half)

### Portfolio Summary (18-coin average)

| Regime | Avg WR% | Avg P&L% | Avg MaxDD% | Trade Retention |
|--------|---------|---------|-----------|----------------|
| Baseline | 33.59% | -51.99% | 68.40% | 100% |
| Ext Fear | 30.41% | -25.49% | 34.65% | 12.6% |
| Fear | 32.93% | -41.17% | 51.94% | 22.2% |
| Neutral | 33.80% | -48.07% | 52.61% | 15.3% |
| Greed | 29.51% | -18.50% | 19.82% | 2.6% |
| Ext Greed | 29.19% | -4.71% | 5.58% | 0.5% |
| Z-Fear | 33.20% | -34.92% | 45.33% | 20.3% |

### Per-Coin WR% Highlights

| Coin | Base | ExtFear | Fear | Neutral | Greed | Z-Fear |
|------|------|---------|------|---------|-------|--------|
| ADA  | 35.7% | 31.2% | 35.1% | 38.4% | 29.2% | 34.9% |
| ALGO | 37.0% | 31.0% | 34.6% | 43.6% | 34.5% | 34.9% |
| AVAX | 31.7% | 27.1% | 30.8% | 35.1% | **46.2%** | 30.2% |
| BNB  | 32.4% | 35.3% | 35.1% | 30.2% | 22.3% | 34.0% |
| DASH | 29.1% | 26.0% | 28.3% | 28.8% | 32.2% | 27.9% |
| LINK | 36.7% | 33.1% | 35.9% | 39.8% | 35.9% | 37.2% |

## Conclusions

### The fear hypothesis is wrong — fear regime produces lower WR, not higher

**ExtFear (RSI<30) produces 30.41% avg WR — the lowest of all regimes, 3 points below baseline 33.59%.** When BTC is deeply oversold, COINCLAW mean reversion entries fail more often than usual. This is the opposite of the hypothesis.

Interpretation: during extreme fear (deep BTC crash), individual coin dips are part of a broader cascade — entries get stop-lossed as the crash continues. Mean reversion requires a stable anchor, which is absent in extreme fear.

### P&L improvements are a sample-size artifact

The apparent P&L improvements in Fear (-41% vs baseline -52%) and Greed (-18.5%) regimes are not from higher WR — they are from **fewer trades**. Fewer trades = smaller total loss in a losing period. The per-trade loss rate is similar or worse in filtered regimes.

- Fear: 22.2% of trades, still losing at 32.93% WR
- Greed: 2.6% of trades, losing at 29.51% WR — the best P&L is because almost no trades fire
- Ext Greed: 0.5% of trades — effectively zero trades, trivially small loss

### Neutral is the best real regime

The Neutral regime (RSI 40–60) produces the highest avg WR (33.80%) with 15.3% of trades. This is the regime where BTC is neither panicking nor euphoric — stable conditions are best for mean reversion. But 33.80% is still 10 points below the 44% breakeven.

### Z-Fear (BTC z50 < -1.0) vs RSI Fear

BTC z-score(50) below -1.0 produces similar results to RSI Fear: 33.20% WR, -34.92% P&L, 20.3% retention. The two filters are capturing largely the same regime since both measure BTC being below a rolling average.

### No filter crosses the breakeven

No regime achieves the 44% WR needed to break even given the SL=0.3% / no-TP structure:

```
Required WR = SL / (avg_win + SL) ≈ 44%
Best regime WR = 33.80% (Neutral) — 10pp gap
```

### AVAX anomaly

AVAX shows 46.2% WR in Greed regime — the only coin to clear breakeven in any regime. However Greed retains only 2.6% of signals (a handful of trades), making this statistically unreliable (small sample).

## Decision

**NEGATIVE** — BTC sentiment regime filtering does not improve COINCLAW win rate above the breakeven threshold. Fear regime actually reduces WR. Neutral is marginally best but insufficient. No COINCLAW changes.

Note: Actual Crypto Fear & Greed data may differ from BTC RSI — it includes social media, volatility, dominance, and search trends. The core finding (fear is bad for mean reversion) is likely robust, as it reflects the underlying market mechanics.

## Files

| File | Description |
|------|-------------|
| `run21_results.json` | Full per-coin and portfolio results (Rust) |
| `run21_1_sentiment_features.py` | Original Python stub |
| `data_fetcher_sentiment.py` | F&G fetcher (alternative.me) |
| `RUN21.md` | This file |

Source: `tools/src/run21.rs`
