# RUN133 — Ultimate Oscillator Confirmation: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| LM35_SM65 | +123.96 | -235.09 | 34.5% | 79.2% |
| LM30_SM65 | +85.71 | -273.34 | 34.5% | 86.9% |

**VERDICT: NEGATIVE** — Ultimate Oscillator configs collapse PnL with 79-98% filter rate and WR dropping to 30-35%.

## Analysis

**Filter mechanism:** UO combines 3 timeframes (7, 14, 28 periods) of buying pressure. LONG requires UO < 30 (deeply oversold), SHORT requires UO > 70 (deeply overbought).

**Why it fails:**
1. **Multi-timeframe smoothing removes signal:** Combining 3 EMAs creates a very lagging oscillator. By the time all 3 timeframes confirm oversold, the z-score extreme moment has passed.
2. **UO extremes are rare:** Requiring UO < 30 AND z-score < -2.0 simultaneously is extremely rare (79-98% filter).
3. **WR drops:** From 38.9% to 30-35%, meaning the filter removes better-than-average trades.

## Conclusion

No COINCLAW changes. Ultimate Oscillator is too lagging for z-score regime entries.
