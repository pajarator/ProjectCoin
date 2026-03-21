# RUN75 — Sharpe-Weighted Capital Allocation: Recommendation

## Hypothesis
Named: `sharpe_weighted_allocation`

Mechanically: Rebalance per-coin capital based on trailing Sharpe ratio. More capital to higher-Sharpe coins, less to lower-Sharpe coins.

## Results

### RUN75.1 — Sharpe Allocation Grid Search (55 configs × portfolio, 5-month 15m data)

**POSITIVE — Many configs beat baseline significantly.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE | +$337.46 | — | 25.4% | 10,919 |
| F336_W10_MIN0.50_MAX3.0 (best) | +$1,819.33 | +$1,481.87 | 25.4% | 10,919 |
| F84_W10_MIN0.50_MAX3.0 | +$1,744.24 | +$1,406.77 | 25.4% | 10,919 |
| F84_W10_MIN0.25_MAX3.0 | +$1,606.83 | +$1,269.37 | 25.4% | 10,919 |
| F168_W10_MIN0.50_MAX3.0 | +$1,632.93 | +$1,295.47 | 25.4% | 10,919 |
| F336_W20_MIN0.50_MAX3.0 | +$1,800.19 | +$1,462.72 | 25.4% | 10,919 |

**Key findings:**
- All configs with MAX=3.0 strongly positive (allow 3x capital concentration to high-Sharpe coins)
- All configs with MAX=1.5 negative (too restrictive)
- FREQ and WINDOW parameters have moderate effect
- WR unchanged at 25.4% — rebalancing doesn't affect trade quality, only capital allocation

**Mechanism:** MAX3.0 allows a high-Sharpe coin to grow from $100 to $300 (3x concentration). Capital compounds toward top performers while bottom performers stay at minimum ($50). The amplification of high-Sharpe coin returns dominates.

### RUN75.2 — Walk-Forward Validation (3 windows, best config: F336_W10_MIN0.50_MAX3.0)

**POSITIVE — 3/3 windows positive OOS.**

| Window | TrainPnL | TestPnL | ΔvsBaseline | WR% | Trades |
|--------|----------|---------|-------------|-----|--------|
| 1 | +$1,814.88 | +$781.65 | +$734.66 | 28.7% | 2,122 |
| 2 | +$1,431.23 | +$703.68 | +$634.20 | 25.5% | 2,165 |
| 3 | +$1,983.68 | +$1,249.09 | +$1,188.70 | 26.3% | 2,185 |

Avg Δ: **+$852.52** across 3 windows.

## Conclusion

**POSITIVE — Walk-forward confirms. Recommend adding to COINCLAW.**

Best config: `F336_W10_MIN0.50_MAX3.0` (rebalance every 336 bars ~2 weeks, 10-trade Sharpe window, 50%-300% cap)

**Caveat:** The Sharpe computation uses annualized approximation that inflates Sharpe values. The 3x max cap (allowing $100 → $300 concentration) is the primary driver. Verify that MAX=3.0 is not too aggressive for live trading risk tolerance.

## Files
- `run75_1_results.json` — Grid search results
- `run75_2_results.json` — Walk-forward results
- `coinclaw/src/run75.rs` — Grid search implementation
- `coinclaw/src/run75_2.rs` — Walk-forward implementation
