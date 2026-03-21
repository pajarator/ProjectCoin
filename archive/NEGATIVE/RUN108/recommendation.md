# RUN108 — Momentum Hour Filter: Recommendation

## Hypothesis

**Named:** `momentum_hour_filter`

Use 1m momentum (ROC) to filter 15m regime entries: suppress LONG when 1m ROC < -0.3% (sharp short-term downtrend) and SHORT when 1m ROC > +0.3% (sharp short-term uptrend).

## Results

### RUN108.1 — Grid Search (28 configs × 18 coins, 5-month 15m+1m data)

**STRONGLY NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| LB3_LM0.003_SM0.003 (best) | +$346.46 | -$14.09 | 39.1% | 13,401 | 4.4% |
| LB6_LM0.003_SM0.003 | +$345.67 | -$14.89 | 39.5% | 13,072 | 9.7% |

**Key findings:**
- ALL 27 configs produce negative PnL delta vs baseline
- Even the most permissive filter (LB3_LM0.003_SM0.003, blocks only 4.4% of entries) reduces PnL by $14
- The "headwind" hypothesis is wrong: regime trades work even when 1m momentum opposes them
- Filtering out "headwind" entries removes a mix of winners and losers — the net effect is negative
- The 1m momentum is noisy and not predictive of regime trade success

## Conclusion

**STRONGLY NEGATIVE.** The 1m momentum filter is fundamentally counterproductive. Regime trades succeed based on 15m mean-reversion signals, not short-term 1m momentum. Fighting the 1m momentum doesn't improve entry quality.

## Files
- `run108_1_results.json` — Grid search results
- `coinclaw/src/run108.rs` — Implementation
