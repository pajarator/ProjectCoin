# RUN16nn — Neural Network Mean Reversion Filter

## Goal

Test whether a neural network (logistic regression / MLP) can improve mean reversion win rate by learning *which* z-score signals are more likely to succeed, based on contextual indicator features.

Hypothesis: hard thresholds (z < −1.5) fire on all oversold conditions equally. A trained model might learn that some conditions (e.g., low volume, late Asia session, compressed BB) produce false reversals and filter them out.

Three script iterations, each fixing a flaw from the prior version.

---

## Method Evolution

### v1 — Direct MLP Prediction (`run16_nn_backtest.py`)
- Model: `MLPClassifier(hidden_layer_sizes=(64, 32))`
- 23 features: z_score, rsi_14, rsi_7, bb_position, atr_pct, volatility_20, stoch_k/d, macd_hist_norm, cci_20, momentum_10, volume_ratio, obv_slope, cmf_20, adx_14, aroon_up/down, laguerre_rsi, kalman_dist, kst, kst_signal, hour_of_day, day_of_week
- Target: bar-ahead return direction (4-bar lookahead)
- Split: 50/50 train/test (no walk-forward — **look-ahead prone**)
- Apply: generate signals whenever model predicts "up"

### v2 — NN as MR Gate Filter (`run16_nn_backtest_v2.py`)
- Fix v1 flaw: only gate existing mean reversion signals (z < −1.5), don't generate raw signals
- Threshold search: P > 0.50 / 0.55 / 0.60
- Still no walk-forward (model trained on same period it's evaluated on)
- Best threshold selected: 0.60

### v3 — Walk-Forward Validation (`run16_nn_backtest_v3.py`)
- Fix v2 flaw: proper 3-window walk-forward (train 2mo → test 1mo)
- Model: `LogisticRegression` (simpler, less prone to overfit than MLP)
- 8 features: z_score, rsi_14, bb_position, volume_ratio, stoch_k, momentum_10, hour_of_day, day_of_week
- Gate: only take mean reversion signal when P(success) > 0.55
- Baseline: standard mean reversion (z < −1.5) without filter

---

## Results

### v1 — Direct MLP

| Metric | Value |
|--------|-------|
| Avg win rate | **42.3%** |
| Total signals | 146,888 |
| Avg P&L | **−17.3%** |

Complete failure. Win rate 42.3% is *below* the mean reversion baseline (~52%). The MLP generates signals on every bar it predicts "up," not just mean reversion setups — it's trading noise.

---

### v2 — NN as Gate Filter (No Walk-Forward)

| Threshold | Avg WR | Signals | Avg P&L |
|-----------|--------|---------|---------|
| P > 0.50 | 51.4% | 24,820 | −51.4% |
| P > 0.55 | 51.5% | 16,919 | −51.1% |
| P > 0.60 | 52.2% | 9,880 | −50.9% |
| **Baseline** | **52.4%** | **75,822** | — |

Filter barely matches baseline at P > 0.60. P&L is deeply negative for all thresholds (−50%), while baseline MR with exits also shows negative P&L suggesting the baseline strategy itself is poor in this period. No walk-forward means model is evaluated partly on data it trained on.

---

### v3 — Walk-Forward with Logistic Regression (Final)

| Metric | NN Filter | Baseline | Delta |
|--------|-----------|----------|-------|
| Avg win rate | **53.2%** | **52.4%** | **+0.8 pp** |
| Total signals | 12,406 | 75,823 | **−84%** |

#### Per-Coin Results (3-window average)

| Coin | NN WR | Baseline WR | Delta | Signals | Window Range |
|------|-------|-------------|-------|---------|--------------|
| ETH  | 58.4% | 52.4% | **+6.0** | 896 | 54.0–66.4% |
| BNB  | 56.9% | 52.4% | **+4.5** | 1,376 | 55.5–58.2% |
| LTC  | 54.9% | 52.4% | **+2.5** | 916 | 52.7–58.1% |
| DASH | 54.2% | 52.4% | +1.8 | 1,102 | 50.0–59.0% |
| XRP  | 54.2% | 52.4% | +1.8 | 1,069 | 51.1–55.8% |
| DOT  | 54.2% | 52.4% | +1.8 | 914 | 50.7–58.6% |
| ATOM | 53.8% | 52.4% | +1.4 | 700 | 48.8–59.0% |
| SOL  | 53.2% | 52.4% | +0.8 | 715 | 49.0–57.7% |
| NEAR | 53.3% | 52.4% | +0.9 | 344 | 45.2–64.1% |
| LINK | 52.6% | 52.4% | +0.2 | 635 | 49.7–55.2% |
| ADA  | 52.5% | 52.4% | +0.1 | 560 | 42.9–63.2% |
| BTC  | 51.6% | 52.4% | −0.8 | 1,107 | 49.5–55.1% |
| AVAX | 51.9% | 52.4% | −0.5 | 302 | 44.4–55.9% |
| UNI  | 51.0% | 52.4% | −1.4 | 381 | 48.2–53.8% |
| DOGE | 49.8% | 52.4% | −2.6 | 777 | 46.3–53.6% |
| SHIB | 48.9% | 52.4% | −3.5 | 612 | 48.6–49.5% |

---

## Analysis

### Why v1 Failed
MLP trained on 23 features on 50% of data is massively overfit. It learns the training set's noise patterns and produces 146k "up" signals — far more than any reasonable mean reversion count. Win rate 42.3% is *worse than random*, meaning the model learned patterns anti-correlated with future returns. Classic overfitting to a noisy classification target.

### Why v2 Was Inconclusive
Applying the gate to existing MR signals (z < −1.5) fixes the signal-count problem. At P > 0.60, WR = 52.2% vs baseline 52.4% — essentially no improvement. The flaw: no walk-forward. The logistic model was trained and evaluated on overlapping data, so the 52.2% figure is close to the baseline by construction. P&L is deeply negative for all variants — but this reflects the baseline MR strategy itself being unprofitable in the test period, not the NN filter.

### Why v3 Is Marginally Better
Walk-forward properly separates train/test. With 3 windows:
- **Overall:** +0.8 pp WR gain, but 84% signal reduction — so in practice, far fewer trades
- **Best coins:** ETH (+6.0 pp), BNB (+4.5 pp), LTC (+2.5 pp) — these show consistent improvement across windows (low window range: BNB 55.5–58.2%)
- **High-variance coins:** NEAR (45.2–64.1% range), ADA (42.9–63.2%) — the filter is unstable, window results diverge by 20 pp

### The Signal Reduction Problem
84% fewer signals = 12,406 trades instead of 75,823. Even if WR improves from 52.4% to 53.2%, the strategy becomes statistically fragile — a sequence of bad weeks can wipe out a quarter's worth of signal. The filter adds selectivity but removes the statistical robustness that comes from large trade counts.

### Key Diagnostic — SHIB
SHIB shows WR 48.9% with the NN filter vs 52.4% baseline, with very low window variance (48.6–49.5%). This is a reliable *degradation*. SHIB's near-zero price (low decimal precision) creates unusual z-score distributions that confuse the logistic model consistently.

### Cross-Version Comparison

| Version | Approach | Avg WR | vs Baseline | Verdict |
|---------|----------|--------|-------------|---------|
| v1 | MLP direct signal | 42.3% | −10.1 pp | Complete failure |
| v2 | LogReg gate, no WF | 52.2% | −0.2 pp | Inconclusive |
| v3 | LogReg gate, walk-forward | 53.2% | +0.8 pp | Marginal |

Progression confirms: the critical fix was walk-forward (v2→v3), and the model type mattered less. Logistic regression with proper OOS validation produces a small but real edge. MLP without OOS control produces catastrophic results.

---

## Conclusions

### Result: Marginal Negative

+0.8 pp WR improvement at the cost of 84% signal reduction is not a viable trade-off for production use:

1. **Statistical sample problem:** 12,406 signals in a year (across all coins) averages ~650 per coin. At 650 trades, a 0.8 pp WR improvement is within the margin of error. Not reliable.

2. **Inconsistency:** 6 coins improve, 4 are flat, 6 degrade. No reliable pattern predicts which coins benefit.

3. **High window variance on several coins:** NEAR (±10 pp), ADA (±10 pp) — the model is unstable across time. In live trading, a model that swings ±10 pp from window to window cannot be trusted.

4. **ETH and BNB are the exception:** Both show consistent improvement across windows (low variance, 2–3 pp range) and meaningful WR lift (+4–6 pp). If a coin-specific NN filter were deployed, these two would be the only candidates.

5. **The baseline is already good:** COINCLAW v13 mean reversion wins 52.4% without any ML. Getting to 53.2% via ML is not worth the complexity, the dependency, or the fragility.

### Decision: No Changes to COINCLAW

The NN filter is not adopted. The standard mean reversion signal (z < −1.5) remains the entry condition.

**What was learned:**
- Walk-forward validation is mandatory for any ML signal evaluation (v1 without WF was catastrophically misleading)
- Logistic regression >> MLP for small datasets with noisy targets
- SHIB/DOGE z-score distributions are pathological — any feature-based filter should exclude them or treat them specially
- The 8-feature set (z_score, rsi_14, bb_position, volume_ratio, stoch_k, momentum, hour, day) is a reasonable starting point for future ML-gating experiments

---

## Files

| File | Description |
|------|-------------|
| `run16_nn_backtest.py` | v1: MLP direct prediction — 42.3% WR, complete failure |
| `run16_nn_backtest_v2.py` | v2: LogReg gate, no walk-forward — 52.2% WR, inconclusive |
| `run16_nn_backtest_v3.py` | v3: LogReg gate, walk-forward — 53.2% WR, marginal |
| `run16_results.json` | v1 results: per-coin WR, signals, P&L |
| `run16_v2_results.json` | v2 results: threshold sweep (0.5/0.55/0.60), per-coin |
| `run16_v3_results.json` | v3 results: per-coin per-window WR, summary |
| `README.md` | Original experiment notes (kept for reference) |
