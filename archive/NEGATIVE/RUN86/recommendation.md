# RUN86 — Coin Correlation Clustering: Recommendation

## Hypothesis
Named: `momentum_pulse_filter`

When 3+ coins in a correlation cluster signal simultaneously, only allow top Sharpe to trade. Suppresses lower-quality signals to reduce correlated drawdown concentration.

## Results

### RUN86.1 — Grid Search (37 configs × 18 coins, 5-month 15m data)

**MARGINALLY POSITIVE — Most configs negative.**

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 |
| T0.70_MS3_MC1_CD4 (best) | +$295.72 | +$2.87 | 25.9% | 9,683 | 0.35 |
| T0.80_MS4_MC1_CD8 | +$293.84 | +$0.99 | 25.9% | 9,698 | 0.35 |

**29/36 configs are negative** — the correlation clustering mechanism only helps in narrow conditions.

### RUN86.2 — Walk-Forward Validation (3 windows, 2mo train / 1mo test)

**NEGATIVE — 1/3 windows pass, avg test Δ = -$0.83**

| Window | Train Δ | Test Δ | Pass? |
|--------|---------|--------|-------|
| Win 1 (0-5760/5760-8640) | +$0.66 | +$0.94 | PASS |
| Win 2 (2880-8640/8640-11520) | +$2.07 | -$1.91 | FAIL |
| Win 3 (5760-11520/11520-14400) | -$1.00 | -$1.52 | FAIL |

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The correlation clustering mechanism is fragile:
- The in-sample improvement (+$2.87) is noise-level and does not hold OOS
- 29/36 grid configs are negative, confirming the mechanism is overfitted to in-sample correlations
- The best config suppresses only 33 entries out of 9,716 — marginal impact
- Walk-forward shows degradation in 2/3 windows

## Files
- `run86_1_results.json` — Grid search results
- `run86_2_results.json` — Walk-forward results
- `coinclaw/src/run86.rs` — Grid search implementation
- `coinclaw/src/run86_2.rs` — Walk-forward implementation
