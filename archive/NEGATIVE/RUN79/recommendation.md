# RUN79 — Breadth-Adaptive Position Sizing: Recommendation

## Hypothesis
Named: `breadth_risk_scaling`

Mechanically: Scale regime RISK based on breadth (market-wide % of coins below z=-1.5). Boost LONG risk in low-breadth, reduce in high-breadth, boost ISO_SHORT risk in high-breadth.

## Results

### RUN79.1 — Breadth-Adaptive Position Sizing Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 27 configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE | +$2,106.97 | — | 25.9% | 9,716 |
| LB1.5_LR0.9_IB1.5 (best) | +$2,032.78 | -$74.19 | 25.9% | 9,716 |
| LB1.1_LR0.7_IB1.1 (worst) | +$1,757.51 | -$349.47 | 25.9% | 9,716 |

**Key findings:**
- ALL 27 configs produce lower PnL than baseline
- WR unchanged at 25.9% — breadth-adaptive sizing doesn't improve trade quality
- Trade count unchanged — sizing changes only affect position size, not entry/exit counts
- Position sizing changes don't compensate — the underlying trade quality is unchanged

**Why it fails:** Scaling position size with breadth doesn't change whether trades win or lose. It only changes the magnitude of wins and losses. Since WR is unchanged, the expected value of each trade is unchanged, and any change in position size symmetrically affects both wins and losses.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run79_1_results.json` — Grid search results
- `coinclaw/src/run79.rs` — Implementation
