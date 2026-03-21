# RUN94 — Partial Reentry After Cooldown: Recommendation

## Hypothesis

**Named:** `partial_reentry_after_cooldown`

Allow partial re-entry during cooldown period when z-score becomes more extreme than original entry:
- **Z_MULT**: z-score must be this many times more extreme than original entry z
- **SIZE_PCT**: re-entry at this fraction of normal position size
- **MAX_COUNT**: max consecutive re-entries per coin per cycle

## Results

### RUN94.1 — Grid Search (36 configs × 18 coins, 5-month 15m data)

**POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| ZM1.1_SP0.7_MC1 (best) | +$317.26 | +$24.41 | 25.9% | 10,994 | 0.38 |
| ZM1.2_SP0.5_MC1 | +$316.87 | +$24.02 | 25.9% | 10,980 | 0.38 |
| ZM1.1_SP0.5_MC1 | +$316.76 | +$23.91 | 25.9% | 10,980 | 0.38 |
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 |

**Key findings:**
- Top 15 configs all beat baseline by $10-24
- Partial re-entry adds ~1,278 extra trades (10,994 vs 9,716)
- Re-entry size at 70% of normal (SP0.7) with z_mult=1.1 works best
- max_count=1 is sufficient — higher counts don't improve results

### RUN94.2 — Walk-Forward Validation (3 windows, best config ZM1.1_SP0.7_MC1)

**POSITIVE — 3/3 windows pass, avg test Δ = +$5.37**

| Window | Train Δ | Test Δ | Pass? |
|--------|---------|--------|-------|
| 1 | +$4.97 | +$7.89 | ✓ |
| 2 | +$12.18 | +$7.43 | ✓ |
| 3 | +$15.72 | +$0.80 | ✓ |

## Conclusion

**POSITIVE — Apply partial re-entry after cooldown**

Best config: `Z_MULT=1.1, SIZE_PCT=0.7, MAX_COUNT=1`

Mechanism: After a position stops out, if z-score reaches 1.1× the original entry z-score during the 2-bar cooldown, enter at 70% position size. This catches continuation moves while maintaining the cooldown period for non-extreme reversals.

## Files
- `run94_1_results.json` — Grid search results
- `run94_2_results.json` — Walk-forward validation
- `coinclaw/src/run94.rs` — Grid search implementation
- `coinclaw/src/run94_2.rs` — Walk-forward validation
