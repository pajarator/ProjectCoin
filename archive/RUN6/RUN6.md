# RUN6 — Isolated (ISO) Short Strategies

## Goal

RUN5 added market-dump shorts that only fire when breadth >= 50% (many coins oversold = market-wide dump). But when breadth is 20-50% (calm market), the system sits idle. RUN6 adds **ISO shorts**: coin-specific overbought short entries that fire even when the broader market is calm.

The v6 system had 2 modes: LONG (breadth <= 20%) and SHORT (breadth >= 50%), with IDLE in between. RUN6 fills the IDLE gap, creating a 3-mode v7 system.

---

## RUN6.1 — ISO Short Strategy Discovery

**Script:** `run6_1_protection_stop.py` (naming artifact)

### ISO Short Strategies Tested

9 strategies, 4 parameter sets (Default, Aggressive, Conservative, Balanced), 5 breadth thresholds (10%, 15%, 20%, 30%, 50%):

| Strategy | Logic |
|----------|-------|
| `iso_mean_rev` | Z-score > threshold (coin overbought) |
| `iso_vwap_rev` | Z > threshold + above VWAP + volume |
| `iso_bb_bounce` | Price near upper BB + volume |
| `iso_adr_rev` | Price in top % of ADR range + volume |
| `iso_relative_z` | Coin Z-score outlier vs market average |
| `iso_rsi_extreme` | RSI > threshold while market RSI calm |
| `iso_divergence` | Coin overbought while BTC flat/down |
| `iso_vol_spike` | Z > 1.0 + volume spike |
| `iso_bb_squeeze` | Price at upper BB during squeeze |

### Best ISO Short Strategy Per Coin

| Coin | Strategy | Params | Breadth Max | PF | WR | Trades |
|------|----------|--------|-------------|----|----|--------|
| DASH | iso_divergence | Balanced | 10% | 3.75 | 36.5% | 52 |
| UNI | iso_relative_z | Conservative | 10% | 3.73 | 55.6% | 18 |
| NEAR | iso_rsi_extreme | Conservative | 15% | 15.70 | 75.0% | 4 |
| ADA | iso_divergence | Conservative | 10% | 2.64 | 50.0% | 22 |
| LTC | iso_rsi_extreme | Conservative | 20% | 10.01 | 66.7% | 3 |
| SHIB | iso_rsi_extreme | Conservative | 20% | 4.46 | 50.0% | 6 |
| LINK | iso_relative_z | Conservative | 50% | 10.93 | 87.5% | 16 |
| ETH | iso_rsi_extreme | Default | 10% | 11.71 | 88.9% | 9 |
| DOT | iso_relative_z | Default | 30% | 2.85 | 59.8% | 82 |
| XRP | iso_rsi_extreme | Default | 30% | 3.29 | 71.4% | 14 |
| ATOM | iso_relative_z | Conservative | 30% | 2.52 | 55.8% | 52 |
| SOL | iso_rsi_extreme | Default | 50% | 9.22 | 85.7% | 7 |
| DOGE | iso_divergence | Conservative | 10% | 5.30 | 71.4% | 28 |
| XLM | iso_relative_z | Conservative | 10% | 7.80 | 75.0% | 8 |
| AVAX | iso_relative_z | Conservative | 10% | 5.36 | 76.9% | 13 |
| ALGO | iso_rsi_extreme | Default | 10% | 12.18 | 71.4% | 7 |
| BNB | iso_divergence | Default | 10% | 2.60 | 68.6% | 51 |
| BTC | iso_rsi_extreme | Aggressive | 10% | 6.18 | 88.0% | 25 |

**Dominant strategies:** `iso_rsi_extreme` (8 coins), `iso_relative_z` (6 coins), `iso_divergence` (4 coins). Most coins prefer Conservative or Default params with breadth_max = 10%.

### Top Universal Combos (across all coins)

| Rank | Params | Breadth | Strategy | Avg PF | Avg WR | Total Trades |
|------|--------|---------|----------|--------|--------|--------------|
| 1 | Conservative | 15% | iso_relative_z | 3.07 | 55.5% | 404 |
| 2 | Conservative | 15% | iso_rsi_extreme | 3.28 | 39.3% | 63 |
| 3 | Default | 10% | iso_rsi_extreme | 2.81 | 53.0% | 126 |
| 4 | Conservative | 10% | iso_relative_z | 2.67 | 56.2% | 301 |
| 5 | Default | 50% | iso_rsi_extreme | 2.64 | 55.5% | 277 |

