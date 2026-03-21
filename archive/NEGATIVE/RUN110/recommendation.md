# RUN110 — BB Width Compression Entry: Recommendation

## Hypothesis

**Named:** `bb_width_compression`

BB width compresses before explosive moves. After N bars of compressed BB (bb_width/bb_width_avg < threshold), relax the z-score entry threshold to capture imminent breakouts.

## Results

### RUN110.1 — Grid Search (27 configs × 18 coins, 5-month 15m data)

**NEGATIVE** — All configs produce identical PnL to baseline.

| Config | PnL | ΔPnL | WR% | Trades | CompEntries |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0 |
| BT0.60_BB3_ZR0.10 | +$360.55 | $0.00 | 39.0% | 13,689 | 0 |
| BT0.80_BB5_ZR0.30 | +$360.55 | $0.00 | 39.0% | 13,689 | 0 |

**Key findings:**
- ALL 27 configs produce zero compression entries — bb_comp_count never reaches bb_bars threshold
- bb_width/bb_width_avg never drops below any tested threshold (0.60, 0.70, 0.80) for 3+ consecutive bars
- BB compression is not a predictive signal for mean-reversion entries on 15m crypto charts
- The compression-relaxation mechanism never activates

## Conclusion

**NEGATIVE.** BB width compression does not predict regime trade success. The compression condition never met any of the tested thresholds across 5 months of 15m data. BB-based entry timing is not viable for this timeframe/regime system.

## Files
- `run110_1_results.json` — Grid search results
- `coinclaw/src/run110.rs` — Implementation
