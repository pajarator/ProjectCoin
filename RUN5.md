# RUN5 — Long+Short Directional Trading System

## Date: 2026-03-14

## Overview

RUN5 adds short strategies to profit during the 22.8% idle time identified in RUN4.6. When breadth is high (market-wide dump), overbought coins get dragged down — a prime shorting opportunity. The system now trades directionally: long when the market is healthy, short when the market dumps, idle in between.

**Result:** P&L nearly doubled (+5.9% to +9.7%), idle time cut from 40% to 14%.

---

## RUN5.1 — Walk-Forward Validation of Breadth Filter

**Script:** `run5_1_walk_forward_breadth.py` | **Results:** `run5_1_results.json`

### Objective
Validate that BREADTH_MAX=20% doesn't overfit. Same walk-forward methodology as RUN4.3 (train 2mo, test 1mo, 3 windows) but with breadth filter applied.

### Method
- **Train:** Find best strategy + breadth threshold per coin (test thresholds: OFF, 50%, 40%, 30%, 25%, 20%, 15%)
- **Test:** Run out-of-sample with fixed BREADTH_MAX=20%
- Compare to no-breadth baseline

### Results

| Metric | Value |
|--------|-------|
| Avg Train PF | 4.09 |
| Avg Test PF (with breadth) | **2.30** |
| Avg Test PF (no breadth) | 1.72 |
| Degradation (train → test) | 43.8% |
| Breadth filter OOS impact | **+0.58 PF** |
| Verdict | HIGH overfitting risk |

### Consistency per Coin
| Status | Coins | Count |
|--------|-------|-------|
| OK (≥67% windows profitable) | DASH, UNI, ADA, DOGE, ALGO | 5/18 |
| WEAK (<67% windows profitable) | remaining 13 coins | 13/18 |

### Interpretation
The breadth filter has HIGH train-to-test degradation (43.8%), but it still adds +0.58 PF on out-of-sample data vs no filter. The filter genuinely helps — it's just that the in-sample improvement overstates the benefit. The 20% threshold is usable but not a magic bullet.

---

## RUN5.2 — Short Strategy Backtesting

**Script:** `run5_2_short_strategies.py` | **Results:** `run5_2_results.json`

### Objective
Backtest 4 short strategies that activate when breadth >= 50% (market dump).

### Short Strategies

| Strategy | Entry Condition |
|----------|----------------|
| `short_vwap_rev` | z > +1.5 AND price > SMA20 AND vol > vol_ma x 1.2 |
| `short_bb_bounce` | price >= BB_hi x 0.98 AND vol > vol_ma x 1.3 |
| `short_mean_rev` | z > +1.5 |
| `short_adr_rev` | price >= ADR_hi - range x 0.25 |

**Entry guard:** Skip if price already below SMA20 or z < -0.5 (already oversold)

**Exit conditions (mirrored from longs):**
- Stop loss: price rises 0.5% above entry
- Take profit (after min_hold): price drops below SMA20, or z drops below -0.5

### Test Matrix
4 strategies x 18 coins x 3 breadth thresholds (40%, 50%, 60%) x 4 param sets = 864 combinations

### Parameter Sets

| Name | z_threshold | bb_margin | vol_mult | adr_pct | exit_z |
|------|-------------|-----------|----------|---------|--------|
| Default | 1.5 | 0.98 | 1.2 | 0.25 | -0.5 |
| Aggressive | 1.2 | 0.99 | 1.0 | 0.30 | -0.3 |
| Conservative | 2.0 | 0.97 | 1.5 | 0.20 | -0.7 |
| Balanced | 1.8 | 0.98 | 1.3 | 0.25 | -0.5 |

### Top Results (by composite score PF x sqrt(WR))

| Params | Breadth>= | Strategy | Avg PF | Avg WR | Trades |
|--------|-----------|----------|--------|--------|--------|
| Aggressive | 60% | short_adr_rev | 2.88 | 64.2% | 344 |
| Aggressive | 60% | short_bb_bounce | 2.79 | 63.7% | 429 |
| Default | 60% | short_bb_bounce | 2.81 | 62.2% | 410 |
| Default | 50% | short_vwap_rev | 2.64 | 58.8% | 142 |

### Optimal Short Strategy per Coin

