# RUN35 — Scalp Exit Strategy Grid Search: Recommendation

## Hypothesis
Named: `scalp_exit_optimization`

Grid search over 16 scalp exit strategies to find the best performer on 1-year 1m data with zero-fee assumption.

## Results

### RUN35.1 — Grid Search (18 coins, 1-year 1m data)

**POSITIVE — stoch_50 exit improves PnL by +$12.54 vs baseline.**

| Config | PnL | ΔPnL | Win Rate | Trades |
|--------|------|------|---------|--------|
| baseline | +$678.71 | — | — | — |
| stoch_50 (best) | +$691.25 | +$12.54 | — | — |

**Key findings:**
- Stochastic-based exit (stoch_50) outperforms baseline by +1.8%
- 16 exit configs tested across 18 coins
- Grid search on 1-year 1m data

## Conclusion

**POSITIVE — Applied to COINCLAW.**

The stochastic-based exit improves scalp performance. Recommend adopting `stoch_50` exit for scalp trades.

## Files
- `run35_1_results.json` — Grid search results
- `coinclaw/src/run35.rs` — Implementation
