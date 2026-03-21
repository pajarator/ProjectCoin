# RUN43 — Breadth Momentum Filter: Recommendation

## Hypothesis
Named: `breadth_momentum_filter`

Breadth velocity (rate of change of market breadth) contains predictive information about regime transitions. Rising breadth → bearish pressure building, falling breadth → bullish pressure building.

## Results

### RUN43.1 — Grid Search (17 configs, 5-month 15m data, 18 coins)

**NEGATIVE — all breadth momentum configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| Baseline (disabled) | +$167.45 | — | 54.7% | 15,101 |
| VW2_VT0.20 (best) | +$131.34 | -$36.11 | 56.0% | 10,862 |
| VW2_VT0.15 | +$127.25 | -$40.20 | 56.2% | 10,294 |
| VW2_VT0.10 | +$115.48 | -$51.97 | 55.5% | 9,509 |
| VW4_VT0.20 | +$112.02 | -$55.43 | 56.0% | 9,391 |
| VW8_VT0.05 (worst) | +$69.79 | -$97.66 | 57.5% | 5,854 |

**Key finding:** WR improves marginally (54.7% → up to 57.5% with VW8_VT0.05), but trade count drops by 42-61%, resulting in net PnL decrease. The breadth velocity filter doesn't provide enough quality improvement to compensate for the reduced quantity.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Breadth momentum (velocity) is not a useful predictor for regime transitions in backtesting. The static breadth threshold system (LONG ≤20%, ISO_SHORT 20-50%, SHORT ≥50%) remains optimal.

## Files
- `run43_1_results.json` — Grid search results
- `run43.rs` — Implementation

## Next RUN
Proceed to RUN44.
