# RUN139 — Darvas Box Filter: STRONGLY NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.55 | — | 39.0% | 0.0% |
| P14_T20 | +41.25 | -319.30 | 40.0% | 93.2% |

**VERDICT: STRONGLY NEGATIVE** — All Darvas Box configs collapse PnL with 93-95% filter rate. Mean-reversion entries almost never coincide with price being within the Darvas box boundary.

## Analysis

**Filter mechanism:** Darvas Box defines consolidation as price within N-period high-low range. For LONG entries, price must be near the box bottom; for SHORT, near the box top.

**Why it fails:**
1. **93-95% filter rate:** When z-score reaches ±2.0 (entry trigger), price has almost always already broken out of any recent consolidation range — by definition, an extreme z-score means price has moved significantly from its recent range.
2. **Inverse relationship:** The filter attempts to enter mean-reversion trades ONLY when price is still in consolidation. But extreme z-scores and consolidation are mutually exclusive — you can't have both.
3. **Darvas concept mismatch:** Darvas Boxes work for trend-following breakout systems, not mean-reversion entries at extreme levels.

## Conclusion

No COINCLAW changes. Darvas Box filter is fundamentally incompatible with z-score extreme entries.
