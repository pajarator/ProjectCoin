# RUN124 — Choppiness Index Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.59 | — | 39.0% | 0.0% |
| LB10_CT40 | +258.79 | -101.80 | 41.2% | 38.4% |
| LB14_CT40 | +258.79 | -101.80 | 41.2% | 38.4% |

**VERDICT: NEGATIVE** — Best non-baseline config (LB10_CT40) loses -$102 vs baseline. Choppiness filter increases WR (+2.2pp) but PnL still collapses.

## Analysis

**Filter mechanism:** Choppiness Index = 100 × log10(ATR_sum/range) / log10(period). Values > 50 indicate choppy markets (no trend); values < 50 indicate trending markets.

**Why it fails:**
1. **WR increases but PnL decreases:** Higher WR (41.2% vs 39.0%) doesn't help because total trade count drops dramatically (8,298 vs 13,664).
2. **Inverse relationship:** Choppiness is inversely related to volatility. When the market is choppy (high CI), mean-reversion should theoretically work better. But the filter blocks entries during chop, which is when the strategy should work best.
3. **Conflicting signals:** The z-score regime strategy already adapts to market conditions. Adding CI filtering creates conflicting signals.

## Conclusion

No COINCLAW changes. Choppiness Index filtering doesn't improve the z-score regime strategy.
