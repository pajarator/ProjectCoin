# RUN9: Scalp Overlay + Rust Live Trader (COINCLAW v9)

## Goal

Add a 1-minute scalp overlay to the existing 15m regime trading system and port the entire trader from Python (COINCLAW v8, 881 lines) to Rust with async I/O, ratatui TUI, and direct Binance REST API calls.

## Hypothesis

Short-duration scalp trades on 1m candles can generate additional alpha between regime trades, using tight SL/TP (0.1%/0.2%) and momentum/mean-reversion signals (volume spikes, stochastic crosses, BB squeeze breakouts).

## Method

### RUN 9.1 — Scalp Grid Search
- **Script:** `run9_1_scalp_grid.py` / `run9_1_rust/` (Rust port for speed)
- **Grid:** SL ∈ {0.10%, 0.15%, 0.20%, 0.25%} × TP ∈ {0.20%, 0.30%, 0.40%, 0.50%} × vol_mult ∈ {2.5, 3.0, 3.5} × rsi ∈ {15, 20, 25} × stoch ∈ {5, 10, 15} × bb_squeeze ∈ {0.4, 0.5, 0.6}
- **Data:** 5-month 15m+1m cached candles, 18 coins
- **Result:** Optimal scalp params: SL=0.1%, TP=0.2%, vol_mult=3.5, rsi=20, stoch=5, bb_squeeze=0.4

### RUN 9.2 — Walk-Forward Validation
- **Script:** `run9_2_walk_forward.py` / `run9_2_rust/`
- **Method:** 3-window walk-forward (train 2mo, test 1mo)
- **Result:** Scalp overlay is additive — net positive P&L contribution across windows

### RUN 9.3 — Combined Comparison
- **Script:** `run9_3_combined.py`
- **Result:** Regime+Scalp outperforms regime-only on aggregate P&L

### COINCLAW v9 — Rust Live Trader
- **Code:** `coinclaw/` (1,870 lines Rust across 9 modules)
- **Architecture:** tokio async runtime with two tasks:
  - Trading loop (5s cycle): fetch 15m candles (18 concurrent, every 60s), fetch 1m candles (18 concurrent, every 15s), compute indicators, market breadth, check exits/entries, save state
  - TUI loop (200ms tick): ratatui rendering with color-coded coins, regime, breadth, scrolling log
- **Features:**
  - Direct Binance REST API (reqwest+rustls, no CCXT dependency)
  - JSON state file compatible with Python `trading_state.json`
  - SIGINT graceful shutdown with state save
  - `--reset` flag to clear state
  - 4.5MB release binary

## Module Structure

```
coinclaw/src/
  main.rs          — tokio runtime, SIGINT, spawn tasks (232 lines)
  config.rs        — Constants, 18 coin configs, strategy mappings (109 lines)
  indicators.rs    — Rolling helpers, 15m/1m indicator computation (342 lines)
  strategies.rs    — Entry signals: long/short/ISO/scalp (272 lines)
  fetcher.rs       — Binance REST kline fetching (56 lines)
  state.rs         — SharedState, CoinState, Position, persistence (227 lines)
  engine.rs        — Exit/entry logic, position management, logging (289 lines)
  coordinator.rs   — Breadth, market mode, regime detection (115 lines)
  tui.rs           — ratatui TUI rendering (228 lines)
```

## Scalp Strategies (1m timeframe)

| Strategy | Signal | Direction |
|----------|--------|-----------|
| `scalp_vol_spike_rev` | Volume > 3.5x avg + RSI < 20 (or > 80) | Long / Short |
| `scalp_stoch_cross` | Stochastic K crosses D below 5 (or above 95) | Long / Short |
| `scalp_bb_squeeze_break` | BB width < 0.4x avg + vol > 2x + price breaks band | Long / Short |

## Live Results (First 13 Hours)

**Period:** 2026-03-14 21:26 → 2026-03-15 10:30

### Regime Trades (15m signals)

| Exit Reason | Wins | Losses | Win Rate | P&L |
|-------------|------|--------|----------|-----|
| SMA | 12 | 0 | 100% | +$4.15 |
| SL | 0 | 10 | 0% | -$1.67 |
| **Total** | **12** | **10** | **54.5%** | **+$2.48** |

### Scalp Trades (1m signals)

| Strategy | TP | SL | Win Rate | P&L |
|----------|----|----|----------|-----|
| scalp_stoch_cross | 81 | 96 | 45.8% | +$1.41 |
| scalp_vol_spike_rev | 23 | 25 | 47.9% | +$0.45 |
| scalp_bb_squeeze_break | 0 | 1 | 0% | -$0.05 |
| **Total** | **104** | **122** | **46.0%** | **+$1.81** |

### Combined

- **248 trades** in 13 hours, net **+$4.29** across 18 coins ($1,800 capital)
- **+0.24% portfolio return**
- Both layers (regime + scalp) independently profitable

### Observations

1. **Regime trades** remain the core profit driver — SMA exits are 12/12 winners. Stop losses fire frequently but are small (0.3% × 10% risk × 5x = 1.5% of trade).
2. **Scalp win rate is sub-50%** (46%) but net positive due to 2:1 TP:SL ratio (0.2% TP vs 0.1% SL). Expected by design.
3. **scalp_stoch_cross** dominates volume (177/226 scalp trades) — primary scalp signal.
4. **scalp_bb_squeeze_break** barely fires (1 trade) — the 0.4 squeeze factor is very restrictive.
5. Early results are promising but need 48+ hours for statistical significance.

## Configuration

### Regime (unchanged from v8)
- SL: 0.3%, No TP, No trailing
- Signal exits: SMA20 crossback, Z-score > 0.5 (after MIN_HOLD=2 candles)
- Risk: 10% per trade, 5x leverage
- 3-mode: LONG (breadth ≤ 20%), ISO_SHORT (20-50%), SHORT (≥ 50%)

### Scalp (new in v9)
- SL: 0.1%, TP: 0.2%
- Risk: 5% per trade, 5x leverage
- Params: vol_mult=3.5, rsi=20, stoch=5, bb_squeeze=0.4
- 1m candles fetched every 15s
- No cooldown between scalps (separate from regime cooldown)
