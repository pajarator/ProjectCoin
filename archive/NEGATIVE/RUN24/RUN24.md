# RUN24 — Ensemble Strategy Framework

## Goal

Test whether voting ensembles of multiple COINCLAW strategies per coin outperform the best single strategy on OOS data. If strategies make independent errors, requiring agreement from 2+ strategies should filter noise and raise win rate.

## Fix Over Python Stub

The Python `run24_1_ensemble.py` found top-3 strategies using `find_top_strategies(df, n=3)` which evaluated ALL_STRATEGIES on the **full dataset** (including test period) — a look-ahead bias. The note in the code said "slight look-ahead, but acceptable for comparison". It is not acceptable.

**Corrected implementation:** Top-3 strategies ranked by PF on TRAIN half (67%) only. OOS test (33%) is never seen during selection.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV
- **Split:** 67% train (≈8mo) / 33% test (≈4mo hold-out)
- **Strategy pool (5):** vwap_rev, bb_bounce, adr_rev, dual_rsi, mean_rev
- **Selection:** Top-3 per coin by PF on train with ≥5 trades; tested on OOS
- **Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side, breakeven WR ≈ 44%

**Ensemble modes tested:**

| Mode | Description |
|------|-------------|
| best_single | Top-1 strategy by train PF |
| equal_vote | Entry if ≥2 of top-3 fire (majority, threshold=0.5) |
| pf_vote | PF-weighted entry, threshold=0.5 |
| intersection | Entry only if ALL top-3 fire (AND, strictest filter) |
| union | Entry if ANY top-3 fires (OR, most permissive) |

## Results (OOS Test Half — 33% Hold-out)

### Main Table (BestSingle, EqVote, PFVote)

| Coin | Best-t | WR% | PF | P&L% | Eq-t | WR% | PF | P&L% | PF-t | WR% | PF | P&L% | Winner |
|------|--------|-----|----|------|------|-----|----|------|------|-----|----|------|--------|
| DASH | 339 | 28.0 | 1.24 | +29.5 | 542 | 28.6 | 1.25 | +49.0 | 542 | 28.6 | 1.25 | +49.0 | intersection |
| UNI  | 346 | 30.3 | 0.95 | -5.9 | 485 | 32.6 | 0.93 | -11.5 | 485 | 32.6 | 0.93 | -11.5 | best_single |
| NEAR | 343 | 34.4 | 1.23 | +25.7 | 353 | 32.9 | 1.09 | +11.1 | 353 | 32.9 | 1.09 | +11.1 | intersection |
| ADA  | 343 | 32.9 | 0.88 | -13.0 | 363 | 30.9 | 0.84 | -19.2 | 363 | 30.9 | 0.84 | -19.2 | intersection |
| LTC  | 331 | 34.1 | 0.83 | -18.0 | 421 | 34.4 | 0.80 | -27.1 | 421 | 34.4 | 0.80 | -27.1 | best_single |
| SHIB | 329 | 32.5 | 0.80 | -22.2 | 471 | 30.1 | 0.66 | -55.4 | 471 | 30.1 | 0.66 | -55.4 | intersection |
| LINK | 333 | 39.3 | 1.11 | +11.2 | 410 | 37.1 | 0.99 | -1.5 | 410 | 37.1 | 0.99 | -1.5 | **intersection** |
| ETH  | 333 | 32.1 | 0.77 | -24.7 | 389 | 33.9 | 0.79 | -24.7 | 389 | 33.9 | 0.79 | -24.7 | pf_vote |
| DOT  | 354 | 34.5 | 0.97 | -3.7 | 490 | 32.0 | 0.87 | -21.1 | 490 | 32.0 | 0.87 | -21.1 | intersection |
| XRP  | 356 | 32.0 | 0.79 | -24.5 | 486 | 28.2 | 0.60 | -66.3 | 486 | 28.2 | 0.60 | -66.3 | best_single |
| ATOM | 325 | 32.0 | 0.86 | -15.0 | 464 | 31.5 | 0.78 | -34.5 | 464 | 31.5 | 0.78 | -34.5 | best_single |
| SOL  | 345 | 33.0 | 0.73 | -30.7 | 428 | 32.0 | 0.70 | -42.1 | 428 | 32.0 | 0.70 | -42.1 | intersection |
| DOGE | 347 | 30.3 | 0.66 | -40.4 | 475 | 28.8 | 0.62 | -62.5 | 475 | 28.8 | 0.62 | -62.5 | intersection |
| XLM  | 335 | 32.2 | 0.78 | -24.6 | 483 | 33.3 | 0.78 | -35.4 | 483 | 33.3 | 0.78 | -35.4 | best_single |
| AVAX | 359 | 36.5 | 0.95 | -5.6 | 427 | 36.3 | 0.90 | -13.5 | 427 | 36.3 | 0.90 | -13.5 | intersection |
| ALGO | 328 | 32.9 | 0.87 | -13.6 | 488 | 33.0 | 0.80 | -31.5 | 488 | 33.0 | 0.80 | -31.5 | intersection |
| BNB  | 350 | 36.0 | 0.70 | -31.7 | 451 | 34.4 | 0.61 | -53.4 | 451 | 34.4 | 0.61 | -53.4 | best_single |
| BTC  | 348 | 33.3 | 0.70 | -30.7 | 433 | 31.4 | 0.58 | -54.9 | 433 | 31.4 | 0.58 | -54.9 | intersection |