| Coin | Short Strategy | PF | WR | Trades |
|------|---------------|-----|-----|--------|
| DASH | short_mean_rev | 7.33 | 48.1% | 27 |
| UNI | short_adr_rev | 2.82 | 58.3% | 12 |
| NEAR | short_adr_rev | 3.38 | 53.6% | 28 |
| ADA | short_bb_bounce | 6.46 | 88.9% | 9 |
| LTC | short_mean_rev | 9.12 | 85.7% | 7 |
| SHIB | short_vwap_rev | 3.64 | 80.0% | 5 |
| LINK | short_bb_bounce | 4.60 | 83.3% | 6 |
| ETH | short_adr_rev | 3.11 | 80.0% | 5 |
| DOT | short_vwap_rev | 11.61 | 75.0% | 4 |
| XRP | short_bb_bounce | 2.69 | 66.7% | 24 |
| ATOM | short_adr_rev | 2.92 | 56.5% | 23 |
| SOL | short_adr_rev | 4.02 | 81.8% | 11 |
| DOGE | short_bb_bounce | 1.55 | 53.3% | 15 |
| XLM | short_mean_rev | 15.94 | 90.9% | 11 |
| AVAX | short_bb_bounce | 7.70 | 86.7% | 15 |
| ALGO | short_adr_rev | 14.48 | 93.8% | 16 |
| BNB | short_vwap_rev | 5.32 | 87.5% | 8 |
| BTC | short_adr_rev | 4.54 | 88.9% | 9 |

### Summary
- **18/18 coins** have viable short strategies (PF >= 1.0)
- Avg PF across best per coin: **6.18**
- Avg WR across best per coin: **75.5%**
- Best breadth threshold: **60%** (slightly better PF/WR than 40% or 50%)

---

## RUN5.3 — Combined Long+Short Backtest

**Script:** `run5_3_combined.py` | **Results:** `run5_3_results.json`

### Objective
Full directional system — long when breadth low, short when breadth high, idle in between.

### Directional Mode
```
breadth <= 20%  → LONG mode  (check long entries)
20% < breadth < 50%  → IDLE  (no new entries, exits still active)
breadth >= 50%  → SHORT mode (check short entries)
```

Single position per coin at a time. State machine: `position = None | 'long' | 'short'`.

### Comparison

| Mode | Avg WR | Avg PF | Trades | Avg MaxDD | Avg P&L |
|------|--------|--------|--------|-----------|---------|
| Long-only (v5) | 59.4% | 2.33 | 1,394 | 1.5% | +5.9% |
| Short-only | 67.4% | 4.35 | 376 | 0.7% | +3.5% |
| **Combined (v6)** | **61.3%** | **2.28** | **1,769** | **1.5%** | **+9.7%** |

### Time Allocation
| Mode | % of Time |
|------|-----------|
| LONG | 60.4% |
| IDLE | 14.4% |
| SHORT | 25.2% |

vs long-only: idle was 39.6%, now 14.4% (-25.2 percentage points).

### Per-Coin Details (Combined Mode)

| Coin | Long Strat | Short Strat | WR | PF | L trades | S trades | MaxDD | P&L |
|------|-----------|-------------|-----|-----|----------|----------|-------|------|
| DASH | vwap_reversion | short_mean_rev | 39.9% | 2.61 | 97 | 66 | 2.0% | +47.5% |
| UNI | vwap_reversion | short_adr_rev | 51.9% | 2.63 | 37 | 15 | 1.3% | +10.6% |
| NEAR | vwap_reversion | short_adr_rev | 50.0% | 2.69 | 24 | 38 | 1.6% | +13.8% |
| ADA | vwap_reversion | short_bb_bounce | 60.9% | 1.24 | 10 | 13 | 1.1% | +0.5% |
| LTC | vwap_reversion | short_mean_rev | 77.3% | 4.56 | 32 | 12 | 0.8% | +9.3% |
| SHIB | vwap_reversion | short_vwap_rev | 51.7% | 1.44 | 22 | 7 | 1.7% | +1.5% |
| LINK | vwap_reversion | short_bb_bounce | 80.8% | 4.50 | 13 | 13 | 0.5% | +4.5% |
| ETH | vwap_reversion | short_adr_rev | 76.9% | 3.57 | 14 | 12 | 0.5% | +3.9% |
| DOT | vwap_reversion | short_vwap_rev | 65.2% | 4.01 | 19 | 4 | 0.7% | +6.2% |
| XRP | vwap_reversion | short_bb_bounce | 62.5% | 1.41 | 21 | 27 | 0.8% | +1.8% |
| ATOM | vwap_reversion | short_adr_rev | 45.0% | 1.33 | 53 | 58 | 4.3% | +5.1% |
| SOL | vwap_reversion | short_adr_rev | 70.8% | 1.80 | 22 | 26 | 1.0% | +2.8% |
| DOGE | bb_bounce | short_bb_bounce | 61.7% | 1.51 | 171 | 12 | 2.2% | +9.3% |
| XLM | dual_rsi | short_mean_rev | 59.4% | 1.77 | 167 | 8 | 1.9% | +14.5% |
| AVAX | adr_reversal | short_bb_bounce | 62.9% | 1.84 | 216 | 21 | 1.5% | +20.2% |
| ALGO | adr_reversal | short_adr_rev | 58.6% | 1.64 | 245 | 16 | 1.6% | +18.6% |
| BNB | vwap_reversion | short_vwap_rev | 60.0% | 1.17 | 27 | 8 | 2.1% | +0.6% |
| BTC | bb_bounce | short_adr_rev | 68.6% | 1.26 | 204 | 19 | 1.6% | +4.5% |

