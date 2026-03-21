# RUN97 — BB Width Scalp Gate: Recommendation

## Hypothesis

**Named:** `bb_width_scalp_gate`

Suppress scalp entries when Bollinger Band width is below average (low volatility):
- `bb_ratio = bb_width / bb_width_avg`
- When `bb_ratio < BB_SCALP_WIDTH_MIN`, suppress all scalp entries for that coin
- Rationale: scalp TP=0.8% requires oscillation; narrow BB = compressed oscillation range

## Results

### RUN97.1 — Grid Search (5 configs × 18 coins, 5-month 1m data)

**NEGATIVE — All configs produce identical results to baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Blocked |
|--------|------|------|-----|--------|---------|
| BASELINE | +$330.87 | — | 21.0% | 13,556 | 0 |
| BB0.50 through BB0.80 | +$330.87 | $0.00 | 21.0% | 13,556 | 0 |

**Key findings:**
- BB width ratio `bb_width / bb_width_avg` rarely falls below 0.50
- The gate never fires — 0 blocked entries across all thresholds
- The BB squeeze breakout signal (`bb_width < avg * 0.4` + vol spike) fires independently of whether the ratio is below a threshold
- The `bb_squeeze_break` signal itself is the volatility filter — adding another threshold on top is redundant

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run97_1_results.json` — Grid search results
- `coinclaw/src/run97.rs` — Implementation
