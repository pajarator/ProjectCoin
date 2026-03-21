# RUN25 — ML-Based Regime Detection

## Goal

Test whether a 4-regime BTC framework (BULL/BEAR × HIGH/LOW volatility) predicts when COINCLAW strategies produce better win rates. If strategies work better in specific regimes, gating trades to the best regime could improve overall WR.

## Fix Over Python Stub

The Python `run25_1_regime_ml.py` trains RF/XGB to predict regime labels defined by `SMA50>SMA200 + vol>median`. This is circular: ML is learning to reconstruct the same rule it was given as the label definition. Classification "accuracy" measures how well the model recovers inputs from derived features — not predictive power.

**Corrected implementation:** Skip ML. Directly measure COINCLAW per-coin strategy WR across 4 BTC regimes using the rule-based labels. The actionable question is "does any regime produce WR ≥ 44%?" not "can ML predict regime labels with N% accuracy?"

## Method

- **Data:** 18 coins + BTC, 15m 1-year OHLCV
- **Split:** 67% train / 33% OOS test (regime labels derived from full data; strategy evaluated on test half only)
- **Regime definition** (BTC SMA50, SMA200, 20-bar return std):
  - Vol threshold = global median of train-half 20-bar return std (no look-ahead)
  - BEAR_LOW_VOL (0): SMA50 < SMA200 AND vol ≤ median
  - BEAR_HIGH_VOL (1): SMA50 < SMA200 AND vol > median
  - BULL_LOW_VOL (2): SMA50 > SMA200 AND vol ≤ median
  - BULL_HIGH_VOL (3): SMA50 > SMA200 AND vol > median
- **BTC test-half regime distribution:** BEAR_LOW_VOL 17.7%, BEAR_HIGH_VOL 37.5%, BULL_LOW_VOL 14.1%, BULL_HIGH_VOL 30.7%
- **Strategy per coin:** COINCLAW v13 primary long (from COIN_STRATEGIES)
- **Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side, breakeven WR ≈ 44%

## Results (OOS Test Half — 33% Hold-out)

### Per-Coin WR% by Regime

| Coin | Base WR% | BEAR_LOW t | WR% | PF | BEAR_HI t | WR% | PF | BULL_LOW t | WR% | PF | BULL_HI t | WR% | PF | Best |
|------|----------|-----------|-----|----|----------|-----|----|-----------|-----|----|----------|-----|----|------|
| DASH | 28.8 | 140 | 33.6 | 1.28 | 287 | 28.2 | 1.39 | 107 | 30.8 | 0.59 | 237 | 27.8 | 1.05 | BEAR_LOW_VOL |
| UNI  | 35.1 | 231 | 30.3 | 0.29 | 560 | 35.4 | 0.96 | 194 | 37.1 | 0.46 | 406 | 32.5 | 0.65 | BULL_LOW_VOL |
| NEAR | 36.0 | 231 | 39.8 | 0.54 | 572 | 33.7 | 0.91 | 196 | 33.7 | 0.39 | 407 | 31.4 | 0.72 | BEAR_LOW_VOL |
| ADA  | 36.6 | 249 | 35.7 | 0.29 | 572 | 36.7 | 0.94 | 194 | 33.0 | 0.36 | 383 | 29.2 | 0.52 | BEAR_HIGH_VOL |
| LTC  | 35.9 | 194 | 32.5 | 0.21 | 498 | 36.7 | 0.78 | 175 | 21.1 | 0.16 | 379 | 34.3 | 0.52 | BEAR_HIGH_VOL |
| SHIB | 32.8 | 204 | 23.0 | 0.19 | 560 | 37.0 | 0.76 | 192 | 29.2 | 0.31 | 377 | 28.4 | 0.47 | BEAR_HIGH_VOL |
| LINK | 38.0 | 215 | 29.8 | 0.24 | 535 | 38.9 | 0.98 | 174 | 33.3 | 0.31 | 362 | 34.0 | 0.61 | BEAR_HIGH_VOL |
| ETH  | 31.9 | 189 | 20.1 | 0.13 | 506 | 37.0 | 0.83 | 146 | 17.8 | 0.11 | 329 | 30.7 | 0.53 | BEAR_HIGH_VOL |
| DOT  | 36.9 | 240 | 37.1 | 0.34 | 522 | 38.1 | 1.01 | 198 | 32.8 | 0.33 | 390 | 31.8 | 0.62 | BEAR_HIGH_VOL |
| XRP  | 31.5 | 222 | 20.7 | 0.12 | 541 | 33.6 | 0.77 | 169 | 23.7 | 0.18 | 365 | 31.2 | 0.53 | BEAR_HIGH_VOL |
| ATOM | 36.2 | 194 | 33.5 | 0.31 | 520 | 37.7 | 0.89 | 183 | 30.6 | 0.34 | 405 | 32.1 | 0.45 | BEAR_HIGH_VOL |
| SOL  | 33.3 | 209 | 23.9 | 0.14 | 518 | 35.3 | 0.82 | 154 | 27.9 | 0.20 | 389 | 29.0 | 0.50 | BEAR_HIGH_VOL |
| DOGE | 30.3 | 61 | **44.3** | 0.41 | 143 | 25.9 | 0.88 | 53 | 32.1 | 0.25 | 106 | 27.4 | 0.53 | BEAR_LOW_VOL |
| XLM  | 31.3 | 67 | 22.4 | 0.17 | 222 | 32.0 | 0.83 | 76 | 31.6 | 0.33 | 168 | 26.2 | 0.43 | BEAR_HIGH_VOL |
| AVAX | 34.0 | 70 | 27.1 | 0.25 | 154 | 31.2 | 0.83 | 43 | 25.6 | 0.21 | 71 | 35.2 | 1.22 | BULL_HIGH_VOL |
| ALGO | 36.3 | 100 | 36.0 | 0.26 | 151 | 31.1 | 0.74 | 51 | 35.3 | 0.35 | 86 | 40.7 | 1.18 | BULL_HIGH_VOL |
| BNB  | 30.0 | 178 | 11.8 | 0.05 | 467 | 33.4 | 0.65 | 151 | 16.6 | 0.07 | 322 | 32.3 | 0.39 | BEAR_HIGH_VOL |
| BTC  | 33.3 | 60 | 23.3 | 0.10 | 157 | 33.8 | 0.81 | 42 | 14.3 | 0.04 | 106 | 32.1 | 0.56 | BEAR_HIGH_VOL |

