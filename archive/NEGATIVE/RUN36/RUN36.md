# RUN36 — Scalp Choppiness Detector + Bayesian Pause/Resume

## Hypothesis
Scalp entries succeed or fail in bursts. The market alternates between "trending" (scalp-friendly) and "choppy" (scalp-hostile) regimes. If we can detect choppiness early using a market-wide or trade-history-based filter, we can pause scalp entries during bad regimes and resume when the regime shifts back to favorable — improving overall win rate and P&L.

## Pattern Description
- Good scalp entries/exits → bad scalp entries/exits → good again
- Timescale: minutes to hours (1m candles)
- All coins simultaneously enter choppy state → suggests market-wide regime shift

## Methods Tested

### Method 1: Choppiness Index (CI)
Standard CI = 100 * log10(sum ATR over N) / (log10(N) * (HH - LL) / LL)
- CI > 75 = choppy
- N = 20/40/60 bars
- Grid: 3 window sizes × 3 pause durations = 9 configs

### Method 2: ADX-based (replaced ZStd which didn't trigger)
Low ADX = choppy (no clear trend)
- Choppy if ADX[i] < rolling_avg_ADX[i] * 0.5
- Grid: 3 window sizes × 3 pause durations = 9 configs
- **Issue: ADX method did not trigger (0% pause for all configs)**

### Method 3: Bayesian Win-Rate Regime
Track rolling win rate of recent scalp trades.
- Prior: P(choppy) = 0.286 (Beta(2,5) prior)
- Likelihoods: P(WIN|choppy) = 0.25, P(WIN|trending) = 0.55
- Bayesian update after each trade
- Threshold 0.6/0.7/0.8 → pause scalp entries
- Grid: 3 thresholds × 3 pause durations = 9 configs

### Method 4: Consecutive Loss Detection
After N consecutive scalp losses, enter "pause mode"
- N = 3/4/5 consecutive losses
- Resume after pause_bars bars
- Grid: 3 consec thresholds × 3 pause durations = 9 configs

## Results Summary

| Config     | Trades | WR%   | PnL$     | ΔWR    | ΔPnL$    | Pause% |
|------------|--------|-------|----------|--------|----------|--------|
| baseline   | 27283  | 20.4% | +$678.71 | —      | —        | 0.0%   |
| ci20_p5    | 21510  | 20.3% | +$515.65 | -0.1pp | -$163.07 | 24.2%  |
| ci60_p20   | 17328  | 20.2% | +$411.60 | -0.2pp | -$267.12 | 38.4%  |
| bay06_p5   | 6068   | 21.2% | +$142.47 | +0.8pp | -$536.25 | 81.9%  |
| bay06_p20  | 1935   | 22.1% | +$48.53  | +1.7pp | -$630.18 | 94.2%  |
| cs3_p5     | 9980   | 20.9% | +$234.14 | +0.5pp | -$444.58 | 66.5%  |
| cs5_p5     | 12874  | 20.7% | +$300.68 | +0.3pp | -$378.03 | 54.9%  |

**Best by PnL: baseline** (+$678.71)
**Best by WR: bay06_p20** (22.1% WR but -$630 PnL)

## Analysis

### Why No Config Improves PnL
1. **Baseline WR is 20.4% vs breakeven ~27.8%** (accounting for 0.1% SL and 0.08% TP)
   - Scalping is already deeply underwater at realistic fee levels
   - Zero-fee PnL = +$678.71; at realistic fees the strategy would be highly negative
   - Any filter reduces trade count, so even if WR improves, total edge declines

2. **CI filter**: Triggers 24-38% of the time but reduces PnL by $163-267
   - The filtered trades (during CI > 75) are NOT all losers
   - CI is a trend strength measure, not a scalp-specific measure
   - 24-38% pause is too aggressive for the marginal improvement

3. **ADX method**: Never triggered (0% pause)
   - The ADX-based condition (ADX < 0.5 * rolling_avg) is too strict
   - ADX rarely drops to half its rolling average
   - **Fix needed**: Use ADX directly < absolute threshold (e.g., ADX < 20)

4. **Bayesian WR**: Most aggressive filtering (82-94% pause)
   - WR improves +0.8-1.7pp but trade count drops 80-94%
   - PnL drops -$535 to -$630
   - The filter is TOO good at detecting bad regimes — it pauses almost everything

5. **ConsecLoss**: Least bad filtering (55-89% pause)
   - cs5_p5 (pause 5 bars after 5 consecutive losses): best PnL among filtered configs
   - Still loses -$378 vs baseline, but smallest degradation
   - Trade count drops 55% with only +0.3pp WR improvement

## Per-Coin Breakdown (baseline)
| Coin | Trades | WR%   | PnL$   |
|------|--------|-------|--------|
| DASH | 2012   | 24.9% | +$86.49 |
| UNI  | 1857   | 20.8% | +$50.03 |
| NEAR | 1694   | 21.1% | +$46.23 |
| ADA  | 1718   | 20.0% | +$40.44 |
| ETH  | 1736   | 20.5% | +$43.46 |
| BTC  | 864    | 17.8% | +$12.74 |

