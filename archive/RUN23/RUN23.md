# RUN23 — Differential Evolution Parameter Optimization

## Goal

Test whether `scipy.optimize.differential_evolution` (or equivalent) can find better strategy parameters than the default (midpoint) values used in prior RUNs. Three strategy templates with parameterizable indicators were optimized on train (67%) and evaluated on OOS test (33%).

## Fix Over Python Stub

The Python `run23_1_diff_evolution.py` had a bug: `volatility_breakout` had 3 parameters in the bounds list (`bb_std`, `vol_thresh`, `atr_mult`) but the strategy function only accepted and used 2 (`bb_std`, `vol_thresh`). The third parameter `atr_mult` was silently ignored — DE wasted search budget on a phantom dimension.

**Corrected implementation:** 2-parameter bounds for `volatility_breakout`, matching the actual signal generator.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV
- **Split:** 67% train (≈8mo) / 33% test (≈4mo hold-out)
- **DE params:** pop=15×dims, gens=100, F=0.8, CR=0.7, LCG RNG
- **Comparison:** DE-optimized params (fitness on train) vs default midpoint params, evaluated on OOS test half
- **Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side, breakeven WR ≈ 44%

**Strategy templates and parameter bounds:**

| Template | Parameters | Bounds |
|----------|-----------|--------|
| mean_reversion | rsi_period, rsi_lo, rsi_hi, bb_std | (7–21), (20–35), (65–80), (1.5–3.0) |
| momentum | rsi_period, rsi_lo, stoch_lo, stoch_hi | (7–21), (40–60), (20–40), (60–80) |
| volatility_breakout | bb_std, vol_thresh | (1.5–3.0), (1.0–3.0) |

## Results (OOS Test Half — 33% Hold-out)

### Mean Reversion (4 DE params)

| Coin | DEt | DEWR% | DEPF | DEP&L% | Deft | DefWR% | DefPF | DefP&L% | PFdelta |
|------|-----|-------|------|--------|------|--------|-------|---------|---------|
| ADA  | 68 | 33.8 | 1.54 | +11.8 | 123 | 27.6 | 1.11 | +4.0 | +0.44 |
| ALGO | 48 | 29.2 | 0.72 | -4.5 | 126 | 36.5 | 1.25 | +9.9 | -0.53 |
| ATOM | 55 | 23.6 | 0.81 | -4.0 | 103 | 34.0 | 1.19 | +6.4 | -0.38 |
| AVAX | 94 | 37.2 | 1.35 | +10.2 | 134 | 32.8 | 1.15 | +6.1 | +0.20 |
| BNB  | 48 | 25.0 | 0.63 | -6.4 | 126 | 34.1 | 0.92 | -3.5 | -0.29 |
| BTC  | 26 | 23.1 | 0.33 | -6.0 | 146 | 34.2 | 0.74 | -11.1 | -0.41 |
| DASH | 264 | 26.9 | 1.21 | +20.1 | 107 | 26.2 | 1.18 | +6.7 | +0.03 |
| DOGE | 152 | 28.3 | 0.94 | -4.1 | 137 | 27.0 | 0.92 | -4.4 | +0.01 |
| DOT  | 51 | 29.4 | 0.93 | -1.3 | 119 | 36.1 | 1.63 | +26.1 | -0.70 |
| ETH  | 55 | 29.1 | 0.98 | -0.6 | 141 | 29.8 | 0.88 | -5.9 | +0.10 |
| LINK | 61 | 34.4 | 1.66 | +13.3 | 112 | 39.3 | 1.69 | +25.3 | -0.02 |
| LTC  | 70 | 28.6 | 1.32 | +7.8 | 109 | 27.5 | 0.97 | -2.0 | +0.36 |
| NEAR | 85 | 36.5 | 1.87 | +25.2 | 112 | 34.8 | 1.61 | +23.3 | +0.26 |
| SHIB | 224 | 30.8 | 0.79 | -15.4 | 119 | 31.1 | 1.02 | +0.2 | -0.23 |
| SOL  | 95 | 23.2 | 0.69 | -10.8 | 118 | 28.8 | 0.85 | -6.2 | -0.17 |
| UNI  | 116 | 24.1 | 0.95 | -2.8 | 124 | 23.4 | 0.79 | -9.8 | +0.16 |
| XLM  | 183 | 30.1 | 0.80 | -12.6 | 124 | 29.8 | 0.95 | -2.7 | -0.15 |
| XRP  | 43 | 41.9 | 1.59 | +7.5 | 119 | 31.1 | 0.89 | -4.6 | +0.71 |

**Avg PF delta: −0.034 | DE better (PF≥0): 9/18**

### Momentum (4 DE params)