### Portfolio Average WR% by Regime

| Regime | Avg WR% | vs Baseline |
|--------|---------|-------------|
| Baseline | 33.79% | — |
| BEAR_HIGH_VOL | **34.20%** | **+0.41pp** |
| BULL_HIGH_VOL | 31.47% | −2.32pp |
| BEAR_LOW_VOL | 29.17% | −4.62pp |
| BULL_LOW_VOL | 28.14% | **−5.65pp** |

**WR > 44% with ≥10 trades: 1/72** (DOGE BEAR_LOW_VOL at 44.3%, 61 trades)

## Conclusions

### No regime achieves 44% WR — BEAR_HIGH_VOL is marginally best at 34.20%

**Only 1 of 72 coin-regime combinations clears the 44% breakeven threshold.** DOGE at 44.3% WR in BEAR_LOW_VOL regime has only 61 trades — statistically unreliable. The portfolio average WR in every regime is 28–34%, all below breakeven.

BEAR_HIGH_VOL is the best regime (+0.41pp over baseline at 34.20%). The mechanism is intuitive: high-volatility bear markets create sharp dips that snap back quickly — the ideal conditions for mean-reversion entries. But even in this regime, WR is 10pp below breakeven.

### BULL_LOW_VOL is worst: slow bull destroys mean-reversion

BULL_LOW_VOL produces the lowest average WR at 28.14%, 5.65pp below baseline. When BTC is above SMA200 but vol is low (gradual uptrend), individual coin dips tend to be trend-continuation pauses rather than reversion candidates. Mean-reversion entries fire but price continues drifting lower rather than reverting.

### Regime filtering reduces trades without improving per-trade quality

Similar to RUN21 (sentiment regime filter): restricting to a specific regime reduces the number of trades proportionally without meaningfully improving WR in that subset. The per-trade loss rate is regime-dependent but no regime reliably produces profitable trades.

### ALGO and AVAX anomaly in BULL_HIGH_VOL

ALGO (40.7% WR, PF=1.18) and AVAX (35.2% WR, PF=1.22) both perform best in BULL_HIGH_VOL — the regime with BTC in uptrend AND high volatility. These two coins have shown consistent upside bias in prior RUNs. However 40.7% is still below 44% breakeven, and BULL_HIGH_VOL covers only 30.7% of test bars.

### The fundamental problem persists

Mean-reversion strategies require WR ≥ 44% to break even given SL=0.3%/no-TP structure. The best achievable regime filter improves portfolio WR by 0.4pp — from 33.8% to 34.2%. Closing a 10pp gap with a 0.4pp filter is insufficient. Regime filtering optimizes over a fixed-structure problem but cannot manufacture breakeven performance from a negative-expectancy baseline.

## Decision

**NEGATIVE** — 4-regime BTC framework (BULL/BEAR × HIGH/LOW vol) does not improve COINCLAW win rate above 44%. BEAR_HIGH_VOL is marginally best but +0.41pp improvement is negligible. BULL_LOW_VOL is the worst regime and should be avoided, but even this insight doesn't produce a profitable strategy. No COINCLAW changes.

## Files

| File | Description |
|------|-------------|
| `run25_results.json` | Per-coin WR/PF across 4 BTC regimes |
| `run25_1_regime_ml.py` | Original Python stub (circular ML) |
| `RUN25.md` | This file |

Source: `tools/src/run25.rs`
