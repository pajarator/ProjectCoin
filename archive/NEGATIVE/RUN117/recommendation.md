# RUN117 — ROC Percentile Filter: Recommendation

## Hypothesis

**Named:** `roc_percentile_filter`

Require ROC to be at extreme percentile of its own historical distribution before allowing regime entries.

## Results

### RUN117.1 — Grid Search (27 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| LP20_SP20_LB3 (best) | +$286.30 | -$74.25 | 36.8% | 10,482 | 39.1% |
| LP20_SP20_LB5 | +$285.01 | -$75.54 | 36.8% | 10,112 | 43.7% |

**Key findings:**
- ALL 27 configs produce lower PnL than baseline
- WR drops from 39% to 34-37% across all configs
- Filter rates 39-85% — ROC percentile blocks a significant portion of entries
- Blocking entries reduces total opportunity more than it improves quality

## Conclusion

**NEGATIVE.** ROC percentile normalization doesn't improve mean-reversion entries. While it blocks 39-85% of entries, WR only marginally improves and PnL always declines. ROC extremes and z-score extremes are not complementary enough to justify the trade count reduction.

## Files
- `run117_1_results.json` — Grid search results
- `coinclaw/src/run117.rs` — Implementation
