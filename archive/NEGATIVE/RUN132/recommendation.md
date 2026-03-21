# RUN132 — RSI Divergence Confirmation: STRONGLY NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +359.05 | — | 38.9% | 0.0% |
| LB14_T1 | +87.82 | -271.24 | 32.5% | 87.1% |
| LB10_T1 | +85.79 | -273.27 | 32.1% | 87.0% |

**VERDICT: STRONGLY NEGATIVE** — RSI Divergence filter achieves 87-99% filter rate and collapses PnL by -$271 to -$338. WR drops from 38.9% to 24-33%, removing BETTER trades.

## Analysis

**Filter mechanism:** For LONG: require price makes a new local low while RSI forms a higher low (bullish hidden divergence). For SHORT: require price new high, RSI lower high.

**Why it fails:**
1. **87-99% filter rate:** At the moment z-score reaches ±2.0, RSI divergence almost never coincides with it.
2. **WR drops significantly:** From 38.9% to 24-33%. The filtered entries were actually BETTER than average — the filter removes the best opportunities.
3. **Divergence detection is incompatible with extremes:** By definition, RSI divergence requires comparing current RSI to prior RSI at a price extreme. At z-score extreme, RSI is already at an extreme — the divergence pattern rarely forms simultaneously.
4. **Tolerance doesn't help:** Even with 5% tolerance (LB*_T5), filter rate stays at 98-99%.

## Conclusion

No COINCLAW changes. RSI divergence is fundamentally incompatible with z-score extreme entries.
