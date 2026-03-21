# RUN46 — Partial Reversion Exit: Recommendation

## Hypothesis
Named: `partial_reversion_exit`

Exit when a fixed % of the z-score reversion has been captured (e.g., enter at z=-2.5, exit at z=-0.875 after 65% reversion).

## Results

### RUN46.1 — Grid Search (6 configs × 18 coins, 5-month 15m data)

**NEGATIVE — partial reversion exit never triggers; all configs identical.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| R50 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| R60 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| R70 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| R80 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| R90 | +$176.97 | +$0.00 | 54.7% | 15,101 |

**Key finding:** All configs produce identical results, meaning the partial reversion condition never fires before the standard Z0 crossback exit triggers. The z-score reversion percentage from entry to any bar is almost never ≥50%.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The z-score reversion exit hypothesis is not supported by data. The z-score at any given bar relative to the entry z-score almost never reaches 50% reversion before the Z0 crossback fires.

## Files
- `run46_1_results.json` — Grid search results
- `run46.rs` — Implementation
