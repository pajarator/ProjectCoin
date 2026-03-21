# RUN118 — Hull Suite Trend Filter: Recommendation

## Hypothesis

**Named:** `hull_suite_filter`

Use Hull Moving Average (HMA) as low-lag trend filter for entries and HMA crossover as adaptive trailing exit.

## Results

### RUN118.1 — Grid Search (12 configs × 18 coins, 5-month 15m data)

**MARGINAL POSITIVE (effectively baseline-equivalent)**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate | HullExits | HE_WR% |
|--------|------|------|-----|--------|--------|--------|--------|
| LB20_NS_NH (best) | +$360.77 | +$0.22 | 39.0% | 13,687 | 0.0% | 0 | — |
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% | 0 | — |
| LB10_NS_NH | +$358.09 | -$2.46 | 39.0% | 13,599 | 0.9% | 0 | — |
| LB20_NS_HE | +$336.98 | -$23.57 | 47.3% | 14,291 | 0.0% | 5,121 | 93.7% |

**Key findings:**
- Best config (LB20_NS_NH) = +$0.22 above baseline — essentially baseline-equivalent
- HMA entry filter blocks only 0-35% of entries across configs
- HMA crossover exits have 87-96% WR but reduce overall PnL — they exit too early on winning trades
- HMA as entry filter is effectively neutral; HMA as exit reduces PnL despite high per-exit WR

## Conclusion

**MARGINAL POSITIVE (effectively baseline-equivalent).** HMA filter provides no meaningful improvement over baseline. LB20_NS_NH (+$0.22) is technically positive but negligible. HMA crossover exits have excellent per-exit WR (87-96%) but overall reduce PnL by replacing longer winning trades with shorter ones. No COINCLAW changes warranted.

## Files
- `run118_1_results.json` — Grid search results
- `coinclaw/src/run118.rs` — Implementation
