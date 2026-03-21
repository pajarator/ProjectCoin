# RUN136 — Trade Zone Index Confirmation: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| All TZI configs | +359.05 | 0.00 | 38.9% | 0.0% |

**VERDICT: NEGATIVE** — All TZI configs produce identical results to baseline with 0% filter rate. The TZI threshold is never exceeded — the filter never triggers.

## Analysis

**Filter mechanism:** TZI = % of bars where close is within the recent high/low range. Require TZI > threshold (e.g., >50) to enter, meaning market must be consolidating.

**Why it never triggers:**
1. **Crypto doesn't consolidate enough:** In the 5-month dataset, TZI almost always exceeds 50, 60, and 70 — crypto's constant volatility means price frequently touches both recent highs and lows.
2. **At z-score extremes, TZI is already high:** When price reaches an extreme (z-score ±2.0), it has typically bounced off recent highs/lows, meaning TZI is high.
3. **Effectively inactive:** 0% filter rate means this is identical to baseline.

## Conclusion

No COINCLAW changes. TZI threshold needs to be re-tuned much higher if revisited.
