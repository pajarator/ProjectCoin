# RUN11 — STRAT1000 Strategy Discovery

## Goal
Test ~45 new strategies from the STRAT1000 list (1000 AI-generated strategy names filtered down to implementable candidates) to find strategies that beat current COINCLAW v10 assignments.

## Method
Three sub-runs, all Rust + rayon parallel processing:
- **RUN11a**: 18 price action + volume strategies (no new indicators needed)
- **RUN11b**: 15 new indicator strategies (Aroon, Hull MA, KAMA, TRIX, Vortex, Elder Ray, Awesome Oscillator, Heikin Ashi, BB+Keltner Squeeze, ATR, Donchian, etc.)
- **RUN11c**: 12 structural/algorithmic strategies (ORB, Pivot Points, Fibonacci, OU half-life, Hurst, etc.)

Each follows the standard X.1 grid search → X.2 walk-forward → X.3 comparison lifecycle.

## Critical Implementation Fix
During RUN11a, discovered two incorrect implementations:

1. **Stochastic RSI was backwards**: Computed `RSI(stochastic_k)` instead of correct `Stochastic(RSI)` formula (Chande & Kroll 1994). The wrong formula happened to produce a spurious edge on DASH that disappeared when corrected.

2. **Effort vs Result (VSA) used wrong spread measure**: Used fixed 0.3% close-to-close threshold instead of relative high-low range vs average. Also missing close-position-within-bar check (core VSA element). Corrected to: relative spread < 40-70% of avg_range20, close in upper half of bar.

After corrections, both strategies were re-tested across all 19 coins.

## RUN11a Results

### 11a.1 Grid Search (corrected implementations)
- 19 coins × 204 strategy configs × 7 exit modes = ~27,000 backtests
- **4 winners** (WR≥60%, T≥30, PF≥1.2):

| Coin | Strategy | Exit | Trades | WR% | PF | P&L% |
|------|----------|------|--------|-----|-----|------|
| SHIB | effort_result (1.2/0.7/z-0.5) | signal_only | 31 | 67.7% | 2.29 | +0.7% |
| XLM | effort_result (1.2/0.7/z-1.0) | signal_only | 31 | 64.5% | 1.49 | +0.3% |
| SHIB | effort_result (1.2/0.7/z-0.5) | signal_10 | 31 | 64.5% | 1.30 | +0.3% |
| ADA | inside_bar | signal_only | 30 | 63.3% | 1.25 | +0.2% |

### 11a.2 Walk-Forward Validation
- 7 target coins × 3 windows (train 2mo, test 1mo)
- **Only DASH passes OOS**: avg OOS WR=65.0%, PF=1.26, WR degradation +4.0%
- All other coins fail: SHIB 28.2%, XLM 33.4%, ALGO 45.8% OOS WR

### 11a.3 Side-by-Side Comparison (DASH)
Compared against DASH's current VwapReversion (COINCLAW v10):
- VwapReversion baseline: WR=62.6%, PF=0.86, P&L=-3.7% (signal_only, 270 trades)
- effort_result (corrected VSA): WR=57.0%, PF=2.03, P&L=+3.9% (signal_05, 107 trades)
- Trade counts too thin for definitive replacement recommendation

## RUN11b Results

### 11b.1 Grid Search
- 19 coins × 132 strategy configs × 7 exit modes = ~17,500 backtests
- **3 winners** (WR≥60%, T≥30, PF≥1.2):

| Coin | Strategy | Exit | Trades | WR% | PF | P&L% |
|------|----------|------|--------|-----|-----|------|
| DASH | kama_revert (0.02/z-1.5) | signal_only | 55 | 61.8% | 1.37 | +2.3% |
| DOT | chandelier_entry | signal_10 | 37 | 62.2% | 1.23 | +0.2% |
| LINK | vortex_cross | tp_sl_10_20 | 31 | 61.3% | 1.45 | +0.6% |

- High WR results (heikin_ashi 72.7%, trix_cross 70.6%) had <25 trades — insufficient sample
- Walk-forward validation not run — results too weak to justify

## RUN11c Results

### 11c.1 Grid Search
- 19 coins × 112 strategy configs (12 strategies × z-filters) × 5 exit modes
- **12 structural/algorithmic strategies**: ORB, Pivot Point Reversion, Fibonacci Retracement, Percentile Rank, OU Mean Reversion, Hurst Filter, Acceleration Reversal, Dual Thrust, Impulse Follow, Gap and Go, Momentum Shift, VWAP+ATR Reversion
- **Key winner**: ou_mean_rev on DASH — 74.1% WR, PF=1.92, 54 trades (signal_only, halflife=10, dev=2.0)

### 11c.2 Walk-Forward Validation
- All 19 coins × 56 configs × 3 exit modes × 3 windows (train 2mo, test 1mo)
- **DASH ou_mean_rev is the only coin to pass**:
  - Avg OOS WR: 68.6%, PF: 1.49
  - WR degradation: -10.9% (actually improved OOS vs training)
  - Consistent across all 3 windows

### 11c.3 Side-by-Side Comparison (DASH)
Full 5-month head-to-head, DASH's VwapReversion baseline vs ou_mean_rev:

| Strategy | Exit | Trades | WR% | PF | P&L% |
|----------|------|--------|-----|-----|------|
| vwap_rev (baseline) | signal_only | 270 | 62.6% | 0.86 | -3.7% |
| ctrl_mean_rev | signal_only | 293 | 64.8% | 0.90 | -3.0% |
| ou_dev1.5 | signal_only | 70 | 67.1% | 1.86 | +2.8% |
| **ou_dev2.0** | **signal_only** | **54** | **74.1%** | **1.92** | **+2.5%** |
| ou_dev2.5 | signal_only | 22 | 81.8% | 2.27 | +1.4% |

- VwapReversion has PF < 1.0 on DASH (losing money)
- ou_dev2.0 is the sweet spot: 74.1% WR, PF=1.92, sufficient trades (54)
- ou_dev2.5 has 81.8% WR but only 22 trades — too few for confidence
- Per-month: ou_dev2.0 hits 80%+ WR in months 1, 3, 4; worst month 58.3%

### Action Taken
**DASH long strategy changed from VwapReversion to OuMeanRev** (halflife=10, deviation threshold=2.0σ).
Updated COINCLAW v10 → v11. Required increasing 15m candle fetch from 50 to 150 (OU needs 100-bar rolling window).

## Key Finding
The z-score filter does all the heavy lifting for most strategies. All 33 tested strategies in RUN11a/11b had their best results with z_filter=-0.5 to -1.5, just filtering into subsets of mean-reversion entries.

However, **Ornstein-Uhlenbeck half-life estimation provides genuine independent edge** — it mathematically confirms the market is in a mean-reverting regime before entering, rather than just checking if price is extended. This is the only strategy from STRAT1000 that adds real value.

## Conclusion
- Of 45 new strategies tested across 3 sub-runs (RUN11a/b/c), only **1 strategy on 1 coin** justified a change
- **ou_mean_rev on DASH**: 74.1% WR, PF=1.92, validated OOS with 68.6% WR — replaces VwapReversion which was PF<1.0
- The OU half-life test works because it adds regime detection (is mean-reversion actually happening?) on top of deviation measurement
- The original StochRSI "edge" was from a wrong formula — a cautionary tale about implementation verification
- Properly implemented VSA (effort_result) shows real absorption detection but too few signals on 15m
- COINCLAW v11: DASH uses OuMeanRev, all other coins unchanged from v10
