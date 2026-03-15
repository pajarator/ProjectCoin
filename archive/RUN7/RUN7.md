# RUN7 — Unified Stop Loss Optimization

## Goal

The original plan was to add a "protection stop" (trailing/breakeven exit) that activates after a position reaches a profit threshold. During analysis, we realized this was just bolting a second stop loss on top of the existing one. The real question: **is the current -0.5% fixed SL optimal, and should it trail?**

Redesigned RUN7 to optimize the stop loss as a single unified system:
- `initial_sl`: the fixed stop loss level
- `trail_mode`: none (fixed), breakeven (SL moves to entry after profit), trail (SL follows peak)
- `trail_activation`: min profit before SL starts moving
- `trail_distance`: how far below peak the trailing SL sits

The current hardcoded `sl=0.5%, none` is just one point in this grid.

---

## RUN7.1 — Grid Search: Optimal Stop Loss Parameters

**Script:** `run7_1_protection_stop.py`

### Parameter Grid

| Parameter | Values | Description |
|-----------|--------|-------------|
| `initial_sl` | [0.003, 0.005, 0.007, 0.010] | Initial stop loss (0.3%-1.0%) |
| `trail_mode` | [none, breakeven, trail] | Does SL move after profit? |
| `trail_activation` | [0.001, 0.002, 0.003, 0.005] | Min profit to activate trailing |
| `trail_distance` | [0.001, 0.002, 0.003, 0.004] | Distance below peak (trail only) |

**Grid sizes:** none: 4, breakeven: 16, trail: 64 = **84 combos x 18 coins = 1,512 backtests**

### Shadow Tracking (Counterfactual Analysis)

For every trail/breakeven exit, a "shadow" position continues tracking what would have happened with just the initial SL + signal exits:
- **Save:** shadow hits SL -> trailing saved us from a loss
- **Premature:** shadow hits signal exit at bigger profit -> trailing cut a winner short
- **Timeout:** 50 candles without resolution -> excluded from counts
- `net_impact = saves - prematures` (positive = trailing helps)

### Stop Loss Size Analysis (none mode, no trailing)

| SL | WR | PF | Avg P&L | Trades | Note |
|----|----|----|---------|--------|------|
| 0.3% | 49.6% | 2.62 | +18.6% | 2,686 | **Best PF and P&L** |
| 0.5% | 59.5% | 2.32 | +17.0% | 2,578 | Current baseline |
| 0.7% | 65.8% | 2.08 | +15.6% | 2,499 | |
| 1.0% | 71.2% | 1.91 | +13.0% | 2,429 | Highest WR, worst P&L |

**Finding:** Clear inverse relationship between SL tightness and WR. Tighter SL = more stopped out (lower WR) but smaller losses per stop = higher PF and P&L. The 0.3% SL makes +1.6% more P&L than current 0.5%.

### Best by Mode

| Mode | Best Params | Avg WR | Avg PF | Avg P&L | Net Impact |
|------|------------|--------|--------|---------|------------|
| **none** | sl=0.3% | 49.6% | **2.62** | **+18.6%** | n/a |
| breakeven | sl=0.3%, act=0.5% | 42.6% | 2.43 | +15.2% | +5.8 |
| trail | sl=0.3%, act=0.5%, dist=0.1% | 56.2% | 2.31 | +11.9% | -9.6 |

**Finding:** Trailing and breakeven stops **do not help**. Trail mode has negative net impact (-9.6 saves vs prematures), meaning it cuts winners short more often than it saves from losses. Best overall is the simplest: tighter fixed SL with no trailing.

### Best Per Coin

