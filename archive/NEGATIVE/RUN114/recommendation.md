# RUN114 — Aroon Regime Confirmation: Recommendation

## Hypothesis

**Named:** `aroon_regime_confirm`

Require Aroon to confirm range-bound state (both Aroon Up and Down below threshold, Aroon oscillator confirming direction) before regime entries.

## Results

### RUN114.1 — Grid Search (27 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| LB14_OM10_TM80 (best) | +$20.34 | -$340.21 | 48.3% | 791 | 97.3% |
| LB14_OM20_TM80 | +$16.61 | -$343.94 | 48.1% | 626 | 97.9% |

**Key findings:**
- ALL Aroon configs block 97-99% of entries — catastrophically restrictive
- WR improves dramatically (39% → 48-52%) due to extreme filtering
- PnL collapses from $360 to $5-20 despite WR improvement
- The "range-bound" requirement is too strict: Aroon almost always shows trending state in crypto

## Conclusion

**STRONGLY NEGATIVE.** Aroon regime confirmation is far too restrictive for crypto markets. The hypothesis assumed crypto markets spend significant time in range-bound states detectable by Aroon, but 97-99% block rate shows the indicator consistently registers trending conditions. Mean-reversion requires entries to fire with some frequency — WR improvements at 2% trade retention don't compensate.

## Files
- `run114_1_results.json` — Grid search results
- `coinclaw/src/run114.rs` — Implementation
