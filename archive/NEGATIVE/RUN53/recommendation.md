# RUN53 — Partial Exit / Scale-Out: Recommendation

## Hypothesis
Named: `partial_scale_out`

Exit regime trades progressively in tiers — secure 33-50% of position at tier profit thresholds, hold remainder until full signal exit.

## Results

### RUN53.1 — Grid Search (33 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All partial exit configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| T1P0.00/T2P0.01/F0.33 (best) | +$170.28 | -$6.69 | 54.7% | 15,101 |
| T1P0.01/T2P0.01/F0.50 (worst) | +$137.72 | -$39.25 | 54.7% | 15,101 |

**Key findings:**
- ALL 32 partial exit configs produce lower PnL than baseline
- WR is identical (54.7%) across all configs — partial exits change monetary outcomes, not trade outcomes
- EXIT_FRAC=0.33 (smaller exits) hurts less than EXIT_FRAC=0.50 (larger exits)
- TIER1_PCT and TIER2_PCT differences are minimal within each EXIT_FRAC group
- Exit fraction is the dominant parameter — larger fractions = more PnL lost
- PnL loss from partial exits exceeds any gain from securing early profits

**Why it fails:** With WR=54.7% and 5× leverage, the base position sizing is already well-calibrated. Exiting early (even partially) reduces the average winning trade's profit more than it reduces average loss exposure. The win/loss asymmetry at this leverage means holding full position to exit signal is optimal.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Partial scale-out reduces PnL because WR=54.7% is near breakeven for 5× leverage. Early exits reduce average win more than they reduce average loss. The baseline full-position exit is optimal.

## Files
- `run53_1_results.json` — Grid search results
- `coinclaw/src/run53.rs` — Implementation