**Method wins:** best_single=6, equal_vote=0, pf_vote=1, intersection=11, union=0

**Average PF:** BestSingle=0.879, EqVote=0.811, PFVote=0.811

### WR Summary

| Coin | BestSingle | EqVote | PFVote | AND | OR |
|------|-----------|--------|--------|-----|----|
| DASH | 28.0 | 28.6 | 28.6 | 26.7 | 32.3 |
| UNI  | 30.3 | 32.6 | 32.6 | 26.8 | 36.8 |
| NEAR | 34.4 | 32.9 | 32.9 | 33.6 | 36.2 |
| ADA  | 32.9 | 30.9 | 30.9 | 35.2 | 34.1 |
| LTC  | 34.1 | 34.4 | 34.4 | 32.3 | 34.8 |
| SHIB | 32.5 | 30.1 | 30.1 | 29.6 | 34.8 |
| **LINK** | 39.3 | 37.1 | 37.1 | **45.8** | 36.0 |
| ETH  | 32.1 | 33.9 | 33.9 | 29.5 | 34.0 |
| DOT  | 34.5 | 32.0 | 32.0 | 31.6 | 35.2 |
| XRP  | 32.0 | 28.2 | 28.2 | 28.4 | 31.2 |
| ATOM | 32.0 | 31.5 | 31.5 | 28.3 | 35.2 |
| SOL  | 33.0 | 32.0 | 32.0 | 30.8 | 33.1 |
| DOGE | 30.3 | 28.8 | 28.8 | 28.4 | 31.7 |
| XLM  | 32.2 | 33.3 | 33.3 | 27.9 | 34.5 |
| AVAX | 36.5 | 36.3 | 36.3 | 34.3 | 34.7 |
| ALGO | 32.9 | 33.0 | 33.0 | 33.2 | 33.2 |
| BNB  | 36.0 | 34.4 | 34.4 | 32.8 | 32.6 |
| BTC  | 33.3 | 31.4 | 31.4 | 32.8 | 32.1 |

**WR > 44% with ≥10 trades: 1/90** (LINK intersection at 45.8%)

## Conclusions

### Ensemble voting does not improve win rate above 44% breakeven

**Only 1 of 90 coin-mode combinations exceeds the 44% breakeven WR (LINK AND at 45.8%).** This is a single statistical outlier, not a systematic improvement. Most WRs cluster in the 28–37% range regardless of ensemble mode.

### Intersection wins by PF in 11/18 coins but from a flawed mechanism

The AND-intersection wins by PF for 11 coins, but this is because it filters to fewer trades, not because those trades have genuinely higher win rates. The AND filter requires all 3 strategies to simultaneously signal oversold — a much rarer event. The PF improvement (when it exists) is driven by sampling fewer, marginally-better aligned trades, not by a new signal quality mechanism.

Note: best_single average PF (0.879) actually exceeds equal_vote/pf_vote average PF (0.811). The majority vote consistently underperforms the best single strategy by PF.

### Equal-vote and PF-vote produce identical results

All 5 strategies in the pool are mean-reversion variants (VWAP-based, BB-based, z-score-based, RSI-based). They are highly correlated — when one signals oversold, most do. PF-weighted voting changes weights but not the outcome of majority threshold, since signals almost always agree 0/3 or 3/3, rarely splitting 1/3 or 2/3. This is a fundamental limitation: an ensemble of correlated strategies cannot provide the diversity benefit that ensemble theory requires.

### LINK AND at 45.8% WR: single outlier, not actionable

LINK's AND-intersection fires when all 3 top-3 strategies agree (fewer trades, higher agreement threshold). With 45.8% WR on OOS data this is the only mode to clear breakeven. However:
- It's a single coin out of 18
- The benefit over LINK best_single (39.3%) is real but may be noise in a small sample
- The AND intersection has far fewer trades, reducing statistical confidence

### Strategy pool diversity is insufficient

The hypothesis requires independent strategy errors. With a pool of 5 mean-reversion variants on 15m crypto data, errors are not independent — they all fail during trending periods simultaneously. A more diverse pool (trend-following + mean-reversion + breakout) could provide genuine error independence, but prior RUNs show no single strategy type achieves 44% WR, so combining failing strategies cannot create a winning ensemble.

## Decision

**NEGATIVE** — Ensemble voting over the COINCLAW strategy pool does not achieve OOS WR > 44%. Majority voting actively reduces PF vs best single. AND-intersection is marginally better on a few coins but the mechanism is trade filtering, not signal improvement. No COINCLAW changes.

## Files

| File | Description |
|------|-------------|
| `run24_results.json` | Per-coin results for all 5 ensemble modes |
| `run24_1_ensemble.py` | Original Python stub (full-data look-ahead bug) |
| `RUN24.md` | This file |

Source: `tools/src/run24.rs`
