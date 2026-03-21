# RUN111 — MACD Histogram Slope Exit: Recommendation

## Hypothesis

**Named:** `macd_histogram_exit`

Exit when MACD histogram crosses zero or fades to a fraction of its entry value, confirming mean-reversion momentum has been exhausted.

## Results

### RUN111.1 — Grid Search (6 configs × 18 coins, 5-month 15m data)

**NEGATIVE** — All configs produce identical PnL to baseline.

| Config | PnL | ΔPnL | WR% | Trades | HistExits |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0 |
| TR0.30_FLIP | +$360.55 | $0.00 | 39.0% | 13,689 | 0 |
| TR0.70_NOFLIP | +$360.55 | $0.00 | 39.0% | 13,689 | 0 |

**Key findings:**
- ALL 6 configs produce zero histogram exits — hist_exits_total = 0 across all coins
- MACD histogram check is placed after Z0/SMA20/regime-flip exits; trades always exit via those conditions first
- MACD histogram never gets a chance to fire as an exit trigger
- Test is inconclusive: the exit priority ordering prevents the hypothesis from being evaluated

## Conclusion

**NEGATIVE (inconclusive).** The MACD histogram exit cannot be evaluated with the current exit priority ordering — existing exits (Z0 reversion, SMA20 crossback, regime signal flip) always close trades before MACD histogram can trigger. To properly test this, MACD histogram would need to be checked BEFORE Z0/SMA20 exits, which would be a structural change. Alternatively, the hypothesis may simply not be testable in this framework.

## Files
- `run111_1_results.json` — Grid search results
- `coinclaw/src/run111.rs` — Implementation
