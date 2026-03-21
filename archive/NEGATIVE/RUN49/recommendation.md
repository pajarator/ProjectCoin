# RUN49 — Cross-Coin Correlation Filter: Recommendation

## Hypothesis
Named: `correlation_cluster_filter`

Suppress correlated entries: when multiple coins generate simultaneous entry signals in the same direction, suppress the weaker signal (lower z-score deviation) to prevent clustered drawdown from correlated positions.

## Results

### RUN49.1 — Grid Search (41 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All correlation filter configs produce drastically worse PnL than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Suppressed |
|--------|------|------|-----|--------|-----------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0 |
| T0.85_W30_W (best filter) | -$1.14 | -$178.11 | 25.1% | 11,751 | 9,984 |
| T0.60_W15_W (most suppressed) | -$1.34 | -$178.31 | 24.7% | 7,357 | 17,821 |

**Key findings:**
- ALL 40 correlation filter configs produce catastrophic PnL degradation (~$178 loss vs +$177 baseline)
- The filter suppresses 10K-18K signals (66-118% of baseline trades), meaning it blocks nearly all signals
- Suppressed signals were never executed, so the WR of executed trades (24-25%) is catastrophically lower than baseline
- The correlation buffer implementation appears to mark most same-direction signals as correlated, suppressing the majority of entries
- Portfolio PnL with correlation filter is near zero regardless of threshold/window/mode

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The correlation filter approach is fundamentally flawed in this implementation. The rolling return correlation at short windows (10-30 bars) is too unstable — most coins in the same market regime show high correlation, causing the filter to suppress nearly all entries rather than only truly clustered ones. The executed trades (non-suppressed) have catastrophically lower WR (25% vs 55%), suggesting the filter removes the good entries and leaves only the worst ones.

## Files
- `run49_1_results.json` — Grid search results
- `coinclaw/src/run49.rs` — Implementation
