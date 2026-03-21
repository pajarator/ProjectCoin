# RUN135 — Stress Accumulation Meter: NEGATIVE (technically weakly positive, effectively neutral)

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| W15_S80 | +359.29 | +0.24 | 39.0% | 0.1% |
| W15_S70 | +359.14 | +0.08 | 39.0% | 0.2% |

**VERDICT: NEGATIVE** — Technically POSITIVE (W15_S80: +$0.24 above baseline), but the effect is effectively zero. Only 14 entries filtered (0.1% filter rate) — filter never triggers.

## Analysis

**Filter mechanism:** Track consecutive directional bars (bar_streak). When abs(streak)/window >= threshold, market is "overstressed" and entries should be suppressed.

**Why the filter barely triggers:**
1. **High bar streak is rare at z-score extremes:** When z-score reaches ±2.0, price has usually made a sharp move — not a sustained streak of consecutive same-direction bars.
2. **Window too short for meaningful stress:** Even with W5, 5 consecutive same-direction bars at the 15m timeframe is rare in crypto's choppy markets.
3. **Effectively neutral:** With 0.1-0.2% filter rate, the filter is inactive for all practical purposes.

## Conclusion

No COINCLAW changes. The SAM filter is essentially inactive with these parameters.
