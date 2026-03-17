# RUN16 — ML Feature Importance & Walk-Forward Validation

## Goal

Test whether Random Forest / XGBoost can:
1. Identify which of 66 features have genuine predictive power for 15m next-bar direction
2. Produce tradeable signals via walk-forward validation (OOS)
3. Improve COINCLAW entry quality as a gate filter on existing strategies

Three-script lifecycle following DATAMINING.md plan.

---

## Method

### 16.1 — Feature Importance Discovery
- Dataset: 1-year 15m OHLCV for all 19 coins (35,040 bars each), warmup row 200 excluded
- Train/test split: 9 months train (~26,129 bars) / 3 months test (~8,710 bars)
- Models: `RandomForestClassifier(n_estimators=200, max_depth=10, min_samples_leaf=20, n_jobs=-1)`
  and `XGBClassifier(n_estimators=200, max_depth=6, lr=0.1, min_child_weight=20)`
- Feature importance: Gini (RF) + permutation (RF) + XGB native
- Cross-validation: `TimeSeriesSplit(n_splits=5)` on training set
- Universal features: features appearing in top-20 Gini importance of >50% of coins

### 16.2 — Walk-Forward Validation
- Top-20 universal features from 16.1 as input
- 3 non-overlapping windows per coin (train 4mo, test 2mo)
- Signals: enter long when `predict_proba > 0.60`, exit when `< 0.45`
- Backtester: fee=0.1%, slippage=0.05%, SL=0.3%
- Baseline: buy every bar, exit after 5 bars

### 16.3 — ML as Gate Filter on COINCLAW Strategies
- 20 COINCLAW strategies tested per coin
- ML gate: only execute strategy signal if RF also predicts `prob_up > 0.55`
- Compare gated vs ungated win rate per strategy per coin

---

## Results

### 16.1 — Feature Importance

#### Model Accuracy Summary

| Metric | Value |
|--------|-------|
| RF avg accuracy (19 coins) | **54.58%** |
| XGBoost avg accuracy | **52.49%** |
| CV mean (TimeSeriesSplit×5) | **53.28%** |
| Universal features found | **46** (present in >50% of coins) |

#### Per-Coin Accuracy

| Coin | RF Acc | XGB Acc | RF F1 | XGB F1 |
|------|--------|---------|-------|--------|
| SHIB | 61.16% | 57.70% | 0.134 | 0.371 |
| LINK | 57.59% | 54.62% | 0.231 | 0.400 |
| ALGO | 57.54% | 55.66% | 0.137 | 0.331 |
| AVAX | 57.12% | 54.51% | 0.163 | 0.370 |
| TRX  | 56.37% | 55.44% | 0.024 | 0.291 |
| XLM  | 54.85% | 52.40% | 0.338 | 0.471 |
| DOT  | 54.58% | 50.24% | 0.340 | 0.502 |
| NEAR | 54.10% | 50.77% | 0.280 | 0.432 |
| UNI  | 53.70% | 51.46% | 0.454 | 0.511 |
| ETH  | 53.63% | 52.39% | 0.594 | 0.571 |
| DOGE | 53.55% | 51.17% | 0.569 | 0.564 |
| ADA  | 53.52% | 51.42% | 0.448 | 0.508 |
| LTC  | 53.51% | 50.86% | 0.555 | 0.588 |
| XRP  | 53.49% | 52.04% | 0.496 | 0.500 |
| ATOM | 52.84% | 50.59% | 0.343 | 0.481 |
| BTC  | 52.65% | 52.18% | 0.540 | 0.545 |
| DASH | 52.55% | 50.91% | 0.416 | 0.520 |
| SOL  | 52.50% | 52.42% | 0.531 | 0.528 |
| BNB  | 51.79% | 50.51% | 0.589 | 0.563 |

**Note on F1:** Low RF F1 on SHIB (0.134) and TRX (0.024) despite high accuracy — model predicts the majority class almost exclusively. XGBoost is better calibrated.

#### Top-20 Universal Features (all 19 coins)

Features ranked by frequency in Gini top-20 across all coins:

