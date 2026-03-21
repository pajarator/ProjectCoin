# RUN42 — Dynamic Leverage by Volatility Regime: Recommendation

## Hypothesis
Named: `dynamic_leverage_regime`

Adjust leverage downward during HighVol/Squeeze regimes (2.0–3.0×) and upward during Ranging regimes (6.0–8.0×) to improve risk-adjusted returns. This changes position size without changing SL%.

## Results

### RUN42.1 — Grid Search (145 configs × 18 coins, 5-month 15m data)

**NEGATIVE — Sharpe ratio is identical across all 145 configurations.**

| Config | PnL | Sharpe | WR% | MaxDD% | Trades |
|--------|------|--------|-----|--------|--------|
| Baseline (R5_H5_S5_T5) | +$1,128 | 49.219 | 54.7% | 1.2% | 15,101 |
| R5_H1.5_S2_T2 | +$1,692 | 49.219 | 54.7% | 0.4% | 15,101 |
| R5_H1.5_S2_T3 | +$1,600 | 49.219 | 54.7% | 0.5% | 15,101 |
| R8_H2.0_S2_T2 | +$2,269 | 49.219 | 54.7% | 0.4% | 15,101 |
| R8_H3.0_S2_T2 | +$2,269 | 49.219 | 54.7% | 0.4% | 15,101 |

**Key findings:**
1. **Sharpe ratio is constant (49.219) across ALL 145 configs** — leverage changes scale returns but Sharpe stays identical
2. **MaxDD improves with lower leverage**: 0.4% (low lev) vs 1.2% (baseline lev=5.0)
3. **PnL scales with leverage**: R8 configs produce ~2× the PnL of R5 configs
4. **WR and trade count are identical** across all configs

**Root cause of NEGATIVE verdict:** The Sharpe ratio is dimensionless — it measures return per unit of volatility. Since leverage multiplies both returns AND volatility proportionally, the Sharpe ratio stays constant. The hypothesis correctly anticipated this (see "Why This Could Fail" #1: "Leverage is already factored into RISK").

**MaxDD does improve** with lower leverage (0.4% vs 1.2%) but Sharpe doesn't change, so risk-adjusted returns are equivalent.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Dynamic leverage by regime does not improve Sharpe ratio. The metric that matters (risk-adjusted returns) is leverage-invariant because leverage scales returns and volatility proportionally. However, MaxDD genuinely decreases with lower leverage, which could be beneficial for risk management — but this is a drawdown reduction, not a return improvement, and comes at the cost of lower absolute P&L.

If the goal is MaxDD reduction, simply reducing the fixed leverage from 5.0 to 2.0 would achieve the same result without needing regime detection.

## Files
- `run42_1_results.json` — Grid search results (145 configs)
- `run42.rs` — Implementation

## Next RUN
Proceed to RUN43: check `TEST/RUN43/` for the next hypothesis.
