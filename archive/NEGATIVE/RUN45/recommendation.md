# RUN45 — Complement-Scalp Mutual Exclusion + Exhaustion Timer: Recommendation

## Hypothesis
Named: `complement_scalp_exclusion`

After a complement long fires, suppress scalp entries for N bars to avoid wasting scalp slots on redundant trades.

## Results

### RUN45.1 — Grid Search (5 configs × 18 coins, 5-month 15m data)

**NEGATIVE — improvements are within noise.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| EX8_120 (best) | +$66.86 | +$0.11 | 51.1% | 6,299 |
| DISABLED (baseline) | +$66.76 | — | 51.1% | 6,323 |
| EX2_30 | +$66.76 | +$0.00 | 51.1% | 6,323 |
| EX4_60 | +$66.37 | -$0.38 | 51.0% | 6,319 |
| EX16_240 | +$65.61 | -$1.15 | 51.0% | 6,275 |

**Key finding:** Best config (EX8) shows +$0.11 improvement — 0.16% of baseline. Within rounding error. No consistent positive trend across configs. EX2_30 is identical to DISABLED (exhaustion of 2 bars has no effect because scalp and complement rarely overlap within 2 bars).

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The complement-scalp mutual exclusion hypothesis is not supported. The effect is too small to be meaningful. The improvement of $0.11 on a $66.76 portfolio is within measurement noise and would not survive walk-forward validation.

## Files
- `run45_1_results.json` — Grid search results
- `run45.rs` — Implementation
