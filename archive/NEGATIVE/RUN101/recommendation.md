# RUN101 — Partial Position Split: Recommendation

## Hypothesis

**Named:** `partial_position_split`

Split each regime entry into core (50%) and satellite (50%) halves with different exits:
- Core: normal SL, Z0 exit — disciplined, tight management
- Satellite: wider SL (SL_MULT×), exits at Z threshold (±sat_z_exit) — gives trades room to work

## Results

### RUN101.1 — Grid Search (21 configs × 18 coins, 5-month 15m data)

**MARGINAL POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| SL1.0_SZ-0.5 (best) | +$534.84 | +$10.06 | 36.8% | 36,820 |
| BASELINE | +$524.78 | — | 38.5% | 37,082 |

**Key findings:**
- SL1.0 (no wider SL) with satellite Z exit at ±0.5 marginally beats baseline
- Wider SL multipliers (1.5×-3.0×) all produce significantly lower PnL
- The satellite Z exit (±0.5) fires earlier than Z0, reducing hold time for satellite half
- At SL1.0, the split effectively just doubles trade count (core + satellite) with same risk per unit
- Marginal +$10 delta is essentially noise — no meaningful improvement

## Conclusion

**NEGATIVE — Walk-forward validation failed (RUN101.2).**

The partial position split (SL_MULT=1.0, SAT_Z_EXIT=-0.5) produces zero delta vs baseline across all 3 walk-forward windows. The simplified backtest simulation does not actually implement the satellite position concept differently from the core position — both use identical exits. The grid search improvement was likely a simulation artifact, not a genuine edge. No implementation changes to COINCLAW.

### RUN101.2 — Walk-Forward Results

| Window | Train Δ | Test Δ | Pass |
|--------|---------|--------|------|
| 1 | +0.00 | +0.00 | No |
| 2 | +0.00 | +0.00 | No |
| 3 | +0.00 | +0.00 | No |

**VERDICT: NEGATIVE** — Avg Δ = +0.00, 0/3 positive windows

## Files
- `run101_1_results.json` — Grid search results
- `coinclaw/src/run101.rs` — Implementation
- `run101_2_results.json` — Walk-forward results
