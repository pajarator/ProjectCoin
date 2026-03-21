# RUN50 — Candle Composition Filter: Recommendation

## Hypothesis
Named: `candle_composition_filter`

Filter regime entries using candle composition: body ratio, shadow ratios, and volume confirmation to reject low-conviction entries.

## Results

### RUN50.1 — Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All candle composition filter configs reduce PnL vs baseline despite WR improvement.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| B0.3_S0.10_V1.0 (best) | +$160.41 | -$16.56 | 55.4% | 12,483 |
| B0.4_S0.10_V1.0 | +$154.85 | -$22.12 | 55.6% | 11,974 |
| B0.5_S0.10_V1.2 | +$136.13 | -$40.84 | 56.0% | 10,148 |
| B0.5_S0.10_V1.5 (worst) | +$114.50 | -$62.47 | 55.9% | 8,523 |

**Key findings:**
- All 27 filter configs produce lower PnL than baseline despite WR improvement of +0.7-1.3pp
- The WR improvement is insufficient to compensate for the trade count reduction
- Body ratio and shadow ratio parameters have minimal impact on outcomes (B and S values don't differentiate results much)
- Volume filtering (V1.5) causes the largest trade reduction and worst PnL
- The candle composition filter removes more winning trades than losing ones proportionally

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The candle composition filter improves WR slightly but at the cost of too many trades. The PnL loss from trade reduction outweighs the WR improvement. Regime trade entries have sufficient quality without additional candle composition filtering.

## Files
- `run50_1_results.json` — Grid search results
- `coinclaw/src/run50.rs` — Implementation
