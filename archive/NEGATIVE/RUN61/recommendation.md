# RUN61 — RSI Divergence Confirmation: Recommendation

## Hypothesis
Named: `rsi_divergence_confirmation`

Require bullish RSI divergence (price lower low + RSI higher low) to confirm LONG entries.

## Results

### RUN61.1 — RSI Divergence Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All RSI divergence configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| LB8_PD0.003_RR1.0 (best) | +$104.96 | -$72.01 | 56.1% | 8,155 | 46% |
| LB4_PD0.005_RR3.0 | +$90.38 | -$86.59 | 56.1% | 7,499 | 50% |

**Key findings:**
- ALL 27 divergence configs produce lower PnL than baseline
- WR improves +1.2-1.8pp but trade count reduction (46-54%) dominates
- RSI divergence filter blocks ~50% of entries, PnL loss exceeds WR improvement
- The "best" config still loses 41% of PnL despite +1.4pp WR improvement

**Why it fails:** Trade count reduction of ~50% cannot be compensated by WR improvement of 1-2pp. The divergence filter removes entries that are, on average, profitable enough that their removal hurts more than it helps.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

RSI divergence filtering does not improve COINCLAW's regime strategy. Trade count reduction dominates the small WR improvement.

## Files
- `run61_1_results.json` — Grid search results
- `coinclaw/src/run61.rs` — Implementation
