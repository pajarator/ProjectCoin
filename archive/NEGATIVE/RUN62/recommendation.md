# RUN62 — Regime Breakeven Stop: Recommendation

## Hypothesis
Named: `regime_breakeven_stop`

Add a breakeven stop to regime trades — once profit reaches a threshold (e.g., 0.5%), move SL to entry price to lock in gains without exiting.

## Results

### RUN62.1 — Breakeven Stop Grid Search (6 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All configs identical to baseline (no effect).**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| BE0.003–BE0.008 | +$176.97 | $0.00 | 54.7% | 15,101 |

**Key findings:**
- ALL 5 breakeven configs produce identical results to baseline — the BE stop never activates
- Z-score signal exit (`z > 0.5` for LONG, `z < -0.5` for SHORT) fires before profit can reach BE threshold
- With 2-bar minimum hold and z-exit as primary exit, positions rarely hold long enough to reach BE activation levels
- The backtest's exit hierarchy (SL → Z-exit → timeout) prevents ever reaching breakeven activation

**Why it fails:** In this backtester, z-score exit always fires before profit can accumulate to the BE threshold. The actual COINCLAW implementation may have different exit timing that could allow BE to work.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The backtest simulation exits positions via z-score signal before breakeven activation can occur. The simulation's exit hierarchy prevents the BE mechanism from ever engaging.

## Files
- `run62_1_results.json` — Grid search results
- `coinclaw/src/run62.rs` — Implementation
