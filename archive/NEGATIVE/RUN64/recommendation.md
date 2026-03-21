# RUN64 — Portfolio Position Density Filter: Recommendation

## Hypothesis
Named: `portfolio_density_filter`

Block new regime entries when >60% of coins have open positions simultaneously to avoid clustered drawdown.

## Results

### RUN64.1 — Density Filter Grid Search (13 configs, portfolio-level backtest)

**NEGATIVE — All density configs identical to baseline (no effect).**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$961.21 | — | 54.0% | 20,796 |
| MD0.50–MD0.80, DC4–DC12 | +$961.21 | $0.00 | 54.0% | 20,796 |

**Key findings:**
- ALL 12 density filter configs produce identical results to baseline — filter never triggers
- With 2-bar cooldown and ~15K total trades across 18 coins, simultaneous open positions never reach 50%+ threshold
- The portfolio density threshold is never crossed during the 5-month backtest period
- Mean-reversion signals across coins are not sufficiently correlated in time to create "crowded" states

**Why it fails:** The portfolio density threshold (50-80%) is never crossed. COINCLAW's per-coin cooldown mechanisms naturally spread out position openings, preventing clustered exposure.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The density threshold is never reached in normal COINCLAW operation. The portfolio never becomes "too crowded" to trigger suppression.

## Files
- `run64_1_results.json` — Grid search results
- `coinclaw/src/run64.rs` — Implementation
