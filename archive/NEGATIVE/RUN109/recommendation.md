# RUN109 — Minimum Volume Surge Confirmation: Recommendation

## Hypothesis

**Named:** `min_vol_surge`

Require volume spike (vol ≥ vol_ma × 2.0+) at regime entry to confirm market participation conviction.

## Results

### RUN109.1 — Grid Search (17 configs × 18 coins, 5-month 15m data)

**STRONGLY NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| SL1.5_SS1.5 (best) | +$267.24 | -$93.31 | 38.0% | 9,474 | 48.0% |

**Key findings:**
- ALL volume surge configs produce lower PnL than baseline
- Even the most permissive filter (SL1.5_SS1.5, blocks 48% of entries) reduces PnL by $93
- Higher surge thresholds reduce PnL by up to $250 (SL3.0_SS3.0: $110 vs $361)
- Volume surge requirement is fundamentally counterproductive for mean-reversion: low-volume extremes are valid setups
- The "market participation conviction" thesis is wrong — mean reversion works regardless of volume

## Conclusion

**STRONGLY NEGATIVE.** Volume surge requirements on regime entries are counterproductive. Mean-reversion setups work on low volume; requiring a spike filters out valid opportunities.

## Files
- `run109_1_results.json` — Grid search results
- `coinclaw/src/run109.rs` — Implementation
