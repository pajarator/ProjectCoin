# RUN128 — Schaff Trend Cycle Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| LT30_ST70_N | +221.22 | -137.83 | 37.7% | 50.8% |
| LT30_ST75_N | +216.16 | -142.89 | 37.7% | 54.0% |

**VERDICT: NEGATIVE** — Best non-baseline config loses -$138 vs baseline with 50.8% filter rate. TURN=true configs (require rising/falling STC) have 100% filter rate — the STC turn condition is never met at z-score extremes.

## Analysis

**Filter mechanism:** Schaff Trend Cycle = MACD(12,26) fed through stochastic cycle + EMA smoothing. LONG requires STC < long_thresh (oversold), SHORT requires STC > short_thresh (overbought). Optional TURN requirement: STC must be rising (LONG) or falling (SHORT).

**Why it fails:**
1. **TURN=true configs: 100% filter rate:** At the moment z-score reaches extreme (±2.0), the STC is usually at a turning point already — it can't simultaneously be rising/falling AND at the extreme needed for entry. The TURN condition is fundamentally incompatible with the z-score extreme entry.
2. **No-TURN configs: 51-64% filter rate:** Even without the TURN requirement, the STC thresholds exclude most entries. The best (LT30_ST70_N) loses -$138.
3. **STC thresholds don't improve WR:** Filtered entries don't selectively remove losing trades — WR barely changes while total PnL collapses.
4. **STC is a trend-following oscillator:** Like many indicators in this series, it's designed for trend confirmation, not mean-reversion timing.

**Key insight:** The STC's MACD + stochastic hybrid was intended to combine trend and momentum, but at the specific moments z-score reaches ±2.0 (the entry points), the STC is typically at an extreme of its own range, making it a poor confirmation tool.

## Conclusion

No COINCLAW changes. STC filtering is incompatible with z-score extreme entries and provides no edge.
