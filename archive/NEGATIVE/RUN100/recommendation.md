# RUN100 — Portfolio Correlation Risk Limit: Recommendation

## Hypothesis

**Named:** `corr_risk_limit`

Reduce portfolio capital deployment when cross-coin return correlation spikes:
- Compute rolling average pairwise correlation across all coins
- When `avg_correlation >= CORR_RISK_THRESHOLD`: reduce deployed capital by `CORR_DEPLOY_MULT`
- When `avg_correlation >= CORR_CRITICAL`: further reduce to 0.50×
- Rationale: diversification disappears in high-correlation regimes; cutting exposure preserves capital

## Results

### RUN100.1 — Grid Search (11 portfolio-level configs × 18 coins)

**NEGATIVE — All correlation risk configs produce lower PnL than baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE | +$208.40 | — | 40.4% | 8,933 |
| RT0.70_CT0.75_M0.70 (best) | +$142.61 | -$65.78 | 40.4% | 8,933 |
| RT0.50_CT0.65_M0.50 (worst) | +$79.01 | -$129.39 | 40.4% | 8,933 |

**Key findings:**
- All 10 correlation risk configs produce lower PnL (all ΔPnL < 0)
- WR unchanged (40.4% across all) — size reduction doesn't change win/loss mix
- Reducing position size during high-correlation periods cuts both winners and losers equally
- High-correlation periods often coincide with the best mean-reversion opportunities (market stress = best contrarian entries)
- With 40.4% WR and small wins relative to losses, cutting size during high-conviction periods loses disproportionate alpha

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run100_1_results.json` — Grid search results
- `coinclaw/src/run100.rs` — Implementation
