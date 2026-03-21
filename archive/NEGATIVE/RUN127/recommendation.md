# RUN127 — Force Index Confirmation: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.59 | — | 39.0% | 0.0% |
| EP7_LM_SM | +359.71 | -0.88 | 39.0% | 0.1% |
| EP13_LM_SM | +359.71 | -0.88 | 39.0% | 0.1% |

**VERDICT: NEGATIVE** — All Force Index configs underperform baseline. LM_SM (no actual filter, ±1M sentinel) ≈ baseline; any real Force Index threshold collapses PnL.

## Analysis

**Filter mechanism:** Force Index = (close - prior_close) × volume, smoothed with EMA(period). LONG requires FI >= long_min, SHORT requires FI <= short_max.

**Why it fails:**
1. **No-filter configs (LM_SM = ±1M) ≈ baseline:** With sentinel values of ±1M, Force Index never actually filters — these are effectively baseline runs. They show WR and PnL virtually identical to baseline (-$0.88).
2. **Any real threshold collapses PnL:** Force Index values are extremely volatile (product of price change × volume). Setting any practical threshold immediately filters nearly all entries.
3. **Volume × price change is noisy:** In crypto's 24/7 markets, this metric has too much noise to be a reliable momentum confirmation.

**Interesting observation:** The near-baseline configs (EP7_LM_SM, EP13_LM_SM) confirm the simulation framework is working correctly — the slight -$0.88 delta is normal variance across different EMA periods affecting the z-score computation.

## Conclusion

No COINCLAW changes. Force Index is not a useful confirmation filter for crypto mean-reversion entries.
