# RUN44 — Multi-Timeframe ISO Short Confirmation: Recommendation

## Hypothesis
Named: `multi_timeframe_iso_confirmation`

ISO short entries require 1h and 4h RSI overbought confirmation in addition to 15m RSI.

## Results

### RUN44.1 — Grid Search (10 configs × 18 coins, 5-month 15m data)

**NEGATIVE — all MTF configs reduce PnL significantly vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| Baseline (disabled) | +$178.20 | — | 51.0% | 20,383 |
| H60_F65 | +$104.76 | -$73.44 | 49.3% | 12,807 |
| H65_F65 | +$102.15 | -$76.04 | 49.2% | 12,512 |
| H70_F70 | +$98.30 | -$79.90 | 49.4% | 11,819 |
| H70_F75 | +$97.09 | -$81.10 | 49.7% | 11,504 |

**Key finding:** Multi-timeframe RSI confirmation significantly reduces trade count (by 39-44%) and WR drops slightly (49-50% vs 51%). The trade quality improvement is insufficient to compensate for fewer opportunities.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run44_1_results.json` — Grid search results
- `run44.rs` — Implementation
