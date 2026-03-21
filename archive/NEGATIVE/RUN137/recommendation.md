# RUN137 — Bollinger Band Width Percentage Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| W200_M35 | +184.91 | -174.15 | 42.6% | 53.2% |
| W100_M35 | +131.04 | -228.02 | 43.4% | 71.4% |

**VERDICT: NEGATIVE** — BBW% squeeze filter increases WR (+4pp) but PnL collapses with 53-98% filter rate. Higher WR doesn't compensate for massive trade reduction.

## Analysis

**Filter mechanism:** BBW% = current Bollinger Bandwidth / max BBW over window. Require BBW% < threshold (e.g., <35%) to enter — only enter during Bollinger squeeze.

**Why it fails:**
1. **Squeeze and z-score extreme are anti-correlated:** A Bollinger squeeze (low volatility) means price hasn't moved much — z-score can't be extreme without volatility.
2. **Massive trade reduction:** 53-98% of entries filtered. Even though WR increases to 42-54%, the remaining trades can't compensate.
3. **Best config (W200_M35):** 53% filter rate, +$185 PnL (vs $359 baseline). The trade quality improvement doesn't offset quantity loss.
4. **BBW% is a volatility proxy:** Volatility-based filters don't work well with mean-reversion at extremes because extremes require volatility to exist.

## Conclusion

No COINCLAW changes. Bollinger squeeze filtering removes too many trades.
