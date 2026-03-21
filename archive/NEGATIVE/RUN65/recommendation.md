# RUN65 — BB Squeeze Duration Filter: Recommendation

## Hypothesis
Named: `bb_squeeze_duration_filter`

Require minimum consecutive Bollinger Band squeeze bars (BB width < 60% of 20-bar average) before entering a regime trade.

## Results

### RUN65.1 — BB Squeeze Duration Grid Search (7 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All BB squeeze duration configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| MS4_EM1 (best) | +$118.45 | -$58.51 | 54.8% | 11,482 | 24% |
| MS4_EM2 | +$118.45 | -$58.51 | 54.8% | 11,482 | 24% |
| MS6_EM1 | +$112.59 | -$64.38 | 55.0% | 10,959 | 27% |
| MS6_EM2 | +$112.59 | -$64.38 | 55.0% | 10,959 | 27% |
| MS8_EM1 | +$105.27 | -$71.70 | 54.6% | 10,355 | 31% |
| MS8_EM2 | +$105.27 | -$71.70 | 54.6% | 10,355 | 31% |

**Key findings:**
- ALL 6 BB squeeze configs produce lower PnL than baseline
- WR improves marginally (+0.1-0.3pp) but trade count reduction (24-31%) dominates
- MS4 (minimum 4 squeeze bars) is least harmful; MS8 most harmful
- EXIT_MODE has zero effect — exit behavior is the same for strict vs relaxed
- The squeeze filter removes ~24-31% of entries, PnL loss exceeds marginal WR improvement

**Why it fails:** Trade count reduction of 24-31% cannot be compensated by WR improvement of 0.1-0.3pp. The squeeze filter removes entries that are, on average, profitable enough that their removal hurts more than it helps.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

BB squeeze duration filtering does not improve COINCLAW's regime strategy. Trade count reduction dominates the marginal WR improvement.

## Files
- `run65_1_results.json` — Grid search results
- `coinclaw/src/run65.rs` — Implementation