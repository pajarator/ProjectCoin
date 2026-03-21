# RUN103 — Stochastic Extreme Exit: Recommendation

## Hypothesis

**Named:** `stochastic_extreme_exit`

Exit regime trades when stochastic reaches extreme levels (OB ≥ 85 for longs, OS ≤ 25 for shorts) while the trade is profitable. The thesis: stochastic extremes signal short-term exhaustion, and exiting while profitable at these points locks in gains before potential pullback.

## Results

### RUN103.1 — Grid Search (28 configs × 18 coins, 5-month 15m data)

**MARGINAL POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades | StochExits | StochExitRate |
|--------|------|------|-----|--------|------------|---------------|
| OB85_OS25_MH3 (best) | +$362.31 | +$1.76 | 40.3% | 13,809 | 2,642 | 19.1% |
| OB80_OS25_MH3 | +$362.30 | +$1.75 | 40.3% | 13,809 | 2,746 | 19.9% |
| OB75_OS25_MH3 | +$362.26 | +$1.71 | 40.3% | 13,809 | 2,870 | 20.8% |
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0 | 0.0% |

**Key findings:**
- All three best configs use OS=25 (oversold threshold of 25, not 15 or 20) and MH=3 (minimum hold of 3)
- Stochastic exits are 100% win rate by design (only fires when pct > 0.0)
- WR improves from 39.0% → 40.3% (+1.3pp) as the stoch exit filters out some losing trades
- However, PnL delta is only +$1.76 — essentially noise
- Higher thresholds (MH=8) reduce stoch_exit_rate to 8-9% and produce negative deltas — holding longer defeats the purpose
- Volume/price can remain extreme for extended periods; 3-bar minimum hold is enough to avoid noise but not enough to miss the peak

## Conclusion

**MARGINAL POSITIVE — Not worth the complexity.** The +$1.76 delta across 18 coins over 5 months is ~$0.10/coin/month — effectively zero. The stochastic extreme exit mechanism works as designed (100% WR on exits) but the overall PnL improvement is negligible.

## Files
- `run103_1_results.json` — Grid search results
- `coinclaw/src/run103.rs` — Implementation
