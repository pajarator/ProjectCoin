# RUN56 — SMA Cross-Back Depth Filter: Recommendation

## Hypothesis
Named: `sma_cross_depth_filter`

Require SMA cross-back to exceed a minimum penetration depth before the SMA exit fires, filtering out shallow touches in ranging markets.

## Results

### RUN56.1 — SMA Depth Grid Search (6 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All configs identical to baseline (no effect).**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| SD0.002–SD0.006 | +$176.97 | $0.00 | 54.7% | 15,101 |

**Key findings:**
- ALL 5 depth configs produce identical results to baseline — the SMA depth filter never triggers
- The z-score exit (`z > 0.5` for LONG, `z < -0.5` for SHORT) fires before the SMA cross-back can develop
- Trades are held for only 1-2 bars typically, insufficient for price to cross SMA20 from entry
- The SMA exit in COINCLAW's actual implementation is gated differently (uses open price vs SMA20 comparison, not close price)
- This implementation may not accurately model COINCLAW's SMA exit mechanism

**Why it fails:** In the backtest simulation, z-score signal exits (z reversion) always fire before SMA cross-back can develop. The minimum hold of 2 bars (`i >= 2`) and z-exit together mean positions rarely survive long enough for SMA cross to be the exit reason.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The simulation's z-score exit mechanism prevents the SMA exit from ever firing, making the depth filter untestable with this backtester design. The actual COINCLAW implementation uses different SMA exit logic that may not have this issue.

## Files
- `run56_1_results.json` — Grid search results
- `coinclaw/src/run56.rs` — Implementation
