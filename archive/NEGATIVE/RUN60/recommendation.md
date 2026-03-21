# RUN60 — Z-Score Momentum Filter: Recommendation

## Hypothesis
Named: `z_momentum_filter`

Only enter a LONG when z-score is rising (recovering toward mean), and SHORT when z-score is falling — filtering out entries where the indicator hasn't yet begun reverting.

## Results

### RUN60.1 — Z-Momentum Grid Search (9 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All z-momentum configs reduce PnL and WR vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| ZM0.02_LB1 (best) | +$61.32 | -$115.65 | 50.4% | 5,743 | 62% |
| ZM0.15_LB2 (worst) | +$34.88 | -$142.09 | 50.7% | 3,307 | 78% |

**Key findings:**
- ALL 8 z-momentum configs produce lower PnL AND lower WR than baseline
- WR drops from 54.7% to 50-51% — the filter removes the BEST trades
- Trade count reduction is 62-78% — far too aggressive
- Z-momentum filter blocks entries at exactly the wrong time: when z is rising for longs (reversion already started), those are the best entries
- The hypothesis is backwards: z being rising at LONG entry means reversion has already begun, so the trade works better, not worse

**Why it fails:** The z-momentum filter removes the best trades. When z is rising at LONG entry (z_delta > 0), the reversion has already begun — these are the most profitable entries. Blocking them reduces both trade count AND win rate.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Z-momentum filtering is counterproductive. The best regime trades occur when z-score is already recovering — filtering these out reduces both WR and trade count simultaneously.

## Files
- `run60_1_results.json` — Grid search results
- `coinclaw/src/run60.rs` — Implementation
