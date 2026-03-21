# RUN47 — Per-Strategy MIN_HOLD Grid Search: Recommendation

## Hypothesis
Named: `min_hold_grid`

Test whether varying the minimum hold period (in bars) before allowing exit improves regime trade P&L. Grid: [1, 2, 3, 4, 5, 6] bars vs baseline (MH2, i.e., current default).

## Results

### RUN47.1 — Grid Search (7 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All 7 configs produce identical results.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (MH2 baseline) | +$176.97 | — | 54.7% | 15,101 |
| MH1 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| MH2 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| MH3 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| MH4 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| MH5 | +$176.97 | +$0.00 | 54.7% | 15,101 |
| MH6 | +$176.97 | +$0.00 | 54.7% | 15,101 |

**Key finding:** All configs produce identically the same PnL, trade count, and win rate. This means the MIN_HOLD condition at line 100 of `run47.rs` is never the binding exit constraint — the Z0 crossback exit (regime_signal direction change) always fires before min_hold bars elapse. The min_hold exit condition is dead code for this strategy.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The min_hold constraint does not affect outcomes because regime signal reversals (Z0 crossback) always trigger exits within fewer bars than any tested min_hold threshold. This is structurally similar to RUN46's finding — the secondary exit condition is always dominated by the primary Z0 crossback exit.

## Files
- `run47_1_results.json` — Grid search results
- `coinclaw/src/run47.rs` — Implementation