| Coin | Best Mode | SL | Act | Dist | WR | PF | P&L | vs Base |
|------|-----------|-----|-----|------|----|----|-----|---------|
| DASH | trail | 0.3% | 0.5% | 0.3% | 48% | 3.33 | +61.7% | -12.5% |
| UNI | trail | 0.3% | 0.2% | 0.1% | 59% | 3.06 | +17.0% | -7.7% |
| NEAR | trail | 0.3% | 0.5% | 0.3% | 62% | 4.32 | +13.4% | -9.6% |
| ADA | trail | 0.3% | 0.3% | 0.1% | 56% | 1.92 | +5.6% | +1.3% |
| LTC | none | 0.5% | - | - | 70% | 3.56 | +11.4% | +0.0% |
| SHIB | none | 0.7% | - | - | 63% | 1.82 | +4.4% | -0.3% |
| LINK | none | 1.0% | - | - | 85% | 4.22 | +12.1% | +1.2% |
| ETH | none | 0.5% | - | - | 80% | 4.73 | +6.7% | +0.0% |
| DOT | none | 0.3% | - | - | 52% | 4.23 | +24.8% | +4.5% |
| XRP | none | 1.0% | - | - | 80% | 1.80 | +4.5% | +1.6% |
| ATOM | trail | 0.3% | 0.3% | 0.2% | 53% | 1.92 | +13.6% | -3.3% |
| SOL | none | 0.7% | - | - | 80% | 2.40 | +5.0% | +0.3% |
| DOGE | none | 0.3% | - | - | 51% | 1.95 | +19.6% | +0.7% |
| XLM | trail | 0.3% | 0.2% | 0.1% | 54% | 2.08 | +14.8% | -11.9% |
| AVAX | none | 0.3% | - | - | 52% | 2.13 | +28.5% | +3.8% |
| ALGO | none | 0.7% | - | - | 67% | 1.65 | +21.3% | +0.5% |
| BNB | none | 0.3% | - | - | 53% | 2.02 | +7.4% | +1.2% |
| BTC | none | 0.3% | - | - | 61% | 1.54 | +7.7% | +2.8% |

**12/18 coins prefer `none` mode, 6 prefer `trail`.** Per-coin trail results often show negative "vs base" — trailing hurts more than it helps on individual coins too.

### Best Universal Params

**Mode: none, SL: 0.3%** — no trailing, just tighter fixed stop loss.

---

## RUN7.2 — Walk-Forward Validation

**Script:** `run7_2_walk_forward.py`

### Method

3 windows (train 2 months, test 1 month):
- W1: Oct 15 - Dec 14 -> Dec 15 - Jan 14
- W2: Nov 15 - Jan 14 -> Jan 15 - Feb 14
- W3: Dec 15 - Feb 14 -> Feb 15 - Mar 10

Train: search all 84 SL combos, pick best per coin.
Test OOS: apply per-coin best, universal best (sl=0.3%, none), and baseline (sl=0.5%, none).

### Degradation Analysis

| Config | Train PF | Test PF (OOS) | Degradation |
|--------|----------|---------------|-------------|
| Per-coin optimized | 5.79 | 2.53 | **56.2%** |
| Universal (sl=0.3%, none) | n/a | **2.59** | n/a |
| Baseline (sl=0.5%, none) | n/a | 2.51 | n/a |

**Finding:** Per-coin optimization degrades 56% OOS — heavy overfitting, not viable. Universal params (sl=0.3%) produce OOS PF of 2.59 vs baseline 2.51. Marginal but consistent improvement.

### Recommendation

**Universal params preferred.** Per-coin SL tuning overfits. The universal sl=0.3% survives walk-forward and slightly beats baseline.

### Low Confidence Flags

- ETH: 1/3 windows with < 3 trades

---

## RUN7.3 — Combined Backtest Comparison

**Script:** `run7_3_combined.py`

### Portfolio-Level Comparison

| Mode | Avg WR | Avg PF | Trades | Avg MaxDD | Avg P&L | Total P&L |
|------|--------|--------|--------|-----------|---------|-----------|
| v7 (sl=0.5%, none) | 59.5% | 2.32 | 2,578 | 1.7% | +17.0% | +306.7% |
| v8 (sl=0.3%, none) | 49.6% | **2.62** | 2,686 | **1.1%** | **+18.6%** | **+335.4%** |

**Delta:** WR -9.9%, PF +0.30, P&L +1.6%, MaxDD -0.6%, Total P&L +28.7%

### Per-Coin Breakdown

