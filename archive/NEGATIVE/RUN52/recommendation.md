# RUN52 — Z-Score Deviation Position Sizing: Recommendation

## Hypothesis
Named: `z_confidence_sizing`

Scale position size based on z-score deviation extremity at entry — more extreme deviation = larger position.

## Results

### RUN52.1 — Grid Search (13 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All position sizing configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 |
| CM0.25_ZM3.5 (best) | +$161.54 | -$15.43 | 54.7% | 15,101 |
| CM1.00_ZM2.5 (worst) | +$90.27 | -$86.70 | 54.7% | 15,101 |

**Key findings:**
- All 12 sizing configs produce lower PnL than baseline
- WR is identical (54.7%) across all configs — position sizing only scales monetary outcomes
- Since WR < ~60% breakeven for 5× leverage, amplifying position size amplifies losses more than wins
- Lower CONFIDENCE_MULTIPLIER (0.25) causes least harm; higher multipliers amplify losses proportionally
- The base position size (2%) is already well-calibrated; increasing it for extreme z-scores hurts

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Position sizing amplification hurts when WR is below the breakeven threshold for the given leverage. At 5× leverage with 54.7% WR, increasing position size on high-confidence signals amplifies both wins and losses — but losses are amplified more since WR < 60%. The base position size of 2% per trade is optimal.

## Files
- `run52_1_results.json` — Grid search results
- `coinclaw/src/run52.rs` — Implementation