All 18 coins are profitable in combined mode. Top earners: DASH (+47.5%), AVAX (+20.2%), ALGO (+18.6%).

---

## RUN5.4 — trader.py Update (v6)

**Modified:** `trader.py`

### Changes
1. **`SHORT_BREADTH_MIN = 0.50`** — short entries only when breadth >= 50%
2. **`short_pref` field** added to each coin in COINS list with RUN5.2 optimal assignments
3. **`short_entry()`** method — mirrors `entry()` with overbought conditions (z > +1.5, price > SMA20, etc.)
4. **`exit()`** updated — handles both long and short PnL calculations
5. **`buy()` renamed to `open_position(direction)`** — stores `dir` in position dict
6. **`sell()` renamed to `close_position()`** — computes PnL based on direction
7. **`effective_bal()`** updated — inverts unrealized PnL for short positions
8. **Directional mode** in main loop:
   - `breadth <= 20%` → LONG mode (check long entries)
   - `20% < breadth < 50%` → IDLE (no new entries)
   - `breadth >= 50%` → SHORT mode (check short entries)
9. **Display updates:**
   - POS column: `LONG` / `SHORT` / `CASH`
   - SHORT positions colored magenta (curses color_pair 6)
   - Breadth line shows: `MODE: LONG` / `MODE: IDLE` / `MODE: SHORT`
   - Header: COINCLAW v6
10. **State persistence** — position `dir` field persisted in `trading_state.json`

---

## v6 Configuration

```python
# Parameters
STOP_LOSS = 0.005        # 0.5%
MIN_HOLD_CANDLES = 2     # 30min
RISK = 0.10              # 10%
LEVERAGE = 5             # 5x

# Directional thresholds
BREADTH_MAX = 0.20       # Long entries only when breadth <= 20%
SHORT_BREADTH_MIN = 0.50 # Short entries only when breadth >= 50%
```

### Short Strategy Assignments
| Coin | Long Strategy | Short Strategy |
|------|--------------|----------------|
| DASH | vwap_rev | short_mean_rev |
| UNI | vwap_rev | short_adr_rev |
| NEAR | vwap_rev | short_adr_rev |
| ADA | vwap_rev | short_bb_bounce |
| LTC | vwap_rev | short_mean_rev |
| SHIB | vwap_rev | short_vwap_rev |
| LINK | vwap_rev | short_bb_bounce |
| ETH | vwap_rev | short_adr_rev |
| DOT | vwap_rev | short_vwap_rev |
| XRP | vwap_rev | short_bb_bounce |
| ATOM | vwap_rev | short_adr_rev |
| SOL | vwap_rev | short_adr_rev |
| DOGE | bb_bounce | short_bb_bounce |
| XLM | dual_rsi | short_mean_rev |
| AVAX | adr_rev | short_bb_bounce |
| ALGO | adr_rev | short_adr_rev |
| BNB | vwap_rev | short_vwap_rev |
| BTC | bb_bounce | short_adr_rev |

---

## Evolution Summary (RUN1 → RUN5)

| Version | Run | PF | WR | Key Change |
|---------|-----|-----|-----|------------|
| v1-v2 | RUN1-2 | ~1.0 | 67-87% | Initial strategies, single coin |
| v3 | RUN3 | 1.18 | 67% | Multi-coin, 20 coins |
| v4 | RUN4.1 | 1.64 | 50% | Tighter SL (0.5%), faster exits |
| v5 | RUN4.6 | 2.28 | 60% | Breadth filter, skip market dumps |
| **v6** | **RUN5** | **2.28** | **61%** | **+shorts, P&L +5.9%→+9.7%** |

---

## Caveats

1. **Breadth filter overfitting (RUN5.1):** 43.8% degradation train→test. The 20% threshold helps on OOS but less than in-sample suggests. Monitor live performance.
2. **Short trade count:** Only 376 short trades across all coins (vs 1,394 long). Small sample — short strategy edge is less statistically robust.
3. **Short strategies not walk-forward validated:** RUN5.2 was full-sample. A walk-forward on shorts would strengthen confidence but sample size may be too small.
4. **PF slightly decreased in combined mode:** 2.33 → 2.28. The shorts add volume at slightly lower edge, but total returns increase because idle capital is now productive.

---

## Files

| File | Purpose |
|------|---------|
| `run5_1_walk_forward_breadth.py` | Breadth filter walk-forward validation |
| `run5_2_short_strategies.py` | Short strategy backtesting |
| `run5_3_combined.py` | Combined long+short backtest |
| `trader.py` | Live paper trading (v6, directional) |
| `run5_1_results.json` | RUN5.1 results |
| `run5_2_results.json` | RUN5.2 results |
| `run5_3_results.json` | RUN5.3 results |
