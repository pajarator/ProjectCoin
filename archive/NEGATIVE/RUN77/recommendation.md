# RUN77 — Z-Score Recovery Rate Exit: Recommendation

## Hypothesis
Named: `recovery_rate_exit`

Mechanically: Exit when `recovery_velocity = (z_entry - z_curr) / effective_held < MIN_VELOCITY`. Forces close on stalling mean reversion trades.

## Results

### RUN77.1 — Recovery Rate Grid Search (37 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | RecEx% |
|--------|------|------|-----|--------|---------|
| BASELINE | +$293.21 | — | 25.9% | 9,714 | 0.0% |
| MB30_MV0.015_GR5 (best) | +$292.25 | -$0.96 | 26.5% | 9,907 | 2.8% |
| MB30_MV0.020_GR15 | +$291.82 | -$1.40 | 26.5% | 9,898 | 2.7% |
| MB15_MV0.010_GR5 (worst) | +$287.06 | -$6.15 | 26.6% | 9,985 | 4.4% |

**Key findings:**
- ALL 36 configs produce lower PnL than baseline
- WR improves marginally: +0.4 to +1.3pp
- Recovery exit rate only 2-6% — too few to matter
- Cutting short stalling trades also cuts short some that would have recovered
- Recovery exits are cutting winners prematurely

**Why it fails:** Slow recovery is not a reliable predictor of trade failure. Many trades that appear to be stalling (low recovery velocity) subsequently reverse sharply. The recovery exit cuts winners short before they can realize their full profit. The exit is not discriminating enough.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run77_1_results.json` — Grid search results
- `coinclaw/src/run77.rs` — Implementation
