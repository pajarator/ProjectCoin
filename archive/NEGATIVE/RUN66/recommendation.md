# RUN66 — Exit Priority Reordering: Recommendation

## Hypothesis
Named: `exit_priority_reorder`

Reorder the evaluation of exit conditions (SL → Z0 → SMA → Timeout vs SL → SMA → Z0 → Timeout, etc.) to optimize which signal "wins" when multiple exit conditions fire simultaneously.

## Results

### RUN66.1 — Exit Priority Grid Search (4 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All exit priority configs produce identical PnL to baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Z0 | SMA | SL | TMO |
|--------|------|------|-----|--------|-----|-----|-----|-----|
| A_Z0_SMA (baseline) | +$256.20 | — | 49.0% | 13,545 | 674 | 163 | 5,010 | 7,698 |
| B_SMA_Z0 | +$256.20 | $0.00 | 49.0% | 13,545 | 674 | 163 | 5,010 | 7,698 |
| C_Z0_TMO | +$256.20 | $0.00 | 49.0% | 13,545 | 674 | 0 | 5,010 | 7,861 |
| D_SMA_TMO | +$256.20 | $0.00 | 49.0% | 13,545 | 0 | 163 | 5,010 | 8,372 |

**Key findings:**
- ALL 4 priority configs produce identical PnL — exit priority ordering has zero effect on financial outcomes
- The only difference is in exit REASON distribution (which bucket trades get counted in)
- When Z0 and SMA both fire in the same bar, both exit at the same price — priority determines which reason is recorded, not the PnL
- SMA exit fires very rarely (163 exits) vs Z0 (674 exits) — SMA crossover is not a common regime trade exit
- Exit priority ordering is irrelevant to COINCLAW's financial performance

**Why it fails:** In this backtest, all non-SL exits use the current bar's closing price as the exit price. When multiple exit conditions fire simultaneously, they all reference the same price, so the priority order only changes which exit reason is logged, not the actual PnL. The financial outcome is identical regardless of priority.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Exit priority reordering has no effect on PnL in simulation. The priority only affects exit reason tracking, not financial outcomes.

## Files
- `run66_1_results.json` — Grid search results
- `coinclaw/src/run66.rs` — Implementation