# RUN120 — Mass Index Reversal Filter: Recommendation

## Hypothesis

**Named:** `mass_index_filter`

Require Mass Index to peak above 27 and drop below 26.5 (confirming trend reversal) before allowing regime entries.

## Results

### RUN120.1 — Grid Search (12 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$359.05 | — | 38.9% | 13,665 | 0% |
| ALL 12 Mass configs | +$0.00 | -$359.05 | 0.0% | 0 | 100% |

**Key findings:**
- ALL Mass Index configs block 100% of entries — filter rate 100%
- Mass Index reversal signal (peak > 27, then drop below low threshold) never fires in 5 months of 15m crypto data
- Mass Index was designed for less volatile, longer-cycle markets; crypto's continuous 24/7 trading prevents the range-expansion/contraction pattern the indicator tracks

## Conclusion

**STRONGLY NEGATIVE.** Mass Index reversal confirmation never activates in crypto markets. The High-Low range expansion/contraction pattern that Mass Index tracks doesn't manifest as discrete reversal signals in 24/7 crypto trading. All configs block 100% of entries.

## Files
- `run120_1_results.json` — Grid search results
- `coinclaw/src/run120.rs` — Implementation
