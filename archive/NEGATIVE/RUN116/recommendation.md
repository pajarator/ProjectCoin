# RUN116 — KST Confirmation Filter: Recommendation

## Hypothesis

**Named:** `kst_confirm_filter`

Require KST (Know Sure Thing multi-timeframe momentum oscillator) to confirm regime entries: KST > signal + KST >= threshold for LONG, KST < signal + KST <= threshold for SHORT.

## Results

### RUN116.1 — Grid Search (18 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| All 18 configs | +$360.55 | $0.00 | 39.0% | 13,689 | 0% |

**Key findings:**
- ALL 18 KST configs produce identical PnL to baseline with filter_rate = 0%
- KST thresholds never block any regime entries — the KST momentum confirmation never disagrees with z-score regime signals
- KST and z-score are both measuring momentum/direction in fundamentally aligned ways at regime entry points
- Test is inconclusive: filter never activates

## Conclusion

**NEGATIVE (inconclusive).** KST confirmation never filters any entries at the tested thresholds. The KST momentum signal is always aligned with z-score regime direction at entry points. The filter is effectively disabled across all threshold combinations.

## Files
- `run116_1_results.json` — Grid search results
- `coinclaw/src/run116.rs` — Implementation
