# RUN93 — Consecutive Wins Streak Boost: Recommendation

## Hypothesis

**Named:** `streak_risk_boost`

Scale position size based on recent consecutive win/loss streaks:
- After STREAK_WIN_THRESHOLD consecutive wins: increase RISK × WIN_MULT
- After STREAK_LOSS_THRESHOLD consecutive losses: decrease RISK × LOSS_MULT

## Results

### RUN93.1 — Grid Search (81 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 81 configs worse than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 |
| WT2_LT4_WM1.5_LM0.8 (best) | +$273.02 | -$19.82 | 25.9% | 9,716 | 0.35 |
| WT4_LT2_WM1.5_LM0.6 (worst) | +$215.01 | -$77.84 | 25.9% | 9,716 | 0.35 |

**Key findings:**
- ALL 81 configs produce lower PnL than baseline
- Same trade count and WR (streak boost only changes position size, not entry frequency)
- All PnL degradation follows the Kelly fraction logic: with 25.9% WR and asymmetric win/loss sizes, increasing bet size on wins and decreasing on losses is counterproductive
- The strategy's Kelly fraction is negative — the optimal bet is actually to decrease size overall, not scale with streaks

**Why it fails:** With 25.9% WR, wins are smaller than losses on average. Scaling UP position size on wins amplifies small wins. Scaling DOWN on losses cuts the only thing that occasionally produces profits. The mechanism is inverse-Kelly for a negatively skewed strategy.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run93_1_results.json` — Grid search results
- `coinclaw/src/run93.rs` — Implementation