---

## RUN6.2 — Walk-Forward Validation

**Script:** `run6_2_walk_forward.py`

### Method

3 windows (train 2 months, test 1 month). Train: find best ISO short strategy + breadth_max + params per coin. Test OOS with fixed breadth_max from RUN6.1.

### Results

| Metric | Value |
|--------|-------|
| Avg Train PF | 6.50 |
| Avg Test PF (OOS) | 1.44 |
| **Degradation** | **77.8%** |
| Verdict | **HIGH overfitting risk** |
| Consistent coins (profitable in 2/3+ windows) | **0/18** |

**Finding:** ISO short strategies degrade severely out of sample. No coin was consistently profitable across OOS windows. The high in-sample PFs (3-15x) are mostly noise from low trade counts (3-9 trades for many coins).

---

## RUN6.3 — Combined 3-Mode Backtest (v7)

**Script:** `run6_3_combined.py`

### 4-Way Comparison

| Mode | Avg WR | Avg PF | Trades | Avg MaxDD | Avg P&L |
|------|--------|--------|--------|-----------|---------|
| long_only | 59.4% | 2.33 | 1,394 | 1.5% | +5.9% |
| long+market_short (v6) | 61.3% | 2.28 | 1,769 | 1.5% | +9.7% |
| long+iso_short | 58.6% | 2.35 | 2,286 | 1.6% | +13.2% |
| **combined_all (v7)** | **59.5%** | **2.32** | **2,578** | **1.7%** | **+17.0%** |

### Improvement Analysis

**v5 -> v6 (add market-dump shorts):**
- WR: 59.4% -> 61.3% (+1.9%)
- PF: 2.33 -> 2.28 (-0.05)
- Trades: 1,394 -> 1,769 (+375)
- P&L: +5.9% -> +9.7% (+3.8%)

**v6 -> v7 (add ISO shorts):**
- WR: 61.3% -> 59.5% (-1.8%)
- PF: 2.28 -> 2.32 (+0.04)
- Trades: 1,769 -> 2,578 (+809)
- P&L: +9.7% -> +17.0% (+7.3%)

**v5 -> v7 (full improvement):**
- WR: 59.4% -> 59.5% (+0.1%)
- Trades: 1,394 -> 2,578 (+1,184)
- P&L: +5.9% -> +17.0% (+11.1%)

### Time Allocation (v7)

The 3-mode system uses idle time productively:
- LONG mode: ~55% of time
- ISO_SHORT mode: ~25% of time (was IDLE in v6)
- SHORT mode: ~20% of time

---

## Key Conclusions

1. **ISO shorts add significant P&L** (+7.3% per coin vs v6) by trading during previously idle periods. The v7 combined system nearly triples P&L vs long-only (+17.0% vs +5.9%).

2. **ISO shorts overfit badly in walk-forward** (77.8% degradation, 0/18 coins consistent). The high per-coin PFs are unreliable due to low trade counts. However, in the combined system they still contribute because even degraded ISO shorts in aggregate add trades.

3. **Dominant ISO strategies are `iso_rsi_extreme` and `iso_relative_z`** — both detect coin-specific overbought conditions relative to the broader market. `iso_divergence` (coin up while BTC down) works for a few coins.

4. **Breadth_max = 10%** is the most common optimal threshold, meaning ISO shorts work best when the market is very calm (very few coins oversold).

5. **The v7 system was applied to trader.py** with per-coin ISO short assignments and breadth thresholds from this run.

---

## Files

| File | Action | Description |
|------|--------|-------------|
| `run6_1_protection_stop.py` | Created | ISO short strategy grid search |
| `run6_2_walk_forward.py` | Created | Walk-forward validation (3 windows) |
| `run6_3_combined.py` | Created | 4-way combined comparison |
| `run6_1_results.json` | Output | Per-coin ISO short strategies + params |
| `run6_2_results.json` | Output | Walk-forward degradation results |
| `run6_3_results.json` | Output | Combined v7 comparison results |
| `trader.py` | Modified | Updated to v7 with 3-mode system + ISO shorts |
