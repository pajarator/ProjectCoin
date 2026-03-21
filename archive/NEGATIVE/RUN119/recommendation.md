# RUN119 — Vortex Indicator Confirmation: Recommendation

## Hypothesis

**Named:** `vortex_confirm`

Require VI- > VI+ for LONG (negative vortex) and VI+ > VI- for SHORT (positive vortex) as directional entry filter.

## Results

### RUN119.1 — Grid Search (6 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.59 | — | 39.0% | 13,686 | 0% |
| LB14_NS (all VI configs) | +$171.16 | -$189.42 | 40.3% | 6,695 | 69.9% |

**Key findings:**
- ALL 6 VI configs produce identical results — filter rate 69.9%, WR +1.3pp, PnL halved
- VI crossover filter is far too restrictive: VI+/- almost always points the opposite direction from regime entry
- The "negative vortex" condition (VI- > VI+) almost never aligns with regime LONG (z < -2)

## Conclusion

**STRONGLY NEGATIVE.** Vortex Indicator and z-score regime signals are fundamentally opposed: z < -2 (LONG) typically occurs when price is already bouncing up, while VI- > VI+ requires downward momentum. Blocking 70% of entries to gain 1.3pp WR doesn't compensate for the trade count loss.

## Files
- `run119_1_results.json` — Grid search results
- `coinclaw/src/run119.rs` — Implementation
