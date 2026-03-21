# RUN89 — Market-Wide ADX Confirmation: Recommendation

## Hypothesis

**Named:** `market_adx_confirm`

When market-wide average ADX is high, suppress regime entries:
- avg_adx >= SUPPRESS_ALL: block ALL regime entries
- avg_adx >= SUPPRESS_LONG: block LONG entries (ISO_SHORT allowed)

## Results

### RUN89.1 — Grid Search (16 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL configs worse than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | ADXsupp% |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.0% |
| SL20_SA26 (best non-baseline) | +$287.06 | -$5.79 | 26.0% | 9,511 | 6.3% |

**Key findings:**
- ALL 16 configs produce identical results (same PnL=-$5.79, same 6.3% suppression rate)
- Different thresholds (20-28 for SL, 26-35 for SA) all block the exact same entries
- The avg ADX is consistently above 20 throughout the dataset, so all thresholds capture the same regime
- Suppressing entries during high-ADX periods loses $5.79 vs letting them run
- 205 fewer trades (9,511 vs 9,716) — suppression is meaningful but doesn't improve quality

**Why it fails:** Market-wide ADX suppression cuts both good and bad trades equally. The entries blocked during high-ADX periods include some winners. The net effect is negative because the ADX threshold doesn't discriminate between trades that would have won vs lost.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run89_1_results.json` — Grid search results
- `coinclaw/src/run89.rs` — Implementation
