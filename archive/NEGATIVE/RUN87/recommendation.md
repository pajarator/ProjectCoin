# RUN87 — Drawdown Recovery Mode: Recommendation

## Hypothesis

**Named:** `drawdown_recovery_mode`

When portfolio drawdown >= DD_RECOVERY_THRESHOLD (8-15%), shift market mode bias:
- Tighten ISO_SHORT_BREADTH_MAX (lower threshold → more ISO shorts fire)
- Widen BREADTH_MAX for LONG (higher threshold → fewer LONG entries)

## Results

### RUN87.1 — Grid Search (81 configs × 18 coins, 5-month 15m data)

**CATASTROPHIC — ALL configs identical to baseline, recovery never activates.**

| Config | PnL | ΔPnL | WR% | Trades | Recovery% |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$20.56 | — | 26.8% | 817 | 91.8% |
| TH0.08_EX0.04_IT0.03_LW0.03 (best) | +$20.56 | $0.00 | 26.8% | 817 | 0.0% |

**Key findings:**
- ALL 81 non-baseline configs show 0.0% recovery activation rate
- The drawdown thresholds (8%, 10%, 15%) are never reached during the 5-month period
- With only 817 trades vs 9,716 in the full-regime run86, this simulation starts from a reduced capital base (INITIAL_BAL=100 vs 100 in run86, but portfolio-level simulation with different structure)
- The baseline recovery rate of 91.8% is a bug — baseline has threshold=0.0 so it enters recovery on any drawdown > 0

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The 5-month dataset (14400 bars) does not contain a 8%+ portfolio drawdown event, making all thresholds un-testable. The hypothesis cannot be validated with current data.

## Files
- `run87_1_results.json` — Grid search results
- `coinclaw/src/run87.rs` — Implementation
