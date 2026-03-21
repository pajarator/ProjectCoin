# RUN69 — Winning Streak Profit-Taking: Recommendation

## Hypothesis
Named: `winning_streak_profit_take`

After N consecutive winning trades, trigger "streak mode" that lowers the z-exit threshold (exit earlier at z > 0.3 instead of z > 0.5) to lock in gains before the favorable regime ends.

## Results

### RUN69.1 — Winning Streak Grid Search (13 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All streak configs produce near-identical PnL to baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| SC2_SE-0.3 (best) | +$326.26 | +$1.51 | 36.5% | 11,968 |
| SC3_SE-0.3 | +$325.68 | +$0.93 | 36.3% | 11,947 |
| SC4_SE-0.3 | +$325.28 | +$0.54 | 36.2% | 11,938 |
| SC5_SE-0.3 | +$325.02 | +$0.28 | 36.2% | 11,931 |
| SC4_SE-0.4 | +$324.80 | +$0.06 | 36.2% | 11,931 |
| BASELINE (SC0_SE-0.5) | +$324.75 | — | 36.1% | 11,928 |
| SC2_SE-0.4 | +$323.29 | -$1.46 | 36.2% | 11,941 |

**Key findings:**
- All configs produce nearly identical PnL (range: $323-$326, spread of only $3)
- SE-0.5 configs are completely identical to baseline — streak never activates with z_exit at 0.5 threshold
- SE-0.3 (exit at z > 0.3) slightly helps because it exits earlier before gains can reverse
- Trade count variation is minimal (11,928-11,968)
- The "streak" mechanism has negligible effect because z-exit fires so rarely in the simulation

**Why it fails:** The z-exit threshold (z > 0.5 or z < -0.5) fires very infrequently in the backtest. After 2 consecutive wins, requiring z > 0.3 (instead of 0.5) barely changes anything — most trades already exit via regime reversal or timeout before the z-exit would trigger. The streak mechanism doesn't meaningfully alter trade outcomes.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The winning streak profit-taking mechanism produces negligible PnL differences ($1-3 range across all configs). The z-exit is too rare to be meaningfully modulated by streak count.

## Files
- `run69_1_results.json` — Grid search results
- `coinclaw/src/run69.rs` — Implementation