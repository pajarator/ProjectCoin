# RUN48 — Z-Score Recovery Suppression: Recommendation

## Hypothesis
Named: `z_recovery_suppression`

Block re-entries for N bars after a regime trade exit unless z-score has drifted back below threshold (fresh deviation), preventing "chase" entries after mean-reversion has already completed.

## Results

### RUN48.1 — Grid Search (41 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All suppression configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| SB4_T-0.50_W (best) | +$170.40 | -$6.57 | 54.4% | 14,789 |
| SB6_T-0.50_W | +$165.20 | -$11.77 | 54.4% | 14,348 |
| SB8_T-0.50_W | +$161.02 | -$15.95 | 54.4% | 14,001 |
| SB12_T-0.50_W | +$156.64 | -$20.33 | 54.5% | 13,420 |
| SB16_T-0.50_A (worst) | +$136.07 | -$40.90 | 54.2% | 11,850 |

**Key findings:**
- ALL 40 suppression configs produce lower PnL than baseline (no suppression)
- Longer suppress windows and higher trade reduction correlate with larger PnL losses
- Mode W (after_win_only) consistently outperforms Mode A (after_any_exit) — suppressing only after wins is less harmful than suppressing after all exits
- The suppression blocks more winning re-entries than losing ones
- Trade count reduction ranges from -2% (SB4) to -21% (SB16_A), but PnL loss is proportionally larger than the trade reduction

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Blocking re-entries after regime trade exits does not improve P&L. The "chase" hypothesis is not supported: re-entries immediately after exit (when z is near mean) are still profitable on net. The z-score recovery suppression filter removes more winning opportunities than losing ones.

## Files
- `run48_1_results.json` — Grid search results
- `coinclaw/src/run48.rs` — Implementation
