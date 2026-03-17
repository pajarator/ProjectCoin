# RUN26 — ATR-Based Dynamic Stops Grid Search

## Goal

Test whether replacing the fixed 0.3% stop loss with ATR-based dynamic stops and/or trailing stops improves strategy profitability. ATR stops adapt to current volatility; trailing stops are intended to let winners run further — both could lower the effective breakeven win rate.

## Fix Over Python Stub

Python `run26_1_atr_stops.py` selected the "best strategy" on full data (look-ahead bias) and evaluated on full data. **Corrected:** uses COINCLAW v13 per-coin primary strategy from `COIN_STRATEGIES`; grid search on train half (67%); OOS eval on test half (33%).

## Method

- **Data:** 18 coins, 15m 1-year OHLCV
- **Split:** 67% train / 33% OOS test
- **Strategy:** COINCLAW v13 per-coin primary long (vwap_rev, bb_bounce, adr_rev, dual_rsi, mean_rev)
- **Grid (255 combos per coin):**
  - ATR mult: [0.5, 0.75, 1.0, 1.5, 2.0]
  - ATR period: [7, 14, 21]
  - Trail configs (17): none + 4 trail_pct × 4 activation combinations
- **ATR stop:** `stop_price = entry_price − ATR[entry_bar] × atr_mult`
- **Trailing stop:** once price rises ≥ activation above entry, trail at `peak × (1 − trail_pct)`
- **Scoring on train:** `PF × sqrt(trades) / max(max_dd, 1)`
- **Key metrics:** WR%, PF, avg_win%, avg_loss%, breakeven WR = `avg_loss / (avg_win + avg_loss)`

## Results (OOS Test Half — 33% Hold-out)

### Per-Coin: Baseline (fixed 0.3% SL) vs Best ATR Config

| Coin | Bt | BWR% | BPF | BPnL% | BEwr | | At | AWR% | APF | APnL% | AEwr | dPF | dWR | Best Config |
|------|----|----|-----|------|------|---|----|----|-----|------|------|------|-----|-------------|
| DASH | 744 | 28.8 | 1.21 | +54.9 | 25.1 | | 834 | 37.3 | **1.29** | +69.3 | 31.5 | **+0.085** | +8.5 | ATR0.75×p14 trail0.3%@imm |
| UNI  | 1295 | 35.1 | 0.76 | -94.5 | 41.6 | | 1491 | 35.2 | 0.70 | -127.6 | 43.9 | -0.062 | +0.2 | ATR1.50×p7 trail0.3%@imm |
| NEAR | 1292 | 36.0 | 0.81 | -74.5 | 41.0 | | 1485 | 37.9 | 0.77 | -95.7 | 44.4 | -0.044 | +1.9 | ATR1.50×p7 trail0.3%@imm |
| ADA  | 1277 | 36.6 | 0.71 | -106.3 | 44.8 | | 1006 | 43.2 | 0.61 | -147.3 | 55.5 | -0.098 | +6.7 | ATR2.00×p14 trail0.8%@imm |
| LTC  | 1140 | 35.9 | 0.60 | -126.7 | 48.4 | | 838 | 43.9 | 0.52 | -150.5 | 60.2 | -0.078 | +8.0 | ATR2.00×p21 trail1.0%@imm |
| SHIB | 1221 | 32.8 | 0.58 | -154.9 | 45.7 | | 1431 | 30.9 | 0.50 | -205.9 | 47.0 | -0.074 | -1.9 | ATR1.00×p7 trail0.3%@imm |
| LINK | 1182 | 38.0 | 0.77 | -76.0 | 44.4 | | 1385 | 35.7 | 0.66 | -130.3 | 45.9 | -0.110 | -2.2 | ATR0.75×p21 trail0.3%@imm |
| ETH  | 1075 | 31.9 | 0.62 | -117.1 | 43.1 | | 869 | 38.1 | 0.53 | -145.1 | 53.5 | -0.085 | +6.2 | ATR2.00×p14 trail0.8%@imm |
| DOT  | 1241 | 36.9 | 0.74 | -95.7 | 44.2 | | 1195 | 41.3 | 0.66 | -135.5 | 51.5 | -0.074 | +4.4 | ATR2.00×p7 trail0.5%@imm |
| XRP  | 1192 | 31.5 | 0.57 | -150.8 | 44.8 | | 1121 | 34.9 | 0.53 | -178.6 | 50.5 | -0.043 | +3.3 | ATR1.50×p21 trail0.5%@imm |
| ATOM | 1203 | 36.2 | 0.66 | -118.6 | 46.2 | | 949 | 43.9 | 0.58 | -152.0 | 57.5 | -0.081 | +7.7 | ATR2.00×p21 trail0.8%@imm |
| SOL  | 1155 | 33.3 | 0.62 | -129.7 | 44.8 | | 1314 | 31.7 | 0.56 | -162.2 | 45.1 | -0.052 | -1.7 | ATR1.50×p7 trail0.3%@imm |
| **DOGE** | 347 | 30.3 | 0.66 | -40.4 | 39.8 | | 391 | 31.2 | **0.73** | -27.9 | 38.2 | **+0.076** | +0.9 | ATR0.50×p7 trail0.3%@imm |
| XLM  | 504 | 31.3 | 0.63 | -59.8 | 42.1 | | 487 | 37.4 | 0.60 | -68.4 | 49.7 | -0.024 | +6.0 | ATR0.75×p14 trail0.3%@0.5% |
| AVAX | 309 | 34.0 | 0.81 | -17.8 | 38.8 | | 366 | 33.1 | 0.63 | -37.9 | 44.0 | -0.182 | -0.9 | ATR0.75×p21 trail0.3%@imm |
| **ALGO** | 350 | 36.3 | 0.73 | -28.0 | 43.7 | | 455 | 37.8 | **0.78** | -25.0 | 43.8 | **+0.045** | +1.5 | ATR0.50×p21 trail0.3%@imm |
| BNB  | 1002 | 30.0 | 0.46 | -145.0 | 48.3 | | 945 | 32.3 | 0.38 | -183.5 | 55.5 | -0.077 | +2.2 | ATR2.00×p7 trail0.5%@imm |
| BTC  | 348 | 33.3 | 0.70 | -30.7 | 41.7 | | 321 | 36.4 | 0.44 | -72.7 | 56.5 | -0.258 | +3.1 | ATR1.50×p14 |

