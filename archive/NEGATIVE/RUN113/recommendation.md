# RUN113 — CCI Regime Confirmation: Recommendation

## Hypothesis

**Named:** `cci_regime_confirm`

Require CCI extreme at regime entry: CCI < -100 for LONG, CCI > +100 for SHORT, adding MAD-based oscillator confirmation.

## Results

### RUN113.1 — Grid Search (9 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| CL-50_CS50 | +$360.31 | -$0.25 | 39.0% | 13,686 | 0.0% |
| CL-50_CS100 | +$359.68 | -$0.88 | 39.0% | 13,660 | 0.4% |
| CL-100_CS50 | +$359.58 | -$0.98 | 39.0% | 13,673 | 0.2% |
| CL-150_CS150 | +$346.68 | -$13.87 | 38.9% | 12,745 | 12.5% |

**Key findings:**
- ALL configs produce equal or lower PnL than baseline
- Even strictest thresholds (CL-150_CS150) only block 12.5% of entries
- CCI is redundant with z-score: both measure deviation from mean using different math (MAD vs std dev), but both converge on the same "extreme" signal
- The 0.5-12.5% filter rate shows CCI rarely disagrees with z-score

## Conclusion

**NEGATIVE.** CCI confirmation adds no value — it is redundant with z-score. The two oscillators measure the same concept (price deviation) using slightly different math, and they almost always agree. When CCI does disagree, blocking those entries hurts P&L.

## Files
- `run113_1_results.json` — Grid search results
- `coinclaw/src/run113.rs` — Implementation
