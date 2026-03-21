# RUN140 — Keltner Channel Breakout Filter: NEGATIVE (identical to baseline)

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| P14_M1.5 | +359.05 | 0.00 | 38.9% | 0.0% |
| P14_M2 | +359.05 | 0.00 | 38.9% | 0.0% |
| P14_M2.5 | +359.05 | 0.00 | 38.9% | 0.0% |
| P20_M1.5 | +359.05 | 0.00 | 38.9% | 0.0% |
| P20_M2 | +359.05 | 0.00 | 38.9% | 0.0% |
| P20_M2.5 | +359.05 | 0.00 | 38.9% | 0.0% |
| P30_M1.5 | +359.05 | 0.00 | 38.9% | 0.0% |
| P30_M2 | +359.05 | 0.00 | 38.9% | 0.0% |
| P30_M2.5 | +359.05 | 0.00 | 38.9% | 0.0% |

**VERDICT: NEGATIVE** — All 9 Keltner Channel configs produce identical results to baseline with 0% filter rate. The filter never triggers.

## Analysis

**Filter mechanism:** Keltner Channel (EMA ± MULT × ATR). For LONG, require `close < upper_band` (price has not broken out above). For SHORT, require `close > lower_band` (price has not broken out below).

**Why the filter never triggers:**
1. **ATR expands with the move:** When z-score reaches ±2.0, price has made a significant move. The ATR (which uses the same data) also expands proportionally — the Keltner bands widen to encompass the new price range.
2. **Price is almost always within the channel:** Because both the EMA and ATR are derived from the same closes series, the channel naturally expands to contain price. The condition `close < upper_band` or `close > lower_band` is almost always true.
3. **Larger multipliers make it even less likely to filter:** With MULT=2.5, the bands are even wider, making the filter even less likely to trigger. This explains the monotonic pattern where lower multipliers (P14_M1.5) still don't trigger.

**This is the same mechanism as TZI (RUN136):** Volatility-based channels don't work as filters at z-score extremes because the extremes themselves cause the volatility measure to expand.

## Conclusion

No COINCLAW changes. Keltner Channel filtering is fundamentally incompatible with z-score regime entries.
