# RUN73 — Dynamic Max Hold Based on Entry Z-Score: Recommendation

## Hypothesis
Named: `dynamic_max_hold`

Mechanically: `max_hold = BASE + (|z_at_entry| - Z_BASE) × FACTOR`, capped at CAP.

## Results

### RUN73.1 — Dynamic Max Hold Grid Search (55 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 55 configs produce IDENTICAL PnL to baseline.**

| Config | PnL | ΔPnL | WR% | Trades | AvgHeld |
|--------|------|------|-----|--------|---------|
| BASELINE (fixed 240) | +$293.26 | — | 25.9% | 9,690 | 10.6 |
| B150_ZB15_F30_C280 (any config) | +$293.26 | $0.00 | 25.9% | 9,690 | 10.6 |

All 54 dynamic configs: identical to baseline. No variation whatsoever.

**Key findings:**
- avg_held_bars = 10.6 — positions close via Z-score crossback or SL **before** MAX_HOLD ever triggers
- MAX_HOLD=240 (or 280/320/360) is never reached in practice
- The dynamic formula produces values 150–360, but actual holds never exceed ~50 bars
- Changing a safety net that never activates has zero effect

**Why it fails:** Z-score crossback and stop-loss exits resolve positions in ~10 bars on average. The MAX_HOLD safety net (whether fixed or dynamic) is too loose to matter for this strategy. Positions that survive past the signal exit would be winners if held longer — but the signal exit is triggered by the mean-reversion completing, not by time.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run73_1_results.json` — Grid search results
- `coinclaw/src/run73.rs` — Implementation
