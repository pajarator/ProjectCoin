# RUN41 — Session-Based Trade Filter: Recommendation

## Hypothesis
Named: `session_conditional_trading`

Mean-reversion strategies should work better during Asia session (low volume, ranging) and worse during US session (momentum-driven). Session-based filtering should raise win rate.

## Results

### RUN41.1 — Session Profiling + Grid Search (5 configs × 18 coins, 5-month 15m data)

**NEGATIVE — all session filters reduce P&L.**

#### Session Profile (Baseline — no filter):

| Session | Hours (UTC) | Avg Trades | Avg WR% | Avg PnL |
|---------|-------------|-----------|---------|---------|
| Asia | 0–8 | ~213 | 31–38% | ~$1.0/coin |
| Europe | 9–16 | ~267 | 24–29% | ~$0.8/coin |
| US | 17–23 | ~109 | 32–45% | ~$0.7/coin |

**Finding:** US session actually has the highest WR% (32–45%) despite the hypothesis saying otherwise. Asia has lowest trade volume but moderate WR. Europe has most trades but lowest WR.

#### Grid Search Results:

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| Baseline (disabled) | +$53.53 | — | 30.1% | 10,748 |
| ASIA_EUR | +$43.91 | -$9.63 | 29.8% | 8,767 |
| EUROPE_ONLY | +$23.42 | -$30.11 | 27.4% | 4,857 |
| ASIA_ONLY | +$22.37 | -$31.16 | 33.1% | 4,325 |
| US_ONLY | +$17.47 | -$36.07 | 33.3% | 2,765 |

**Key finding:** Every session filter reduces P&L. US_ONLY has highest WR (33.3%) but lowest trade count (2,765), resulting in worst P&L (-$36). ASIA_ONLY also has high WR but loses -$31 PnL. The baseline's combination of all sessions maximizes total P&L despite lower WR in Europe.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Session filtering does not improve COINCLAW's regime strategy. The hypothesis that Asia session favors mean-reversion is not supported by data — US session actually shows higher win rates. However, reducing trades (even to higher-quality ones) destroys total P&L because COINCLAW's edge comes from frequent small edges across many trades, not from concentrated high-WR bets.

The fundamental issue: COINCLAW's win rate (~30%) is well below 44% breakeven, meaning it relies on the profit factor (winners > losers) to be profitable. Session filtering that reduces trade count reduces both wins and losses proportionally, but the ratio stays similar while the absolute number of opportunities shrinks.

## Files
- `run41_1_results.json` — Grid search results
- `run41.rs` — Implementation

## Next RUN
Proceed to RUN42: check `TEST/RUN42/` for the next hypothesis.
