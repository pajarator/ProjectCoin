# RUN99 — Z-Score Momentum Divergence: Recommendation

## Hypothesis

**Named:** `z_momentum_divergence`

Exit when z-score and price momentum diverge:
- Track `z_momentum = z_current - z_N_bars_ago` (5-bar lookback)
- When z_momentum indicates divergence for DIVERGENCE_BARS consecutive bars, exit
- Rationale: if price makes new extremes while z-score fails to confirm, the move lacks conviction

## Results

### RUN99.1 — Grid Search (12 portfolio-level configs × 18 coins)

**NEGATIVE — All 12 configs produce identical results to baseline (0 divergence exits).**

| Config | PnL | ΔPnL | Trades | DivExits |
|--------|------|------|--------|---------|
| BASELINE | +$208.43 | — | 8,933 | 0 |
| All configs T0.3_B2 through T1.0_B5 | +$208.43 | $0.00 | 8,933 | 0 |

**Key findings:**
- The divergence condition `z_momentum < threshold (negative)` for DIVERGENCE_BARS consecutive bars never fires
- z_momentum and price momentum are rarely oppositely signed during mean-reversion trades
- After entry (z extreme), as price moves in our direction, z recovers toward 0 — they don't diverge during successful trades

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run99_1_results.json` — Grid search results
- `coinclaw/src/run99.rs` — Implementation
