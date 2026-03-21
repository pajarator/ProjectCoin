# RUN126 — Ichimoku Cloud Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Filter Rate |
|--------|-----|------|-----|-------------|
| BASELINE | +360.58 | — | 39.0% | 0.0% |
| C0_X0_K0 | +360.58 | +0.00 | 39.0% | 0.0% |
| C1_X0_K0 | +356.69 | -3.89 | 38.9% | 1.3% |

**VERDICT: NEGATIVE** — Best non-baseline (C1_X0_K0, cloud-only) loses only -$3.89 but adds implementation complexity for zero benefit. Ichimoku Cloud provides no useful filtering signal.

## Analysis

**Filter mechanism:** Ichimoku Cloud (Tenkan/Kijun cross + Senkou cloud thickness + Chikou confirmation).

**Why it fails:**
1. **Cloud-only (C1_X0_K0) ≈ baseline:** +$356.69 vs +$360.58 baseline, only -$3.89 difference. Adding cloud confirmation provides nothing.
2. **Any cross or Chikou confirmation collapses PnL:** Even modest additional requirements (Kijun cross) dramatically reduce entries and PnL.
3. **Ichimoku is a trend-following tool:** Originally designed for longer-term trends, not 15m mean-reversion scalping.

**Near-baseline config (C1_X0_K0):** Cloud=Yes, Cross=No, Chikou=No. Filter rate 1.3%, loses only -$3.89 vs baseline. This is effectively equivalent to baseline in practice.

## Conclusion

No COINCLAW changes. Ichimoku Cloud adds no value as a filter for the z-score regime strategy.
