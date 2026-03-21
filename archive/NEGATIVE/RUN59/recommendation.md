# RUN59 — Same-Direction Consecutive Signal Suppression: Recommendation

## Hypothesis
Named: `same_dir_suppression`

After a loss (SL exit), suppress re-entries in the same direction for N bars to avoid "doubling down" on a failing trade.

## Results

### RUN59.1 — Same-Direction Suppression Grid Search (9 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All suppression configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Suppress% |
|--------|------|------|-----|--------|-----------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| SB4_M1 (best) | +$171.11 | -$5.86 | 54.7% | 14,746 | 2.4% |
| SB12_M2 (worst) | +$146.46 | -$30.50 | 54.8% | 12,836 | 15% |

**Key findings:**
- ALL 8 suppression configs produce lower PnL than baseline
- WR improves slightly (+0.2-0.3pp) but trade count reduction dominates
- Mode 1 (after_SL_only) performs better than Mode 2 (after_any_loss)
- SB4_M1 (4-bar suppression after SL only) is least harmful but still loses $5.86
- Suppression removes winning re-entries along with losing ones — net effect is negative

**Why it fails:** Trade count reduction (2-15%) exceeds the benefit of avoiding losing re-entries. The surviving suppressed trades are more random than beneficial.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Same-direction suppression does not improve COINCLAW's regime strategy. Trade count reduction dominates any benefit from avoiding consecutive same-direction losses.

## Files
- `run59_1_results.json` — Grid search results
- `coinclaw/src/run59.rs` — Implementation
