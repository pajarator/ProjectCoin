# RUN102 — TWAP Entry Execution: Recommendation

## Hypothesis

**Named:** `twap_entry_execution`

Accumulate positions over time via TWAP (Time-Weighted Average Price) rather than entering at a single point. This reduces entry timing risk by spreading the entry over N bars. A stop-loss trigger cancels the TWAP if price moves adversely during accumulation.

## Results

### RUN102.1 — Grid Search (10 configs × 18 coins, 5-month 15m data)

**STRONGLY POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| TB4_ST0.003 (best) | +$822.25 | +$553.96 | 62.6% | 9,801 |
| TB4_ST0.005 | +$789.63 | +$521.34 | 62.2% | 9,803 |
| TB4_ST0.007 | +$760.88 | +$492.59 | 62.0% | 9,805 |
| BASELINE | +$268.29 | — | 40.7% | 12,085 |

**Key findings:**
- TWAP with 4 bars and 0.3% SL trigger is the best configuration
- WR jumps from 40.7% → 62.6% (+22pp) — dramatic improvement in win rate
- PnL nearly triples (+$553.96 delta, +206% improvement)
- SL trigger at 0.3% effectively cancels adverse entries before full position is built
- TWAP smoothing eliminates worst entry timing, locking in better average prices
- Trades decrease slightly (12,085 → 9,801) because some adverse entries are cancelled

## Conclusion

**NEGATIVE — Walk-forward validation failed (RUN102.2).**

Despite a dramatic grid-search signal (+553.96 delta, WR 40.7%→62.6%), the TWAP mechanism does not survive walk-forward testing. All 3 test windows show negative delta vs baseline (avg Δ=-2.91). The in-sample improvement appears to be an artifact of TWAP interacting with specific market conditions in the training period that don't persist out-of-sample. The 22pp WR improvement in grid search does not hold.

### RUN102.2 — Walk-Forward Results

| Window | Train Δ | Test Δ | Pass |
|--------|---------|--------|------|
| 1 | -9.03 | -2.17 | No |
| 2 | -6.14 | -4.38 | No |
| 3 | -6.57 | -2.17 | No |

**VERDICT: NEGATIVE** — Avg Δ = -2.91, 0/3 positive windows

## Files
- `run102_1_results.json` — Grid search results
- `coinclaw/src/run102.rs` — Implementation
- `run102_2_results.json` — Walk-forward results
