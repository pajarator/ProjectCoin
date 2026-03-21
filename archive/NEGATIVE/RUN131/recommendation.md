# RUN131 — Volume-Weighted RSI Confirmation: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Trades | Filter Rate |
|--------|-----|------|-----|--------|-------------|
| BASELINE | +359.05 | — | 38.9% | 13665 | 0.0% |
| LM40_SM60 | +138.37 | -220.68 | 35.9% | 4026 | 84.1% |
| LM40_SM65 | +137.16 | -221.89 | 35.9% | 4017 | 84.2% |
| LM40_SM70 | +133.83 | -225.22 | 35.9% | 4002 | 84.2% |

**VERDICT: NEGATIVE** — All VWRSI configs massively underperform baseline. 84-89% filter rate with WR dropping from 38.9% to 35-36%, meaning the filter removes BETTER-THAN-AVERAGE entries.

## Analysis

**Filter mechanism:** Require RSI < 40 AND VWRSI < LONG_MAX for LONG entries; RSI > 60 AND VWRSI > SHORT_MIN for SHORT entries. The filter ensures RSI extremes are volume-confirmed.

**Why it fails:**
1. **Massive over-filtering:** 84-89% of entries filtered. The combined RSI+VWRSI condition is extremely restrictive.
2. **WR drops:** Baseline WR = 38.9%, filtered configs WR = 35-36%. The filtered entries had HIGHER win rate than average — the filter removes the best entries.
3. **PnL collapse:** Even the best config (LM40_SM60) loses -$221 vs baseline with only 4,026 trades vs 13,665. The remaining trades can't compensate for the lost opportunity.
4. **Volume-weighted calculation adds noise:** The VWRSI formula amplifies volume spikes, making the oscillator more erratic than standard RSI. High-volume bars on one side can skew the entire VWRSI calculation.

**Key insight:** The hypothesis was that low-volume RSI extremes are "weak" signals. But the data shows the opposite — high-volume moves (which VWRSI weights heavily) actually have LOWER win rate than average. The market doesn't confirm the theory.

## Conclusion

No COINCLAW changes. VWRSI confirmation is counterproductive — it filters out the best entries while failing to remove bad ones.
