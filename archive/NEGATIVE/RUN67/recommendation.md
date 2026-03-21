# RUN67 — Scalp Entry Z-Score Threshold Tightening: Recommendation

## Hypothesis
Named: `scalp_tighter_z_threshold`

Require scalp entries to have more extreme z-score (|z| >= 2.0 or higher) to improve scalp win rate on tight 0.10% SL.

## Results

### RUN67.1 — Scalp Z-Threshold Grid Search (6 configs × 18 coins, 5-month 1m data)

**NEGATIVE — Tighter z-threshold configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Blocked | TP | SL |
|--------|------|------|-----|--------|---------|----|----|
| DISABLED (baseline) | +$220.44 | — | 23.2% | 19,010 | 0 | 4,416 | 14,594 |
| Z+1.5 | +$220.44 | $0.00 | 23.2% | 19,010 | 0 | 4,416 | 14,594 |
| Z+1.8 | +$220.44 | $0.00 | 23.2% | 19,010 | 0 | 4,416 | 14,594 |
| Z+2.0 | +$220.44 | $0.00 | 23.2% | 19,010 | 0 | 4,416 | 14,594 |
| Z+2.2 | +$168.45 | -$51.99 | 23.6% | 14,352 | 5,763 | 3,380 | 10,972 |
| Z+2.5 | +$118.26 | -$102.18 | 23.8% | 10,006 | 11,129 | 2,382 | 7,624 |

**Key findings:**
- Thresholds 1.5/1.75/2.0 have ZERO effect — blocked=0 for all because scalp entry already requires z < -2.0 (LONG) or z > 2.0 (SHORT)
- The scalp entry's base z-condition is already more extreme than the tested thresholds, so no additional filtering occurs
- Only Z+2.2 and Z+2.5 actually block entries, but WR improves only +0.4-0.6pp while trade count drops 24-47%
- PnL loss from fewer trades far exceeds marginal WR improvement
- Scalp WR remains abysmal (23%) — far below breakeven — because the tight 0.10% SL is hit by normal volatility

**Why it fails:** Scalp entries already require extreme z-score (|z| >= 2.0). The z-threshold filter in this simulation is redundant with the base scalp entry condition. Any further tightening removes entries that do win occasionally, hurting PnL.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The scalp strategy's z-score filter is already at |z| >= 2.0. Testing tighter thresholds (1.5/1.75) shows no additional filtering. Tighter thresholds (2.25/2.5) remove trades without improving WR enough to compensate.

## Files
- `run67_1_results.json` — Grid search results
- `coinclaw/src/run67.rs` — Implementation