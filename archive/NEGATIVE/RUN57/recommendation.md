# RUN57 — Day-of-Week Trade Filter: Recommendation

## Hypothesis
Named: `day_of_week_filter`

Suppress or selectively allow trades based on day-of-week to avoid low-edge days (Monday weekend unwind, Friday pre-weekend).

## Results

### RUN57.1 — Day-of-Week Grid Search (17 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All day-of-week filter configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| SM1_M0_F1_W1 (best) | +$150.52 | -$26.44 | 54.8% | 12,978 |
| SM2_M0_F0_W0 (worst non-zero) | +$91.22 | -$85.75 | 55.3% | 8,575 |

**Key findings:**
- ALL 16 day-of-week filter configs produce lower PnL than baseline
- WR improves slightly (+0.5-1.6pp) but trade count reduction dominates
- Weekend suppression alone (W0) reduces PnL by $43-96 depending on other settings
- Friday suppression reduces PnL by $26-30
- SM2 configs (allow good days only) perform worst — they block too many trades

**Why it fails:** Day-of-week effects are not significant enough for COINCLAW's regime strategy. Crypto trades 24/7 without traditional market hours. WR improvements (+0.5-1.6pp) cannot compensate for the trade count reduction. All days contribute roughly equally to P&L.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Day-of-week filtering does not improve COINCLAW's regime strategy. All days contribute roughly equally to performance; no single day is significantly harmful or beneficial.

## Files
- `run57_1_results.json` — Grid search results
- `coinclaw/src/run57.rs` — Implementation
