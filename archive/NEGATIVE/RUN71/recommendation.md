# RUN71 — BB Width Percentile Rank Filter: Recommendation

## Hypothesis
Named: `bb_width_percentile_filter`

Block entries when Bollinger Band width is above a percentile threshold (BB not compressed enough). Only enter when BB width is historically tight.

## Results

### RUN71.1 — BB Percentile Grid Search (13 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All BB percentile configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Blocked |
|--------|------|------|-----|--------|---------|
| BASELINE (no filter) | +$176.97 | — | 54.7% | 15,101 | 0 |
| W100_T50 (best) | +$71.26 | -$105.71 | 54.3% | 8,462 | 13,040 |
| W300_T50 | +$64.41 | -$112.56 | 53.9% | 8,680 | 12,517 |
| W100_T20 (worst) | +$28.15 | -$148.82 | 53.9% | 3,982 | 22,218 |

**Key findings:**
- ALL BB percentile configs produce lower PnL than baseline
- Block rate is very high (45-80%) — the filter is too restrictive
- WR barely changes (53.6-54.3%) — BB width is not predictive of trade quality
- Trade count reduction dominates any marginal WR improvement

**Why it fails:** BB width percentile does not predict whether a regime trade will be profitable. Blocking entries when BB width is "too wide" removes entries that are, on average, equally profitable as those that pass.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run71_1_results.json` — Grid search results
- `coinclaw/src/run71.rs` — Implementation