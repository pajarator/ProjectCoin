# RUN8 — Take Profit Optimization

## Goal

The system currently has **no explicit take profit** — positions exit only via SL (-0.3%) or signal exits (SMA20 crossback / Z-score reversion) after MIN_HOLD=2 candles. This means profits are unbounded but depend entirely on when the signal fires. A trade can reach +0.8%, fail to trigger signal exit, reverse, and hit SL — giving back all unrealized gains.

**Question:** Does adding a fixed TP target improve performance?

---

## RUN8.1 — Grid Search: Optimal TP Parameters

**Script:** `run8_1_take_profit.py`

### TP Modes

| Mode | Logic |
|------|-------|
| `none` | Current behavior — signal exits only (baseline) |
| `tp_immediate` | Exit when pnl >= TP%, fires even during MIN_HOLD period |
| `tp_after_hold` | Exit when pnl >= TP%, only after MIN_HOLD candles |

### Parameter Grid

| Parameter | Values |
|-----------|--------|
| `tp_mode` | `none`, `tp_immediate`, `tp_after_hold` |
| `tp_target` | [0.3%, 0.5%, 0.7%, 1.0%, 1.5%, 2.0%] |

**Grid:** 1 (none) + 6 (immediate) + 6 (after_hold) = **13 combos x 18 coins = 234 backtests**

Hardcoded from RUN7: `STOP_LOSS = 0.003`, `trail_mode = 'none'`

### R:R Ratios (vs SL=0.3%)

| TP | R:R |
|----|-----|
| 0.3% | 1.0:1 |
| 0.5% | 1.7:1 |
| 0.7% | 2.3:1 |
| 1.0% | 3.3:1 |
| 1.5% | 5.0:1 |
| 2.0% | 6.7:1 |

### Shadow/Counterfactual Tracking

When TP fires, a shadow position continues tracking what baseline (no TP) would have done:

| Shadow Outcome | Meaning |
|----------------|---------|
| `TP_SAVE` | Shadow hits SL — TP locked profit that would have been lost |
| `TP_PARTIAL_SAVE` | Shadow signal exit at lower PnL than TP — TP did better |
| `TP_PREMATURE` | Shadow signal exit at higher PnL than TP — TP cut winner short |
| `TP_TIMEOUT` | 50 candles, unresolved — excluded |

### Results: All Combos

| Mode | TP% | R:R | Avg WR | Avg PF | Avg P&L | TP# | Save | Part | Prem | Net |
|------|-----|-----|--------|--------|---------|-----|------|------|------|-----|
| **none** | **-** | **-** | **51.1%** | **2.49** | **+10.5%** | **0** | **0** | **0** | **0** | **0** |
| tp_immediate | 0.3% | 1.0:1 | 56.7% | 1.25 | +0.7% | 787 | 121 | 13 | 652 | -518 |
| tp_immediate | 0.5% | 1.7:1 | 53.7% | 1.58 | +3.1% | 514 | 56 | 16 | 442 | -370 |
| tp_immediate | 0.7% | 2.3:1 | 52.4% | 1.80 | +5.0% | 340 | 36 | 4 | 300 | -260 |
| tp_immediate | 1.0% | 3.3:1 | 52.1% | 2.09 | +6.9% | 194 | 22 | 8 | 164 | -134 |
| tp_immediate | 1.5% | 5.0:1 | 51.5% | 2.25 | +8.3% | 88 | 12 | 1 | 75 | -62 |
| tp_immediate | 2.0% | 6.7:1 | 51.3% | 2.33 | +9.1% | 55 | 7 | 0 | 48 | -41 |
| tp_after_hold | 0.3% | 1.0:1 | 55.0% | 1.17 | +0.0% | 725 | 76 | 3 | 645 | -566 |
| tp_after_hold | 0.5% | 1.7:1 | 53.2% | 1.54 | +2.7% | 483 | 39 | 3 | 441 | -399 |
| tp_after_hold | 0.7% | 2.3:1 | 52.2% | 1.79 | +4.7% | 329 | 27 | 2 | 300 | -271 |
| tp_after_hold | 1.0% | 3.3:1 | 52.0% | 2.07 | +6.7% | 185 | 19 | 2 | 164 | -143 |
| tp_after_hold | 1.5% | 5.0:1 | 51.5% | 2.24 | +8.2% | 85 | 10 | 0 | 75 | -65 |
| tp_after_hold | 2.0% | 6.7:1 | 51.3% | 2.33 | +9.0% | 54 | 6 | 0 | 48 | -42 |

