# RUN92 — Exit Reason Weighted Learning: Recommendation

## Hypothesis

**Named:** `exit_weighted_signals`

Track per-coin historical exit quality, suppress entries when expected_exit_score < threshold based on fraction of historical winning exits.

## Results

### RUN92.1 — Grid Search (18 configs × 18 coins, 5-month 15m data)

**CATASTROPHIC NEGATIVE — All configs catastrophic suppression.**

| Config | PnL | ΔPnL | WR% | Trades | Suppressed |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0 |
| W30_MS15_SC0.50 (best) | +$4.62 | -$288.23 | 23.0% | 270 | 251,853 |
| W30_MS20_SC0.70 (worst) | +$4.54 | -$288.31 | 20.8% | 360 | 249,711 |

**Key findings:**
- All non-baseline configs: only 270-360 trades vs 9,716 baseline (97% reduction)
- Suppression count in hundreds of thousands per coin (far exceeding total bars)
- The suppression counter was incrementing every bar during cooldown, not per entry opportunity
- Once exit history accumulates mostly negative exits (COINCLAW WR = 25.9%), exit_score drops below threshold and blocks nearly all entries permanently
- The mechanism collapses once the strategy's poor win rate poisons the historical data

**Why it fails:** The mechanism is fundamentally flawed in a losing strategy: with 74% of exits being losses, the historical "exit quality" score rapidly drops below any reasonable threshold, permanently suppressing all entries. The learning mechanism punishes itself by using the strategy's own poor performance as the gate for future entries.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run92_1_results.json` — Grid search results
- `coinclaw/src/run92.rs` — Implementation
