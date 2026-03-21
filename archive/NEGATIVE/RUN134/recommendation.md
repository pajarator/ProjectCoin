# RUN134 — Connors RSI Confirmation: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +357.72 | — | 39.0% | 0.0% |
| LM30_SM70 | +300.41 | -57.32 | 38.7% | 47.7% |
| LM30_SM75 | +283.98 | -73.74 | 38.5% | 52.9% |

**VERDICT: NEGATIVE** — All Connors RSI configs underperform baseline. Best config (LM30_SM70) loses -$57 with 47.7% filter rate. WR barely changes while PnL collapses.

## Analysis

**Filter mechanism:** Connors RSI combines (1) RSI(price,3), (2) RSI(streak,2), (3) percentile rank of price. LONG requires Connors RSI < threshold.

**Why it fails:**
1. **All configs lose PnL:** Best non-baseline config loses -$57. The filter removes more profitable trades than it saves.
2. **47-78% filter rate:** Significant filtering with no PnL improvement.
3. **Connors RSI responds differently than expected:** The streak component is sensitive to consecutive closes. In crypto's volatile markets, consecutive closes in one direction don't reliably predict mean-reversion success.
4. **No WR improvement:** WR remains 37-39% across all configs, identical to baseline.

## Conclusion

No COINCLAW changes. Connors RSI provides no useful filtering for the z-score regime strategy.
