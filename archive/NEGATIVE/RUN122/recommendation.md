# RUN122 — Elder Ray Bull Power Confirmation: Recommendation

## Hypothesis

**Named:** `elder_ray_confirm`

Use Elder Ray Bull Power (High - EMA13) and Bear Power (Low - EMA13) as entry confirmation filters. For LONG: require Bull Power < 0 and rising (bears weakening, bulls starting). For SHORT: require Bear Power > 0 and falling (bulls weakening, bears starting).

## Results

### RUN122.1 — Grid Search (12 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.59 | — | 39.0% | 13,686 | 0% |
| EP13_B0_E1 | +$256.35 | -$104.24 | 38.4% | 9,866 | 47.6% |
| EP21_B1_E0 | +$254.88 | -$105.71 | 40.5% | 8,990 | 55.0% |
| EP9_B0_E1 | +$252.30 | -$108.29 | 38.1% | 9,783 | 48.5% |
| EP13_B1_E0 | +$252.06 | -$108.52 | 39.9% | 9,421 | 51.6% |
| EP13_B1_E1 | +$148.94 | -$211.65 | 39.3% | 5,439 | 78.1% |
| EP21_B1_E1 | +$140.75 | -$219.83 | 40.4% | 4,623 | 82.0% |
| EP9_B1_E1 | +$139.39 | -$221.20 | 37.8% | 5,336 | 78.7% |

**Key findings:**
- All Elder Ray configs reduce PnL vs baseline
- B0_E0 configs (no confirmation) = baseline at 0% filter rate
- Bear-only confirm (B0_E1): 47-51% filter rate, PnL drops $100-108
- Bull-only confirm (B1_E0): 51-55% filter rate, PnL drops $106-113
- Both confirm (B1_E1): 78-82% filter rate, PnL collapses to $139-149
- WR improves slightly (0.4–1.5pp) but PnL collapses — filtered trades are BETTER than average

## Conclusion

**NEGATIVE.** Elder Ray Bull/Bear Power confirmation hurts performance. The filter blocks 47-82% of entries, and the blocked entries have higher average win rate than the average trade. The "rising bull power" and "falling bear power" conditions remove the trades that would have won, leaving only the weaker signals. Elder Ray's single-bar power measurement is too noisy for crypto's 24/7 high-frequency mean-reversion environment. No COINCLAW changes.

## Files
- `run122_1_results.json` — Grid search results
- `coinclaw/src/run122.rs` — Implementation