1. `hull_vs_close` — HMA distance from price; captures momentum curvature
2. `ema9_vs_close` — short-term EMA position; trend micro-structure
3. `obv_slope` — 5-bar OBV change normalized; institutional flow proxy
4. `rsi_slope` — RSI 3-bar diff; momentum acceleration
5. `log_returns` — single-bar log return; raw autocorrelation signal
6. `atr_ratio` — ATR14/ATR50; local vs historical volatility regime
7. `vwap_distance` — distance from rolling VWAP; mean reversion anchor
8. `volume_trend` — 5-bar vol vs 20-bar vol MA; volume trend confirmation
9. `volatility_ratio` — 5-bar std / 20-bar std; volatility expansion/contraction
10. `returns_5` — 5-bar cumulative return; short-term momentum
11. `returns_1` — 1-bar return; recent price change
12. `volume_momentum` — 5-bar volume diff / vol SMA; volume spike detection
13. `lower_shadow` — lower candle shadow ratio; demand tail signal
14. `bb_position` — Bollinger band position; mean reversion context
15. `volume_ratio` — current bar volume vs 20-bar MA
16. `laguerre_rsi` — Laguerre RSI (γ=0.8); smoothed momentum oscillator
17. `cmf_20` — Chaikin Money Flow; buying/selling pressure
18. `body_ratio` — candle body vs range; conviction signal
19. `rsi_7` — 7-bar RSI; fast RSI
20. `hour_cos` — cyclical hour encoding; time-of-day seasonality

**Notable:** The top-5 features (`hull_vs_close`, `ema9_vs_close`, `obv_slope`, `rsi_slope`, `log_returns`) appear in **100% of coins** in the RF top-20. These are the most universally predictive features in the dataset.

---

### 16.2 — Walk-Forward Validation

#### Aggregate Results

| Model | Avg Win Rate | Avg PF | vs Baseline |
|-------|-------------|--------|-------------|
| RF | **35.4%** | 2.35 | **–1.9%** |
| XGB | **39.5%** | 1.38 | **+2.2%** |
| Baseline (buy every bar) | 37.4% | — | — |

#### Per-Coin Walk-Forward (3-window average)

| Coin | RF WR | XGB WR | Baseline WR | RF PF | XGB PF |
|------|-------|--------|-------------|-------|--------|
| ADA  | 26.1% | 40.4% | 36.3% | 2.72 | 1.41 |
| ALGO | 53.8% | 40.0% | 40.3% | 5.38 | 1.54 |
| ATOM | 42.3% | 38.4% | 39.5% | 2.37 | 1.34 |
| AVAX | 16.4% | 43.2% | 40.3% | 2.54 | 1.50 |
| BNB  | 38.2% | 36.1% | 36.7% | 1.78 | 0.88 |
| BTC  | 40.8% | 35.8% | 37.8% | 1.29 | 0.87 |
| DASH | 36.4% | 38.8% | 36.2% | 3.66 | 1.89 |
| DOGE | 35.3% | 38.0% | 37.3% | 1.87 | 1.37 |
| DOT  | 34.6% | 40.4% | 36.0% | 1.77 | 1.59 |
| ETH  | 34.4% | 40.7% | 37.6% | 1.27 | 1.18 |
| LINK | 51.3% | 42.4% | 40.5% | 4.16 | 1.55 |
| LTC  | 34.4% | 39.8% | 37.2% | 1.82 | 1.37 |
| NEAR | 35.0% | 40.8% | 37.7% | 2.32 | 1.65 |
| SHIB | 52.7% | 43.8% | 37.8% | 2.91 | 1.50 |
| SOL  | 28.8% | 38.7% | 38.4% | 1.56 | 1.29 |
| TRX  |  8.3% | 28.6% | 29.1% | 0.18 | 0.48 |
| UNI  | 38.1% | 42.0% | 37.1% | 3.12 | 1.92 |
| XLM  | 34.9% | 41.7% | 38.3% | 2.30 | 1.51 |
| XRP  | 31.9% | 41.6% | 35.8% | 1.68 | 1.38 |

**Key observations:**
- RF is highly erratic: wins 52.7% (SHIB) and 53.8% (ALGO) but crashes to 8.3% (TRX) and 16.4% (AVAX). High variance makes it unreliable in production.
- XGB is more consistent but at best 2.2% above baseline across all coins. PF < 1.0 for BNB and BTC (money-losing).
- The few RF "wins" (ALGO, LINK, SHIB) show high PF (5.38, 4.16, 2.91) but very few signals — these are cherry-picked windows, not robust performance.

---

### 16.3 — ML Gating on COINCLAW Strategies

| Metric | Value |
|--------|-------|
| Average ML help rate across all coins | **21.2%** |
| Coins where ML helps majority of strategies | **0 / 19** |

#### Per-Coin ML Effectiveness

