# RUN22 — Genetic Algorithm v2: Strategy Discovery

## Goal

Discover new entry/exit rule combinations using a genetic algorithm over 8 technical indicators. Test whether evolved strategies can exceed the 44% breakeven win rate on OOS data.

## Key Fix Over Python Stub

The original Python `run22_1_genetic_v2.py` had a critical bug: **fitness was evaluated on the test set** (`test_df`) during evolution — the `train_df` argument was accepted but never used. This made the entire GA an exercise in test-set overfitting.

**Corrected implementation:** fitness evaluated on train portion only. Best genome evaluated once on held-out test portion (true OOS).

## Method

- **Data:** 18 coins, 15m 1-year OHLCV
- **Split:** 67% train (≈8mo) / 33% test (≈4mo hold-out)
- **Genome:** 1–3 entry rules (AND conjunction) + 1–2 exit rules (OR disjunction)
- **Rule:** (indicator, operator {</>}, threshold)
- **Fitness:** Sharpe × √(trades) on train; requires ≥15 trades on train

**Indicators:**

| ID | Indicator | Range |
|----|-----------|-------|
| RSI14 | RSI(14) | 20–80 |
| z20 | z-score(20) | −2.5–2.5 |
| bb_pos | BB position (close−lower)/(upper−lower) | −0.3–1.3 |
| vol_rat | vol / rolling_mean(vol,20) | 0.3–4.0 |
| ROC5 | Rate of change 5 bars | −10–10 |
| StochK | Stochastic K(14) | 20–80 |
| ADX14 | ADX(14) Wilder's RMA | 5–60 |
| macd_h | MACD hist / ATR(14) normalized | −3–3 |

**GA Parameters:** Pop=80, Gens=40, Tournament=5, Elite=5, Mutate=20%, Crossover=70%

**Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side, breakeven WR ≈ 44%

## Results (OOS Test Half — 33% Hold-out)

| Coin | Trades | WR% | PF | P&L% | MaxDD% |
|------|--------|-----|----|------|--------|
| ADA  | 8 | 12.5% | 4.21 | +9.4% | 2.0% |
| ALGO | 1 | 100% | 1.05 | +1.1% | 0.0% |
| ATOM | 13 | 23.1% | 1.80 | +4.0% | 3.4% |
| AVAX | 32 | 21.9% | 0.68 | −4.0% | 4.8% |
| BNB | 110 | 10.0% | 0.75 | −12.9% | 22.5% |
| BTC | 73 | 1.4% | 0.47 | −18.4% | 30.3% |
| **DASH** | **1,331** | **16.2%** | **1.39** | **+628.8%** | **36.1%** |
| DOGE | 176 | 17.6% | 1.08 | +4.1% | 10.5% |
| DOT | 35 | 25.7% | 0.89 | −1.5% | 5.9% |
| ETH | 152 | 9.2% | 1.43 | +27.1% | 21.7% |
| LINK | 1 | 0.0% | — | −0.5% | 0.5% |
| LTC | 199 | 12.6% | 0.86 | −13.2% | 21.8% |
| NEAR | 312 | 12.2% | 0.82 | −24.2% | 46.8% |
| SHIB | 9 | 44.4% | 1.29 | +0.7% | 1.5% |
| SOL | 51 | 13.7% | 1.21 | +4.1% | 5.4% |
| UNI | 506 | 11.5% | 1.08 | +9.3% | 31.2% |
| XLM | 77 | 10.4% | 0.82 | −6.8% | 13.2% |
| XRP | 116 | 13.8% | 0.89 | −6.0% | 15.3% |

**Portfolio average:** Avg WR=19.78%, Avg PF=1.152, Avg P&L=+33.38%

## Indicator Usage in Evolved Entry Rules

| Indicator | Times Used |
|-----------|------------|
| z20 | 9 |
| ADX14 | 9 |
| macd_h | 8 |
| StochK | 7 |
| ROC5 | 6 |
| RSI14 | 4 |
| bb_pos | 1 |

z-score and ADX are the most frequently evolved indicators. Volume ratio was never selected.

## Conclusions

### No genome achieves breakeven WR on OOS data

**0 of 18 coins exceed 44% WR with ≥10 OOS trades.** The highest meaningful WR is SHIB at 44.4% with only 9 trades (too few to be reliable). Most evolved strategies produce 10–25% WR on the test half — far below breakeven.

### GA overfits to train distribution that doesn't generalise

The train period (8mo) and test period (4mo) have different market regimes. The GA optimises Sharpe × √(trades) on train, but the rule combinations that maximise this score in training do not carry to the test period. This is the classic data-mining bias in evolving trading strategies.

### DASH anomaly: high trade count drives P&L despite low WR

DASH's evolved genome (`bb_pos > -0.04` — almost always true, high-frequency entry) fires 1,331 times in the test half. With PF=1.39 at 16.2% WR, the DASH mean_rev strategy's massive profit in the test period drives the +628.8% result. This is a pre-existing strategy advantage (DASH/mean_rev was positive in all previous runs), not a GA discovery.

### Evolved rules are noisy, not interpretable patterns

Entry rules like "ADX14 > 27.54 AND macd_h < 2.75 AND ROC5 > 4.39" (ADA) are threshold-fitted noise, not genuine market microstructure. The same set of rules produces 12.5% WR on OOS data vs whatever was achieved on train — the GA found a local optimum in the train distribution.

### z20 and ADX most consistently selected

The GA gravitated toward z-score (oversold condition) and ADX (trend strength) most frequently. This is consistent with COINCLAW's existing vwap_rev strategy (which uses z-score as a filter). However the evolved thresholds produce worse results than the hand-crafted strategy.

## Decision

**NEGATIVE** — Genetic algorithm v2 does not discover strategies with OOS WR > 44%. The corrected OOS methodology (fitness on train only) confirms: GA-evolved strategies on 15m crypto data overfit to in-sample regime and degrade significantly on hold-out data. No COINCLAW changes.

## Files

| File | Description |
|------|-------------|
| `run22_results.json` | Per-coin results with evolved genomes |
| `run22_1_genetic_v2.py` | Original Python stub (test-set fitness bug) |
| `RUN22.md` | This file |

Source: `tools/src/run22.rs`
