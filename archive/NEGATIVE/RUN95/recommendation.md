# RUN95 — Scalp Momentum Alignment: Recommendation

## Hypothesis

**Named:** `scalp_momentum_align`

Require scalp entries to align with 15m z-score environment:
- In LONG market mode: only scalp LONG entries allowed
- In SHORT market mode: only scalp SHORT entries allowed
- For scalp LONG: require 15m z < Z_ALIGN_MAX (e.g., -0.5)
- For scalp SHORT: require 15m z > Z_ALIGN_MIN (e.g., +0.5)

## Results

### RUN95.1 — Grid Search (9 configs × 18 coins, 5-month 15m + 1m data)

**NEGATIVE — ALL 9 configs significantly worse than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Blocked |
|--------|------|------|-----|--------|---------|
| BASELINE | +$330.82 | — | 21.1% | 13,486 | 0 |
| ZL0.3_ZS0.3 (best) | +$70.74 | -$260.08 | 19.9% | 3,544 | 12,293 |
| ZL0.7_ZS0.5 (worst) | +$57.16 | -$273.66 | 20.3% | 2,766 | 13,245 |

**Key findings:**
- All configs block 91-98% of scalp entries (12,000+ blocked vs ~3,000 allowed)
- The 15m z-alignment filter is far too restrictive for 1m scalp entries
- Win rate actually decreases slightly with the filter (19.9-20.4% vs 21.1%)
- The filter removes entries that are statistically similar to baseline allowed entries
- Multi-timeframe confirmation at this strictness level destroys scalp's edge entirely

**Why it fails:**
- Scalp operates on 1m timeframes with its own indicators (RSI, stochastic, BB squeeze)
- The 15m z-score is too slow and coarse to filter 1m scalp entries
- Requiring 15m z to be in a specific range (|z| < 0.5) for scalp entries removes the exact entries where scalp has its edge (extreme 1m oversold/overbought within any 15m regime)

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run95_1_results.json` — Grid search results
- `coinclaw/src/run95.rs` — Implementation