### Portfolio Summary

| Metric | Baseline (fixed 0.3% SL) | Best ATR/Trail config | Delta |
|--------|--------------------------|----------------------|-------|
| Avg WR% | 33.8% | 36.8% | **+3.0pp** |
| Avg PF | 0.701 | 0.638 | **−0.063** |
| Avg breakeven WR | 42.7% | 48.6% | **+5.9pp** |
| PF improved | — | 3/18 | — |
| WR > 44% (≥10t) | 0/18 | 0/18 | — |

## Conclusions

### Win rate rises but profit factor falls: the trailing stop paradox

ATR/trailing stops increase average WR by +3.0pp (33.8% → 36.8%) but reduce average PF by −0.063 (0.701 → 0.638). **Only 3/18 coins improve PF** (DASH, DOGE, ALGO). This is the opposite of the expected outcome.

The mechanism: the most-selected trailing configurations use an **immediate activation, 0.3% trail distance**. This creates a de facto take-profit at ~0.3-0.6% gain:
1. Trade enters, price rises 0.3%
2. Trailing stop activates immediately at 0% activation
3. Trail stop = peak × (1 − 0.003) is very tight
4. First reversal hits the trail → "win" at 0.2-0.4%
5. The trade would have continued rising, but was cut short

Result: more wins (higher WR) but average win is smaller. Meanwhile the wider ATR-based stop ($ATR × 1.5-2.0) means stop-losses exit at 0.5-1.5% loss (much larger than the fixed 0.3% SL). The avg_loss grows more than avg_win, which is why PF drops.

### Breakeven WR increases to 48.6% — making the problem worse

The critical metric is breakeven WR = `avg_loss / (avg_win + avg_loss)`. Fixed SL=0.3% produces breakeven WR ≈ 42.7% on average. ATR/trailing configs raise this to 48.6%.

This means the stop structure improvements make the strategy **harder** to profit from, not easier. Wider ATR stops increase avg_loss proportionally more than trailing stops increase avg_win (because the 0.3% trail distance is small relative to the 1.5-2.0× ATR stop widening).

### DASH is the exception (PF +0.085, WR +8.5pp)

DASH's mean_reversion strategy benefits from ATR0.75×p14 with immediate trailing at 0.3%. DASH mean-reversion trades naturally revert quickly, so the 0.3% trail captures the bulk of the reversion move before the signal exit fires. This is structurally different from VWAP-reversion (used by most coins) which holds through larger swings.

### BTC catastrophic with ATR stops (PF −0.258)

BTC's best config uses no trailing (just ATR1.50×p14) but PF drops from 0.70 to 0.44. BTC's OHLCV structure means ATR(14)×1.5 ≈ 0.45-0.7% stop — twice the current 0.3% SL. With a 0.45% stop and ~0.6% avg_win, the R:R is barely above 1:1 but WR is only 36.4%. PF is destroyed.

### Why trailing stops fail here

Trailing stops improve results when:
1. avg_win is much larger than avg_loss (e.g., trend-following with 2:1+ R:R)
2. Wins are captured early and losers are cut

For COINCLAW mean-reversion:
1. avg_win ≈ 0.5-0.8% (price reverts to mean)
2. avg_loss ≈ 0.3% (fixed SL, larger ATR SL)
3. Trailing at 0.3% from peak turns a potential 0.7% win into a 0.3-0.4% win
4. Net effect: smaller wins, same or larger losses → worse PF

Trailing stops are better suited to trend-following strategies where wins can be multiples of losses.

## Decision

**NEGATIVE** — ATR-based and trailing stops do not improve COINCLAW performance. Only 3/18 coins improve PF. The average breakeven WR increases from 42.7% to 48.6%, making the strategy harder to profit from. The fundamental issue is that the optimal trailing distance for mean-reversion is the SMA/z-score signal exit (already implemented in COINCLAW), not a fixed percentage trail. No COINCLAW changes.

Note: The one case where trailing stops might genuinely help — DASH mean_reversion — already uses a different strategy (OuMeanRev) that manages its exits via OU half-life. The generic trailing stop is not needed there either.

## Files

| File | Description |
|------|-------------|
| `run26_results.json` | Per-coin baseline vs best ATR/trail config on OOS |
| `run26_1_atr_stops.py` | Original Python stub (full-data eval) |
| `RUN26.md` | This file |

Source: `tools/src/run26.rs`
