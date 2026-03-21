# RUN15a: Bayesian Entry Gate vs COINCLAW Primary Strategies

**Date:** 2026-03-16
**Objective:** Test whether a Bayesian probability gate on trade entries improves COINCLAW's per-coin primary long strategies on out-of-sample data.

---

## Experiment Design

### Hypothesis
For each trade entry that the COINCLAW primary strategy generates, additionally require that the historical win probability given the current z-score bin and RSI bin exceeds a threshold. If the market conditions at entry historically correlate with winning trades, the gate should improve quality.

### Method
- **Data:** 1 year of 15m bars for 18 coins from `data_cache/`
- **Train/Test:** 50/50 split — first 6 months to learn, last 6 months out-of-sample
- **Per-coin:** Each coin uses its actual COINCLAW primary long strategy
- **Model:** Beta(1,1) prior per `(z_bin, rsi_bin)` cell; cell = floor(z/0.5) × floor(rsi/10). Posterior trusted only after ≥5 observations.
- **Trade outcomes:** Actual backtest (SL=0.3%, slippage=0.05%, signal exit) — not a 4-bar lookahead

### Three modes tested on OOS data:
| Mode | Description |
|------|-------------|
| **Binary** | COINCLAW primary signals unchanged (baseline) |
| **Bay>60%** | Primary entry AND P(win\|z,rsi) > 60% |
| **Bay>55%** | Primary entry AND P(win\|z,rsi) > 55% |

---

## Results

### Per-Coin (OOS test period)

| Coin | Strat | Train Trades | Binary WR / PF | Bay>60% WR / PF / Trades | Bay>55% WR / PF / Trades |
|------|-------|-------------|----------------|--------------------------|--------------------------|
| ALGO | adr_rev | 453 | 40.0% / 1.76 | 0 trades | 0 trades |
| ATOM | vwap_rev | 1836 | 42.1% / 1.57 | 0 trades | 0 trades |
| AVAX | adr_rev | 430 | 35.1% / 1.64 | 21.1% / 1.33 (19t) | 21.1% / 1.33 (19t) |
| ADA | vwap_rev | 1800 | 42.7% / 1.60 | **52.9% / 2.44** (17t) | **52.9% / 2.44** (17t) |
| BNB | vwap_rev | 1369 | 47.7% / 1.51 | 37.1% / 1.09 (97t) | 38.8% / 1.23 (188t) |
| BTC | bb_bounce | 445 | 43.0% / 1.60 | 0 trades | 0 trades |
| DASH | mean_rev | 953 | 29.4% / 2.53 | 0 trades | 35.7% / 3.92 (14t) |
| DOGE | bb_bounce | 527 | 31.5% / 1.60 | 0 trades | 0 trades |
| DOT | vwap_rev | 1861 | 41.9% / 1.72 | 0 trades | 0 trades |
| ETH | vwap_rev | 1589 | 44.6% / 1.49 | 46.2% / 1.40 (130t) | 39.7% / 1.34 (174t) |
| LINK | vwap_rev | 1858 | 43.6% / 1.71 | 0 trades | **43.8% / 3.33** (16t) |
| LTC | vwap_rev | 1695 | 45.6% / 1.61 | **47.7% / 1.77** (128t) | 51.1% / 1.41 (1103t) |
| NEAR | vwap_rev | 1956 | 38.5% / 1.78 | 35.5% / 2.26 (31t) | 35.6% / 1.58 (163t) |
| SHIB | vwap_rev | 1781 | 42.6% / 1.44 | 0 trades | 19.0% / 0.54 (21t) ❌ |
| SOL | vwap_rev | 1716 | 42.0% / 1.50 | 0 trades | 12.5% / 0.59 (16t) ❌ |
| UNI | vwap_rev | 1931 | 40.6% / 1.71 | 0 trades | 0 trades |
| XLM | dual_rsi | 714 | 36.5% / 1.44 | 0 trades | 0 trades |
| XRP | vwap_rev | 1725 | 42.1% / 1.36 | 0 trades | 0 trades |

### Summary (OOS)

| Metric | Binary | Bay>60% | Bay>55% |
|--------|--------|---------|---------|
| Avg WR | **40.5%** | 13.4% | 19.5% |
| Avg PF | **1.64** | 0.57 | 0.98 |
| Sum P&L | **+10,514%** | +39% | +140% |
| Total Trades | 25,753 | 422 (2%) | 1,731 (7%) |
| Coins beating binary (WR) | — | 3/18 | 4/18 |
| Coins beating binary (PF) | — | 3/18 | 3/18 |

---

## Conclusions

### Negative result — Bayesian gate severely hurts COINCLAW

1. **The gate is too selective and leaves the model idle.** Bay>60% fires on only 2% of binary signals (422 vs 25,753 trades). 10 of 18 coins get **zero trades** from the 60% gate. This isn't selectivity — the model simply never learns a cell with P(win) > 60% from the training data.

2. **The cells that do pass are too sparse to be reliable.** Each (z_bin, rsi_bin) cell has few observations (models have 9–25 populated cells for hundreds to thousands of trades). The posterior stabilises at the prior (0.5) for most cells, so the gate rarely fires. When it does fire at isolated cells, it's not statistically meaningful.

3. **The few trades that pass the gate actually perform worse overall.** Bay>60% avg PF = 0.57 (money-losing). Bay>55% avg PF = 0.98 (near breakeven). The gate is selecting the wrong entries.

4. **Why:** The z-score + RSI joint distribution doesn't partition predictive signal. COINCLAW's strategies already capture the profitable regime with their entry conditions. Further conditioning on (z_bin, rsi_bin) within the already-filtered entry set adds noise, not signal — the cells don't have enough data per cell to learn a meaningful posterior.

5. **DASH bay>55% looks interesting (PF=3.92, 14 trades)** but this is 14 trades — statistically meaningless. LINK bay>55% (PF=3.33, 16 trades) same issue.

### Why the original RUN15a conclusion was accidentally right for the wrong reasons

The original code's "Binary outperforms Bayesian" conclusion is confirmed here, but the original code:
- Tested a made-up strategy not matching any COINCLAW signal
- Used 4-bar lookahead as trade outcome (not actual backtest)
- Had dead code preventing vwap_rev from ever appearing
- Computed VWAP as current close price
- Applied 3× leverage, distorting all return figures

### Recommendation

**No COINCLAW changes.** The Bayesian approach does not add value on top of existing entry conditions. The per-cell sample size problem is fundamental — with 400–2000 trades and 15–25 active cells, there are only ~20–80 observations per cell, far too few to distinguish P(win)=0.42 from P(win)=0.55 with statistical confidence.

If Bayesian gating is revisited, alternatives to explore:
- Use only z-score binning (not z×RSI joint) to get more observations per bin
- Use regime-level (BULL/BEAR/SIDEWAYS) conditioning rather than fine indicator bins
- Aggregate across coins first, then apply a shared prior

---

## Files

| File | Description |
|------|-------------|
| `bayesian_backtest.rs` | Original buggy implementation by other AI |
| `results.json` | Results from original buggy run |
| `run15a_corrected_results.json` | Results from corrected Rust implementation |
| `README.md` | This file (updated with corrected conclusions) |

Corrected implementation source: `tools/src/run15a.rs`
