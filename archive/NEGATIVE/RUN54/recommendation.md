# RUN54 — Volatility Regime Entry Filter: Recommendation

## Hypothesis
Named: `volatility_regime_filter`

Trade mean-reversion only when volatility is below median (ATR rank below threshold) to improve signal quality in low-noise environments.

## Results

### RUN54.1 — Grid Search (25 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All volatility filter configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| VW15_VT0.80_M1 (best) | +$82.66 | -$94.31 | 55.2% | 8,266 | 45% |
| VW30_VT0.50_M1 (worst) | +$58.24 | -$118.73 | 55.5% | 5,971 | 60% |

**Key findings:**
- ALL 24 volatility filter configs produce lower PnL than baseline
- WR improves slightly (+0.5-0.8pp) but trade count reduction dominates
- Block rates of 40-60% eliminate too many trades; remaining trades can't compensate
- Relaxed mode (M2, threshold-independent) blocks ~48% of trades identically
- Stricter thresholds block more trades but the remaining trades don't win enough to compensate

**Why it fails:** High-vol regimes don't systematically produce worse win rates for mean-reversion. The filter blocks ~50% of trades but the WR improvement (+0.5-0.8pp) is insufficient to offset the trade count reduction. COINCLAW's z-score already acts as a volatility-adaptive signal.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Trade count reduction dominates the small WR improvement. ATR rank filtering is not a useful signal quality filter for COINCLAW's regime strategy.

## Files
- `run54_1_results.json` — Grid search results
- `coinclaw/src/run54.rs` — Implementation
