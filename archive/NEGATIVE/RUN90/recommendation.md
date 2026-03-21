# RUN90 — Symmetry Exit: Recommendation

## Hypothesis

**Named:** `symmetry_exit`

Add symmetric TP: when |z_at_entry| >= Z_MIN, exit at TP = entry ± SL_dist × RATIO. Enforces fixed R:R on extreme z entries.

## Results

### RUN90.1 — Grid Search (36 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL configs worse than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | PF | SymTP% |
|--------|------|------|-----|--------|-----|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 | 0.0% |
| R2.5_Z2.5_MH2 (best) | +$248.70 | -$44.15 | 30.6% | 10,315 | 0.44 | 11.4% |
| R1.0_Z1.5_MH2 (worst) | -$11.13 | -$303.98 | 42.5% | 12,279 | 0.23 | 39.6% |

**Key findings:**
- ALL 36 configs produce lower PnL than baseline
- WR improves significantly: 30-52% vs 25.9% baseline (symmetry TP exits are wins)
- But total PnL drops because the fixed TP (e.g., 0.45% at R=1.5) is less than what full mean reversion would have captured
- Lower Z_MIN (1.5) configs are catastrophic — they fire on entries that haven't reached the -2.0 regime threshold
- Higher ratio (2.5) is less damaging than lower ratio (1.0) because TP is farther from entry

**Why it fails:** The symmetry TP cuts short the best trades. Mean reversion from z = -2.0 to z = 0 often produces larger gains than the fixed 1.5× or 2.5× SL distance. Exiting early at a fixed TP leaves money on the table from trades that would have been bigger winners. The win rate improvement doesn't compensate for the reduced average win size.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run90_1_results.json` — Grid search results
- `coinclaw/src/run90.rs` — Implementation