### Best by Mode

| Mode | Best TP | Avg WR | Avg PF | Avg P&L |
|------|---------|--------|--------|---------|
| **none (baseline)** | **-** | **51.1%** | **2.49** | **+10.5%** |
| tp_immediate | 2.0% | 51.3% | 2.33 | +9.1% |
| tp_after_hold | 2.0% | 51.3% | 2.33 | +9.0% |

**Best overall: none (no TP).** Even the loosest TP (2.0%, R:R 6.7:1) loses -1.4% P&L and -0.16 PF vs baseline.

### Best Per Coin

| Coin | Best Mode | TP | WR | PF | P&L | vs Base | Net Impact |
|------|-----------|------|----|----|-----|---------|------------|
| DASH | none | - | 34% | 3.60 | +54.1% | +0.0% | 0 |
| UNI | none | - | 44% | 3.46 | +11.6% | +0.0% | 0 |
| NEAR | none | - | 48% | 3.99 | +16.4% | +0.0% | 0 |
| ADA | none | - | 57% | 1.78 | +1.2% | +0.0% | 0 |
| LTC | none | - | 61% | 3.93 | +7.7% | +0.0% | 0 |
| SHIB | tp_immediate | 1.0% | 47% | 1.72 | +1.7% | +0.3% | -1 |
| LINK | none | - | 65% | 3.77 | +3.8% | +0.0% | 0 |
| ETH | none | - | 65% | 3.13 | +2.9% | +0.0% | 0 |
| DOT | none | - | 54% | 4.59 | +6.1% | +0.0% | 0 |
| XRP | none | - | 44% | 1.12 | +0.5% | +0.0% | 0 |
| ATOM | tp_immediate | 1.5% | 37% | 1.55 | +5.8% | +0.0% | -4 |
| SOL | none | - | 56% | 1.76 | +2.5% | +0.0% | 0 |
| DOGE | none | - | 53% | 1.91 | +12.5% | +0.0% | 0 |
| XLM | none | - | 47% | 1.79 | +12.7% | +0.0% | 0 |
| AVAX | none | - | 54% | 2.22 | +23.1% | +0.0% | 0 |
| ALGO | none | - | 46% | 1.69 | +17.4% | +0.0% | 0 |
| BNB | none | - | 51% | 1.48 | +1.3% | +0.0% | 0 |
| BTC | none | - | 60% | 1.52 | +7.2% | +0.0% | 0 |

**16/18 coins prefer no TP. Only SHIB (+0.3%) and ATOM (+0.0%) marginally favor TP, both with negative net_impact.**

### Early Stop Verdict

- Best TP helps **0/18 coins** vs baseline
- Average net_impact across all TP combos: **-239.2**
- **TP does NOT help. Stopped early — RUN8.2 and RUN8.3 not executed.**

---

## RUN8.2 / RUN8.3 — Not Executed

Per the early stop rule: TP hurts across all coins, PF degrades vs baseline, net_impact massively negative. Walk-forward validation and combined comparison were skipped.

---

## Key Conclusions

1. **Take profit does not improve this system.** Every TP target tested (0.3%-2.0%) degraded both PF and P&L compared to signal-only exits. The effect is monotonic: tighter TP = worse performance.

2. **Shadow analysis proves TP cuts winners short.** Across all TP combos, TP_PREMATURE (TP fired but signal exit would have been more profitable) outnumbers TP_SAVE (TP locked profit that would have been lost to SL) by ~6:1 to ~8:1. Net impact is massively negative at every TP level.

3. **Signal exits are already optimal profit-takers.** The SMA20 crossback and Z-score reversion signals naturally capture the fat right tail of winning trades. A fixed TP cap truncates this tail, turning occasional +3-5% winners into capped +0.3-2.0% exits.

4. **tp_immediate vs tp_after_hold makes almost no difference.** The two modes produce nearly identical results at every TP level. The MIN_HOLD period (2 candles = 30 min) is too short to meaningfully change outcomes.

5. **No changes to trader.py.** The system stays at COINCLAW v8: SL=0.3%, no TP, signal exits only.

---

## Files

| File | Action | Description |
|------|--------|-------------|
| `run8_1_take_profit.py` | Created | Grid search: 13 TP combos x 18 coins |
| `run8_1_results.json` | Output | Grid search results |
| `run8_2_walk_forward.py` | Created (not run) | Walk-forward validation script |
| `run8_3_combined.py` | Created (not run) | v8 vs v9 comparison script |
| `trader.py` | No change | TP does not help; stays at v8 |