## Method Summary
| Method     | Best Config  | ΔPnL$   | ΔWR    | Pause% |
|------------|-------------|---------|--------|--------|
| CI         | baseline    | +$0.00  | —      | 0%     |
| ADX        | baseline    | +$0.00  | —      | 0%     |
| BayesianWR | bay08_p5    | -$535.21| +0.8pp | 81.6%  |
| ConsecLoss | cs5_p5     | -$378.03| +0.3pp | 54.9%  |

## Conclusion
**RESULT: NEGATIVE**

No choppiness filter improves PnL. All methods either:
1. Don't trigger at all (ADX-based)
2. Trigger too much and destroy edge (Bayesian WR)
3. Reduce trades without enough WR improvement to offset (CI, ConsecLoss)

The fundamental issue: scalp entries have a ~20.4% WR at zero fees. With realistic fees (~0.04-0.06% per trade round trip), breakeven WR is ~27-28%. The strategy is already a significant loser with fees; filtering trades cannot save it.

The "choppiness" pattern the user describes is real (clusters of bad trades), but detecting it with market-wide indicators (CI, ADX) or trade history (Bayesian, ConsecLoss) does not provide actionable filtering that improves outcomes.

### Recommendations for Future Runs
1. **Scalp-specific churn measure**: Instead of market-wide indicators, measure scalp-specific local z-score oscillation rate (how many times does z-score cross zero in the last N bars?)
2. **Smaller pause windows**: ConsecLoss with pause_bars=3 and consec=3-4 might capture the chop burst without over-pausing
3. **Inverse filter**: Instead of pausing during chop, consider ONLY trading during "confirmed trending" periods (high ADX, low CI)
4. **Coin-specific choppiness**: Different coins may have different chop characteristics
5. **ADX fix**: Use ADX < absolute threshold (e.g., 20) rather than relative to rolling average

## Files
- `RUN36.prompt` — original hypothesis prompt
- `run36_1_results.json` — full grid search results
- `coinclaw/src/run36.rs` — Rust grid search implementation

---

## UPDATE: Absolute ADX Threshold Test (2026-03-19)

### Bug Fixed
Initial run had a bug: `compute_1m_data` computed ADX but stored it in a field that wasn't being used by the simulation loop. The ADX values were all 0.0 (default), so the filter never triggered. After fixing, ADX distribution (DASH 1m, n=526,622):
- p5=13.9, p25=22.9, median=32.7, p75=45.2, max=100.0
- ADX < 15: 6.8%, < 20: 17.7%, < 25: 30.4%, < 30: 43.4%

### Also Fixed: NaN propagation in rolling mean
`rmean()` was propagating NaN through the rolling window. Added `rmean_nan()` (NaN-aware rolling mean) used by `compute_atr` and `compute_adx`.

### ADX Absolute Threshold Results (4 thresholds × 3 pause durations = 12 configs)

| Config | Pause% | WR% | ΔWR | PnL$ | ΔPnL$ |
|--------|--------|-----|------|------|--------|
| adx15_p5 | 8.3% | 20.4% | +0.0pp | 658.74 | -19.97 |
| adx15_p10 | 12.1% | 20.5% | +0.1pp | 623.23 | -55.48 |
| adx15_p20 | 19.8% | 20.4% | +0.0pp | 546.55 | -132.16 |
| adx20_p5 | 18.4% | 20.4% | +0.0pp | 616.99 | -61.72 |
| adx20_p10 | 24.4% | 20.4% | +0.0pp | 550.64 | -128.08 |
| adx20_p20 | 35.4% | 20.6% | +0.3pp | 456.89 | -221.82 |
| adx25_p5 | 29.6% | 20.5% | +0.2pp | 567.60 | -111.12 |
| adx25_p10 | 37.1% | 20.5% | +0.2pp | 483.42 | -195.29 |
| adx25_p20 | 49.4% | 20.7% | +0.3pp | 368.31 | -310.41 |
| adx30_p5 | 40.3% | 20.4% | -0.0pp | 490.21 | -188.51 |
| adx30_p10 | 49.0% | 20.2% | -0.2pp | 391.09 | -287.63 |
| adx30_p20 | 61.3% | 20.5% | +0.1pp | 286.52 | -392.19 |

**Best ADX: adx15_p5** (8.3% pause) — $658.74 PnL, only $20 less than baseline.

### Method Summary
| Method | Best Config | PnL$ | ΔPnL$ | Best WR |
|--------|------------|-------|--------|---------|
| CI | baseline | 678.71 | — | 20.4% |
| AdxAbs | adx15_p5 | 658.74 | -19.97 | 20.4% |
| BayesianWR | bay08_p5 | 143.51 | -535.21 | 21.2% |
| ConsecLoss | cs5_p5 | 300.68 | -378.03 | 20.7% |

### Conclusion
**Still NEGATIVE.** All 40 configs perform worse than baseline. The ADX filter now correctly triggers, but:
1. Even the mildest filter (8% pause) loses $20 vs baseline
2. The filtered trades are not all losers — removing them reduces edge without improving WR enough
3. Baseline WR = 20.4% vs breakeven ~27.8% at realistic fees — the strategy has no real edge
4. Market-wide ADX filtering doesn't selectively remove only the bad entries