| Coin | DEt | DEWR% | DEPF | DEP&L% | Deft | DefWR% | DefPF | DefP&L% | PFdelta |
|------|-----|-------|------|--------|------|--------|-------|---------|---------|
| ADA  | 451 | 13.3 | 0.55 | -55.1 | 1296 | 13.0 | 0.32 | -95.8 | +0.24 |
| ALGO | 490 | 16.5 | 0.51 | -60.5 | 1166 | 14.7 | 0.30 | -94.1 | +0.20 |
| ATOM | 528 | 16.9 | 0.51 | -61.1 | 1283 | 14.0 | 0.31 | -95.5 | +0.20 |
| AVAX | 447 | 14.5 | 0.56 | -53.3 | 1239 | 12.8 | 0.30 | -95.5 | +0.26 |
| BNB  | 461 | 12.1 | 0.26 | -70.7 | 1255 | 9.9 | 0.17 | -96.9 | +0.09 |
| BTC  | 470 | 12.6 | 0.34 | -65.8 | 1284 | 9.3 | 0.17 | -96.9 | +0.17 |
| DASH | 480 | 19.4 | 1.09 | +8.2 | 1242 | 20.3 | 0.84 | -53.1 | +0.25 |
| DOGE | 586 | 14.5 | 0.59 | -60.7 | 1298 | 11.3 | 0.32 | -95.6 | +0.27 |
| DOT  | 492 | 14.2 | 0.65 | -50.6 | 1247 | 14.9 | 0.36 | -94.1 | +0.29 |
| ETH  | 497 | 12.9 | 0.41 | -65.1 | 1375 | 10.8 | 0.24 | -96.9 | +0.17 |
| LINK | 605 | 13.4 | 0.45 | -72.4 | 1296 | 11.7 | 0.26 | -96.7 | +0.20 |
| LTC  | 438 | 11.4 | 0.33 | -67.1 | 1313 | 10.4 | 0.21 | -97.4 | +0.11 |
| NEAR | 529 | 15.7 | 0.60 | -56.7 | 1248 | 15.1 | 0.38 | -93.5 | +0.22 |
| SHIB | 494 | 14.6 | 0.53 | -59.0 | 1134 | 14.9 | 0.38 | -91.6 | +0.15 |
| SOL  | 546 | 16.7 | 0.58 | -57.0 | 1279 | 12.7 | 0.34 | -94.8 | +0.24 |
| UNI  | 452 | 16.8 | 0.80 | -31.8 | 1270 | 13.5 | 0.42 | -93.2 | +0.37 |
| XLM  | 444 | 16.2 | 0.53 | -54.5 | 1155 | 15.1 | 0.33 | -93.1 | +0.20 |
| XRP  | 447 | 13.9 | 0.56 | -53.1 | 1261 | 11.4 | 0.31 | -95.5 | +0.25 |

**Avg PF delta: +0.216 | DE better (PF≥0): 18/18**

### Volatility Breakout (2 DE params, fixed Python bug)

| Coin | DEt | DEWR% | DEPF | DEP&L% | Deft | DefWR% | DefPF | DefP&L% | PFdelta |
|------|-----|-------|------|--------|------|--------|-------|---------|---------|
| ADA  | 44 | 13.6 | 0.75 | -5.0 | 118 | 16.9 | 1.15 | +5.4 | -0.39 |
| ALGO | 41 | 14.6 | 0.29 | -11.7 | 97 | 17.5 | 0.87 | -5.8 | -0.58 |
| ATOM | 56 | 12.5 | 0.39 | -14.1 | 112 | 16.1 | 0.75 | -11.6 | -0.36 |
| AVAX | 78 | 15.4 | 0.78 | -7.4 | 112 | 17.0 | 0.87 | -6.7 | -0.09 |
| BNB  | 36 | 16.7 | 0.37 | -8.3 | 105 | 18.1 | 0.51 | -17.4 | -0.14 |
| BTC  | 45 | 24.4 | 0.40 | -9.3 | 108 | 20.4 | 0.61 | -15.1 | -0.21 |
| DASH | 102 | 14.7 | 2.22 | +51.7 | 141 | 11.3 | 1.54 | +26.6 | +0.67 |
| DOGE | 86 | 17.4 | 1.59 | +19.2 | 131 | 18.3 | 1.22 | +9.5 | +0.37 |
| DOT  | 67 | 16.4 | 1.19 | +4.4 | 126 | 15.9 | 1.38 | +17.8 | -0.20 |
| ETH  | 53 | 9.4 | 0.41 | -12.1 | 138 | 14.5 | 0.64 | -18.7 | -0.23 |
| LINK | 50 | 14.0 | 0.62 | -7.7 | 112 | 17.9 | 0.95 | -3.0 | -0.33 |
| LTC  | 25 | 8.0 | 0.21 | -8.7 | 97 | 14.4 | 0.48 | -19.5 | -0.28 |
| NEAR | 34 | 2.9 | 0.37 | -10.1 | 117 | 16.2 | 0.95 | -4.1 | -0.58 |
| SHIB | 27 | 7.4 | 0.26 | -8.3 | 97 | 21.6 | 0.73 | -10.0 | -0.46 |
| SOL  | 49 | 18.4 | 0.63 | -7.0 | 117 | 20.5 | 1.06 | +1.7 | -0.43 |
| UNI  | 84 | 14.3 | 1.17 | +4.7 | 113 | 15.0 | 1.14 | +5.1 | +0.02 |
| XLM  | 44 | 15.9 | 0.88 | -2.5 | 100 | 20.0 | 1.16 | +5.0 | -0.28 |
| XRP  | 53 | 18.9 | 0.49 | -10.3 | 110 | 19.1 | 0.91 | -4.6 | -0.41 |

