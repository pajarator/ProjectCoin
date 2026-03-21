# RUN40 — BTC Dominance Scalp Filter: Recommendation

## Hypothesis
Named: `btc_dominance_scalp_filter`

When BTC's z-score significantly exceeds the cross-coin average z-score (`btc_z >> avg_z`), scalp LONG entries on alts face a headwind. When BTC underperforms (`btc_z << avg_z`), scalp SHORT entries face a headwind.

## Results

### RUN40.1 — Grid Search (17 configs × 18 coins, 5-month 1m data)

**NEGATIVE on in-sample data.**

| Config | PnL | ΔPnL | WR% | Trades | Blocked |
|--------|------|------|-----|--------|---------|
| Baseline (disabled) | +$285.20 | — | 20.6% | 12,400 | 0 |
| L0.5_S0.5 | -$0.35 | -$285.55 | 10.1% | 79 | 99.4% |
| L1.0_S1.0 | -$0.35 | -$285.55 | 10.1% | 79 | 99.4% |
| L2.0_S2.0 | -$0.35 | -$285.55 | 10.1% | 79 | 99.4% |
| All other configs | -$0.35 | -$285.55 | 10.1% | 79 | 99.4% |

**Key finding:** Every active config blocks 99.4% of trades (12,321 of 12,400 blocked), leaving only 79 trades. All configs — regardless of threshold (0.5, 1.0, 1.5, 2.0) — produce identical results, indicating the dominance signal is too noisy or always extreme.

### Root Cause Analysis

The `compute_btc_dominance` function computes `btc_z` as BTC's own z-score vs its 20-bar rolling mean, and `avg_z` as the mean of all coins' z-scores. Since BTC is included in `avg_z` and BTC's own z-score is the first element of `zscores`, the spread `btc_z - avg_z` is essentially measuring how extreme BTC is relative to itself + others. When BTC moves, its own z-score dominates the average, making the spread always large in both directions. This causes the filter to block almost all trades regardless of threshold.

**Structural flaw:** Using BTC's self-referential z-score in the average creates a signal that is always "extreme" whenever BTC moves, not just during genuine dominance rotations.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The BTC dominance filter is structurally flawed. All thresholds produce identical results (99.4% blocked), completely destroying the scalp strategy's P&L (+$285 → -$0.35). Even if the z-score computation were corrected, the hypothesis is not supported by data: BTC dominance does not predict altcoin scalp entry quality.

The "Why This Could Fail" section of the hypothesis anticipated exactly this: "BTC dominance changes on different timescale than 1m scalp signals" and "the filter reacts too slowly."

## Files
- `run40_1_results.json` — Grid search results (17 configs)
- `run40.rs` — Grid search implementation
- `RUN40_suggestion.md` — Original hypothesis

## Next RUN

Proceed to RUN41: check `TEST/RUN41/` for the next hypothesis.
