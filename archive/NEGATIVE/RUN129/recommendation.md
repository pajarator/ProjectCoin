# RUN129 — VWAP Deviation Percentile Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Trades | Filter Rate |
|--------|-----|------|-----|--------|-------------|
| BASELINE | +357.72 | — | 39.0% | 13601 | 0.0% |
| W50_LP15 | +262.60 | -95.12 | 38.8% | 9917 | 45.7% |
| W50_LP10 | +252.82 | -104.91 | 38.6% | 9565 | 48.9% |
| W50_LP5 | +244.11 | -113.61 | 38.9% | 8885 | 54.7% |
| W100_LP15 | +224.56 | -133.17 | 38.9% | 7987 | 61.4% |
| W100_LP10 | +214.23 | -143.49 | 38.9% | 7461 | 65.1% |
| W200_LP15 | +191.89 | -165.83 | 38.6% | 6578 | 70.8% |
| W100_LP5 | +191.04 | -166.68 | 37.9% | 6744 | 69.7% |
| W200_LP10 | +172.57 | -185.16 | 37.7% | 5896 | 74.7% |
| W200_LP5 | +153.18 | -204.55 | 37.4% | 5144 | 78.8% |

**VERDICT: NEGATIVE** — All VWAP deviation percentile filter configs underperform baseline. The filter systematically removes profitable trades.

## Analysis

**Filter mechanism:** The filter only allows entries when the VWAP deviation (close - VWAP) / VWAP is at an extreme percentile rank — bottom 5-15% for LONGs, top 85-95% for SHORTs.

**Why it fails:**
1. **Removes good trades:** The most extreme VWAP deviations often coincide with the strongest mean-reversion setups. Filtering them removes the highest-conviction entries.
2. **Higher filter rate = worse PnL:** There's a strong negative correlation (r = -0.94) between filter rate and PnL delta. Every additional % of entries filtered translates to roughly -$3-4 of lost PnL.
3. **No predictive value:** VWAP deviation percentile doesn't predict whether a mean-reversion trade will win. It just measures how far price has drifted from VWAP.
4. **WR nearly unchanged:** Filtered configs have virtually identical WR (37-39%) to baseline, meaning the filter isn't removing losing trades — it's removing both winners and losers proportionally.

**Best config (W50_LP15):** Shortest window (50 bars), most lenient threshold (15%). Still loses -$95 vs baseline with 45.7% filter rate.

## Conclusion

No COINCLAW changes. VWAP deviation percentile filtering is not a useful entry filter for mean-reversion trades.