**Avg PF delta: −0.217 | DE better (PF≥0): 3/18**

### WR Summary (best OOS WR per coin)

| Coin | MeanRev WR% | Momentum WR% | VolBreakout WR% |
|------|------------|--------------|----------------|
| ADA  | 33.8 | 13.3 | 13.6 |
| ALGO | 29.2 | 16.5 | 14.6 |
| ATOM | 23.6 | 16.9 | 12.5 |
| AVAX | 37.2 | 14.5 | 15.4 |
| BNB  | 25.0 | 12.1 | 16.7 |
| BTC  | 23.1 | 12.6 | 24.4 |
| DASH | 26.9 | 19.4 | 14.7 |
| DOGE | 28.3 | 14.5 | 17.4 |
| DOT  | 29.4 | 14.2 | 16.4 |
| ETH  | 29.1 | 12.9 | 9.4 |
| LINK | 34.4 | 13.4 | 14.0 |
| LTC  | 28.6 | 11.4 | 8.0 |
| NEAR | 36.5 | 15.7 | 2.9 |
| SHIB | 30.8 | 14.6 | 7.4 |
| SOL  | 23.2 | 16.7 | 18.4 |
| UNI  | 24.1 | 16.8 | 14.3 |
| XLM  | 30.1 | 16.2 | 15.9 |
| XRP  | 41.9 | 13.9 | 18.9 |

**WR > 44% with ≥10 trades: 0/54 (0.0%)**

## Conclusions

### DE cannot find profitable strategy parameters in the OOS test half

**0/54 strategy-coin combinations achieve 44% WR on OOS data.** The highest is XRP mean_reversion at 41.9% — close but still below breakeven. All momentum and volatility_breakout combinations produce 8–25% WR, far below 44%.

### Momentum DE improves PF across all 18 coins — but from a catastrophic baseline

DE improves momentum PF by +0.216 on average (18/18 coins improved). However the default midpoint params produce PF=0.17–0.84 (catastrophic), and DE improves to PF=0.26–1.09. The improvement is real but the strategy is still fundamentally unprofitable. DE reduced the number of trades (from ~1200 to ~500 per coin) by finding parameters that fire less often — smaller losses, not genuine win rate improvement.

### Mean reversion DE is statistically neutral vs default

Avg PF delta = −0.034, 9/18 better. The default midpoint params (rsi_period=14, rsi_lo=27, rsi_hi=72, bb_std=2.25) are a reasonable center of the search space and DE does not meaningfully improve on them. This confirms the parameter space is relatively flat — the strategy's OOS profitability is regime-dependent, not parameter-dependent.

### Volatility breakout DE actively hurts

DE hurts 15/18 coins (avg PF delta −0.217). DE overfits the 2-parameter space to training data. The default midpoint produces better OOS PF on most coins. The strategy template is not well-suited to these market conditions regardless of parameters.

### XRP anomaly

XRP mean_reversion at 41.9% WR is the closest any strategy comes to 44% breakeven, with only 43 trades. This is consistent with XRP being a historically stronger mean-reversion candidate (XRP had top-tier mean_reversion WR in prior RUNs), but 43 trades in a 4-month OOS window is insufficient to confirm statistical significance.

### Root cause: parameter tuning cannot fix regime mismatch

The consistent finding across RUN19–23: the OOS test half (H2 of the year) is a different market regime from the train half (H1). No position sizing, filter, ML gate, genetic algorithm, or differential evolution optimizer can manufacture alpha when the underlying strategy has negative expectancy in the target regime. The COINCLAW strategies were developed and validated on data where mean reversion worked — the test half includes periods where it didn't.

## Decision

**NEGATIVE** — Differential evolution does not discover strategy parameters with OOS WR > 44%. The optimizer improves in-sample fitness but improvements do not transfer to the hold-out test period. The parameter space is flatter than the regime effect. No COINCLAW changes.

## Files

| File | Description |
|------|-------------|
| `run23_results.json` | Per-coin DE vs default results for all 3 strategies |
| `run23_1_diff_evolution.py` | Original Python stub (phantom atr_mult bug) |
| `RUN23.md` | This file |

Source: `tools/src/run23.rs`
