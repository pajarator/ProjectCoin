# RUN123 — A/D Line Divergence: Recommendation

## Hypothesis

**Named:** `ad_line_divergence`

Use Accumulation/Distribution (A/D) Line slope as entry confirmation filter. Require A/D slope > threshold for LONG (rising = accumulation), and A/D slope < -threshold for SHORT (falling = distribution).

## Results

### RUN123.1 — Grid Search (9 configs × 18 coins, 5-month 15m data)

**STRONGLY NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.59 | — | 39.0% | 13,686 | 0% |
| SP10 (all SM) | +$56.97 | -$303.62 | 43.3% | 2,252 | 91.9% |
| SP3 (all SM) | +$49.05 | -$311.54 | 40.4% | 2,151 | 92.4% |
| SP5 (all SM) | +$37.34 | -$323.24 | 39.8% | 1,849 | 93.5% |

**Key findings:**
- A/D slope threshold (SM values) has zero effect — slope is always tiny relative to cumulative A/D drift
- Filter rate: 91.9–93.5% — catastrophic over-filtering
- A/D slope threshold of SP10 gives +4.3pp WR improvement but -$304 PnL
- Cumulative A/D line grows unboundedly; slope comparisons across different lookbacks are meaningless for thresholding

## Conclusion

**STRONGLY NEGATIVE.** A/D Line cumulative nature makes slope-based thresholding impractical in crypto. The cumulative sum diverges over time, making absolute slope thresholds meaningless across different slope periods. The filter blocks 92%+ of entries, removing the bulk of profitable trades. No COINCLAW changes.

## Files
- `run123_1_results.json` — Grid search results
- `coinclaw/src/run123.rs` — Implementation