| Coin | v7 WR | v8 WR | v7 PF | v8 PF | v7 P&L | v8 P&L | Better? |
|------|-------|-------|-------|-------|--------|--------|---------|
| DASH | 36.7 | 31.0 | 2.35 | 3.29 | +74.3% | +88.4% | YES |
| UNI | 45.7 | 41.1 | 2.18 | 3.12 | +24.8% | +31.1% | YES |
| NEAR | 52.7 | 48.1 | 3.39 | 4.56 | +23.0% | +23.6% | YES |
| ADA | 51.0 | 41.7 | 1.36 | 1.63 | +4.3% | +5.4% | YES |
| LTC | 69.6 | 55.4 | 3.56 | 3.56 | +11.4% | +10.0% | NO |
| SHIB | 51.2 | 40.9 | 1.87 | 1.90 | +4.6% | +3.5% | NO |
| LINK | 72.2 | 61.1 | 3.92 | 4.13 | +10.9% | +10.0% | NO |
| ETH | 80.0 | 65.7 | 4.73 | 3.74 | +6.7% | +5.0% | NO |
| DOT | 58.2 | 52.1 | 2.96 | 4.23 | +20.3% | +24.8% | YES |
| XRP | 62.1 | 46.6 | 1.52 | 1.47 | +2.9% | +2.2% | NO |
| ATOM | 48.9 | 40.0 | 1.54 | 1.78 | +16.9% | +17.7% | YES |
| SOL | 73.1 | 59.3 | 2.31 | 2.30 | +4.7% | +4.4% | NO |
| DOGE | 61.0 | 50.8 | 1.71 | 1.95 | +18.9% | +19.6% | YES |
| XLM | 58.5 | 47.0 | 1.92 | 2.11 | +26.6% | +26.0% | NO |
| AVAX | 61.4 | 52.1 | 1.78 | 2.13 | +24.6% | +28.5% | YES |
| ALGO | 58.1 | 46.2 | 1.67 | 1.77 | +20.8% | +20.1% | NO |
| BNB | 62.1 | 53.0 | 1.67 | 2.02 | +6.2% | +7.4% | YES |
| BTC | 68.7 | 60.7 | 1.27 | 1.54 | +4.8% | +7.7% | YES |

**10/18 coins improved P&L, 8 got worse.** Coins that benefited most: DASH (+14.1%), UNI (+6.3%), DOT (+4.5%), AVAX (+3.9%). Coins hurt most: ETH (-1.7%), LTC (-1.4%), LINK (-0.9%).

### Exit Reason Distribution (v8)

| Reason | Count | % |
|--------|-------|---|
| SL | 1,406 | 52.4% |
| SMA | 1,276 | 47.6% |

With the tighter 0.3% SL, ~52% of exits are stop losses (vs ~40% with 0.5%). The tighter SL prevents deeper losses, resulting in higher PF despite more frequent stops.

---

## Key Conclusions

1. **Trailing/breakeven stops don't work for this system.** Shadow analysis shows they cut winners short more often than they save from losses (net impact: -9.6 for trail, +5.8 for breakeven but still underperforms no-trail).

2. **The optimal change is simply tightening the SL from 0.5% to 0.3%.** This is the simplest possible outcome — no new exit mechanism needed, just a parameter change.

3. **The WR vs P&L tradeoff:** 0.3% SL drops WR from 59.5% to 49.6% (below the 70% target from RUN1). However, PF improves from 2.32 to 2.62, P&L improves +1.6% per coin, and MaxDD drops from 1.7% to 1.1%. Mathematically more profitable despite "losing more often."

4. **Walk-forward confirms:** Universal sl=0.3% produces OOS PF 2.59 vs baseline 2.51. Not a dramatic improvement, but it holds up. Per-coin SL tuning overfits badly (56% degradation).

5. **Recommended v8 change for trader.py:** `STOP_LOSS = 0.003` (was 0.005). No trailing, no breakeven, no new exit logic needed.

---

## Files

| File | Action | Description |
|------|--------|-------------|
| `run7_1_protection_stop.py` | Created | Grid search: 84 SL combos x 18 coins |
| `run7_2_walk_forward.py` | Created | Walk-forward validation (3 windows) |
| `run7_3_combined.py` | Created | v7 vs v8 full comparison |
| `run7_1_results.json` | Output | Grid search results |
| `run7_2_results.json` | Output | Walk-forward results |
| `run7_3_results.json` | Output | Combined comparison results |
| `trader.py` | Pending | Change `STOP_LOSS = 0.003` (was 0.005) |
