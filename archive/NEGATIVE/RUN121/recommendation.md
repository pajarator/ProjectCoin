# RUN121 — TD Sequential Entry Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.55 | — | 39.0% | 0.0% |
| SL9_LB8_S | +13.17 | -347.39 | 42.4% | 98.8% |

**VERDICT: NEGATIVE** — TD Sequential filter collapses PnL to near-zero with 98.8-100% filter rate. The consecutive close comparison (close > close_4_bars_ago for LONG) is almost never true at z-score extreme entry moments.

## Analysis

**Filter mechanism:** TD Sequential requires consecutive closes in one direction before entry. For LONG: require N consecutive closes higher than the close 4 bars prior.

**Why it fails:**
1. **98-100% filter rate:** The z-score extreme (±2.0) and TD Sequential condition are nearly mutually exclusive. When z-score is extreme, price has usually already made a sharp move — the TD condition requires persistence that conflicts with the sharp move needed to reach z-score extreme.
2. **Grid search exhaustively tested:** All 12 configs with different SL/LB/STRICT combinations showed 98-100% filter rate.
3. **WR increases slightly (42.4% vs 39.0%)** but the remaining ~1% of trades can't compensate for the lost opportunity.

## Conclusion

No COINCLAW changes. TD Sequential is incompatible with z-score extreme entries.