| Coin | ML Help % |
|------|-----------|
| BNB  | 35.0% |
| BTC  | 35.0% |
| DASH | 35.0% |
| SOL  | 35.0% |
| ADA  | 30.0% |
| DOT  | 30.0% |
| ETH  | 30.0% |
| LINK | 30.0% |
| UNI  | 30.0% |
| ADA  | 30.0% |
| LTC  | 25.0% |
| XRP  | 20.0% |
| NEAR | 15.0% |
| DOGE | 14.3% |
| XLM  | 10.0% |
| ATOM | 18.2% |
| AVAX |  5.0% |
| ALGO |  5.3% |
| SHIB |  0.0% |
| TRX  |  0.0% |

Even the best coins (BNB, BTC, DASH, SOL) only have ML helping 35% of their strategies — below majority. No coin benefits from ML gating across most of its signals.

---

## Bugs Found During Run

None — `feature_engine.py` and `indicators.py` were already fixed in RUN15 (doji NaN, CMF propagation, flat-price RSI). All 16.1–16.3 scripts ran cleanly on first attempt.

---

## Conclusions

### Main Finding: ML Classification Is Not Viable for COINCLAW

The experiment tests the thesis *"ML can learn which 15m bars will go up."*

**The thesis fails on three levels:**

**Level 1 — In-sample (16.1):** RF achieves 54.6% accuracy. While statistically above random, 4.6% edge on binary classification is noise-adjacent. The F1 scores reveal why: many coins show RF accuracy near 54% but F1 near 0.45 — the model is mostly predicting the majority class (up-bars dominate in a bull year). SHIB's 61% accuracy / 0.134 F1 is the clearest example: the model just predicts "up" almost always.

**Level 2 — Out-of-sample (16.2):** RF *underperforms the baseline* by 1.9% WR averaged across all coins. In-sample accuracy does not generalize. XGB is marginally better (+2.2%) but with PF < 1.0 on several major coins (BNB, BTC, TRX). Neither model produces consistent alpha.

**Level 3 — As a gate filter (16.3):** Only 21% of coin-strategy combinations benefit from ML gating. For any given coin, the ML filter randomly helps some strategies and hurts others. There is no predictable pattern.

### What Works / What Doesn't

| Finding | Implication |
|---------|-------------|
| Top-5 features universal across all coins | Confirms COINCLAW indicator selection is correct |
| `hull_vs_close`, `ema9_vs_close`, `obv_slope` top-3 | These are already embedded in COINCLAW (HMA trend, EMA entry, OBV filter) |
| RF high variance across windows | RF is overfit; useless for production |
| XGB small positive OOS edge (+2.2%) | Not enough to justify added complexity |
| Low F1 despite decent accuracy | 2025 bull market creates class imbalance (more up-bars); models predict up by default |

### Feature Engineering Is Valuable (Not the Models)

The 66-feature matrix from RUN15/16 has clean, look-ahead-free features and confirmed what indicators matter. This knowledge is incorporated into:
- **Already in COINCLAW:** hull, ema9, obv, rsi, vwap — all confirmed top features
- **Could add:** `obv_slope` as a secondary filter (volume trend confirmation); `laguerre_rsi` already deployed (RUN13)

### Decision: No Changes to COINCLAW v13

The ML experiment is a negative result. The rule-based strategies outperform ML signals OOS. **COINCLAW v13 is unchanged.**

---

## Files

| File | Description |
|------|-------------|
| `run16_1_feature_importance.py` | RF + XGB feature importance across 19 coins |
| `run16_1_results.json` | Per-coin accuracy, Gini/permutation importance, universal features |
| `run16_2_walk_forward.py` | 3-window walk-forward with ML signals backtested |
| `run16_2_results.json` | Per-coin per-window WR, PF, Sharpe; baseline comparison |
| `run16_3_comparison.py` | ML gate filter applied to COINCLAW strategies |
| `run16_3_results.json` | Per-coin ML help %, per-strategy effectiveness |

**Also in this archive (prior NN experiment):**
- `README.md` — earlier NN mean reversion filter experiment (logistic regression, +0.8% WR)
- `run16_nn_backtest_v1/v2/v3.py` — NN experiment scripts
- `run16_results.json`, `run16_v2_results.json`, `run16_v3_results.json` — NN results

---

## Next

**RUN17** — Monte Carlo validation of COINCLAW v13 strategies. With the ML path closed, confirm that the rule-based strategies have statistically robust P&L distributions (not lucky streaks). Flag any strategy where the 5th-percentile outcome is a losing PF.
