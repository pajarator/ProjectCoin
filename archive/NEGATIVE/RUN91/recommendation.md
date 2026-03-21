# RUN91 — Hourly Z-Threshold Scaling: Recommendation

## Hypothesis

**Named:** `hourly_z_threshold`

Scale entry z-threshold by time-of-day:
- High-vol hours (UTC 13-20): tighten threshold (z < -1.5 × mult)
- Low-vol hours (UTC 0-8): relax threshold (z < -1.5 × mult)

## Results

### RUN91.1 — Grid Search (9 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL configs identical to baseline (Δ = $0.00).**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 |
| HM1.1_LM0.7 | +$292.85 | $0.00 | 25.9% | 9,716 |
| HM1.3_LM0.9 | +$292.85 | $0.00 | 25.9% | 9,716 |

**Key findings:**
- ALL 9 configs produce exactly identical PnL to baseline — zero effect
- The hourly threshold is applied AFTER the regime signal (z < -2.0)
- Since z < -2.0 is always more extreme than any scaled threshold (-1.2 to -1.95), the additional condition is never binding
- The mechanism is fundamentally flawed: the regime signal already enforces the threshold that the hourly scaling tries to add

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The hypothesis had a logical flaw: the regime signal requires z < -2.0, which is already more extreme than any hourly-adjusted threshold (e.g., -1.8 or -1.2). The hourly scaling would only matter if it could tighten the effective threshold below -2.0, but the base z-score range in this dataset doesn't reach those levels consistently enough to create a meaningful distinction.

## Files
- `run91_1_results.json` — Grid search results
- `coinclaw/src/run91.rs` — Implementation
