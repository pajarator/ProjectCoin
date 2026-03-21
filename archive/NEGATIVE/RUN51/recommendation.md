# RUN51 — Drawdown-Contingent Stop Loss Widening: Recommendation

## Hypothesis
Named: `drawdown_contingent_sl`

When a coin's cumulative drawdown exceeds thresholds (3%, 5%, 8%), widen the SL to give trades more room during potential regime changes.

## Results

### RUN51.1 — Grid Search (13 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All 13 configs produce identically the same results as baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| T3_W1.5_R | +$176.97 | +$0.00 | 54.7% | 15,101 |
| T5_W2.0_C | +$176.97 | +$0.00 | 54.7% | 15,101 |
| T8_W2.0_R | +$176.97 | +$0.00 | 54.7% | 15,101 |

**Key finding:** All configs are identical to baseline because the drawdown threshold is **never reached** during the 5-month backtest period. The cumulative drawdown never exceeds 3% for any coin, so the dynamic SL widening is never triggered.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The dynamic SL widening mechanism never activates because the strategy's drawdown never reaches the trigger thresholds (3%, 5%, or 8%). This could mean: (a) the strategy's 0.3% SL is well-calibrated for the historical period, or (b) a longer backtest period or different market conditions would be needed to trigger this mechanism.

## Files
- `run51_1_results.json` — Grid search results
- `coinclaw/src/run51.rs` — Implementation
