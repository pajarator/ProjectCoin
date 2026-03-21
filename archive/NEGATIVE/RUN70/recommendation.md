# RUN70 — Z-Score Convergence Filter: Recommendation

## Hypothesis
Named: `z_convergence_filter`

Require that market-wide convergence (% of coins with rising z-scores) meets threshold before allowing entry. Market-wide synchronization indicates systemic mean-reversion regime.

## Results

### RUN70.1 — Z-Convergence Grid Search (9 configs, portfolio-level backtest)

**NEGATIVE — All convergence configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| BASELINE (no filter) | +$516.36 | — | 53.9% | 17,097 | 0% |
| T+40_W2 (best) | +$378.21 | -$138.15 | 54.4% | 13,357 | 34% |
| T+40_W1 | +$249.93 | -$266.43 | 54.1% | 10,917 | 51% |
| T+60_W2 | +$101.99 | -$414.37 | 55.4% | 6,296 | 75% |
| T+70_W2 | +$101.99 | -$414.37 | 55.4% | 6,296 | 75% |

**Key findings:**
- ALL convergence configs produce lower PnL than baseline
- WR improves only +0.2-1.5pp while trade count drops 34-75%
- The block rate is very high (34-75%) because coins are rarely in convergence simultaneously
- Trade count reduction dominates any marginal WR improvement

**Why it fails:** Requiring market-wide convergence (multiple coins' z-scores rising simultaneously) is too restrictive. The filter blocks 34-75% of entries, and the WR improvement is insufficient to compensate.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run70_1_results.json` — Grid search results
- `coinclaw/src/run70.rs` — Implementation