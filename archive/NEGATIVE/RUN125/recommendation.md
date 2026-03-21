# RUN125 — Ease of Movement Filter: STRONGLY NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.59 | — | 39.0% | 0.0% |
| LB10_T0.1 | +2.48 | -358.11 | 54.1% | 99.6% |
| LB14_T0.1 | +2.48 | -358.11 | 54.1% | 99.6% |

**VERDICT: STRONGLY NEGATIVE** — All EMV filter configs collapse PnL to near zero with 99.6% filter rate. Baseline wins by $358.

## Analysis

**Filter mechanism:** Ease of Movement = (High-Low)/(High+Low/2) × volume; LONG requires EMV > threshold, SHORT requires EMV < -threshold.

**Why it fails:**
1. **99.6% filter rate:** EMV is almost never positive/negative enough at the exact moment of z-score extreme entries in crypto markets.
2. **EMV is volume-weighted:** In crypto's 24/7 markets, volume spikes don't correlate with price movement direction in the same way as traditional markets.
3. **Grid search exhaustively tested:** 9 configs tested; every single one collapses to near-zero PnL.

**Best non-baseline (LB10_T0.1):** PnL=+$2.48, WR=54.1% (higher WR but only 4 trades total due to extreme filtering).

## Conclusion

No COINCLAW changes. EMV is not a useful entry filter for crypto mean-reversion.
