# RUN85 — Momentum Pulse Filter: Recommendation

## Hypothesis
Named: `momentum_pulse_filter`

Require positive short-term momentum (roc_3) for regime entries:
- LONG: roc_3 >= MOMENTUM_PULSE_LONG_MIN
- SHORT: roc_3 <= MOMENTUM_PULSE_SHORT_MAX

## Results

### RUN85.1 — Momentum Pulse Grid Search (17 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 16 configs are catastrophic.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.0% |
| LM0.0005_SM-0.0010 (best) | +$1.10 | -$291.74 | 21.4% | 28 | 99.9% |
| LM0.0010_SM-0.0005 (worst) | +$0.66 | -$292.19 | 17.4% | 23 | 99.9% |

**Key findings:**
- ALL 16 configs block 99.9% of entries (only 19-40 trades vs 9,716 baseline!)
- The roc_3 threshold of 0.05% (0.0005) is FAR too restrictive
- At z-score extreme (-2.0 or +2.0), the 3-bar momentum has likely already started to reverse — requiring additional momentum in our direction eliminates virtually all entries
- This confirms: regime entries work precisely WHEN momentum has NOT yet aligned with our direction (the mean reversion is the signal itself)

**Why it fails:** Mean reversion signals fire at price extremes (z = ±2.0). At that point, the short-term momentum has often ALREADY reversed. Requiring pro-direction momentum on top of an extreme z-score eliminates the edge entirely — the best mean reversion entries are at extremes precisely when the counter-move hasn't started yet. The momentum pulse filter is trying to turn mean reversion into trend following.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run85_1_results.json` — Grid search results
- `coinclaw/src/run85.rs` — Implementation
