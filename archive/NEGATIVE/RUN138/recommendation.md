# RUN138 — Opening Range Breakout Filter: STRONGLY NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.55 | — | 39.0% | 0.0% |
| B3_T20 | +42.66 | -317.90 | 40.2% | 92.6% |

**VERDICT: STRONGLY NEGATIVE** — ORB filter achieves 92-93% filter rate and collapses PnL to +$40. The requirement that price must be near the range boundary for entry is almost never met at z-score extremes.

## Analysis

**Filter mechanism:** Require price to be near the high or low end of the recent N-bar range for entry. For LONG, price must be near the low end (within buffer).

**Why it fails:**
1. **92-93% filter rate:** When z-score reaches ±2.0, price has almost always already moved significantly from the range boundary.
2. **Range-bound entries rarely coincide with extremes:** The filter requires price to be "in the range" — but an extreme z-score means price has departed significantly from the range.
3. **Small WR improvement (40.2% vs 39.0%)** is irrelevant with 92% of trades filtered.

## Conclusion

No COINCLAW changes. ORB filter is fundamentally incompatible with z-score regime entries.
