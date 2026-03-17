# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ProjectCoin is a crypto day trading strategy discovery and paper trading system. The goal is to find strategies with ≥70% win rate. It uses CCXT (Binance) for market data, custom technical indicators, and a backtesting engine.

## Running

All scripts are standalone Python files. The project uses a virtualenv at `venv/`.

```bash
source venv/bin/activate

# Run backtests across all strategies for a symbol
python main.py

# Paper trade a single coin (VWAP Reversion on ETH/USDT 15m)
python paper_trading.py

# Paper trade with TUI (single coin)
python paper_trading_tui.py

# Multi-coin paper trading (top 20 performers, plain output)
python multi.py              # use --reset to clear state

# Multi-coin paper trading with curses TUI (split price/log view)
python multi_curses.py
```

There is no test suite, linter, or build system.

## Architecture

**Data pipeline:** `data_fetcher.py` → `indicators.py` → strategy module → `backtester.py`

- **`data_fetcher.py`** — Fetches OHLCV candles from Binance via CCXT. Returns pandas DataFrames. `DEFAULT_PAIRS` defines the 15 default trading pairs.
- **`indicators.py`** — Pure-function technical indicators (RSI, MACD, Bollinger Bands, ATR, Stochastic, VWAP, ADX, etc.). `add_all_indicators(df)` attaches all indicators to a DataFrame.
- **`strategies.py`** — 15 base strategies. Each is a function `(df, **kwargs) -> (entry_signal, exit_signal)` returning boolean pd.Series. Exported via `STRATEGIES` dict.
- **`strategies_enhanced.py`** — 8 enhanced strategies (Pro variants, confluence, VWAP reversion, etc.). Exported via `ENHANCED_STRATEGIES` dict.
- **`strategies_new.py`** — Additional strategies (dual RSI, Keltner channel, etc.) used by multi-coin paper trading.
- **`backtester.py`** — `Backtester` class takes a DataFrame + entry/exit signals, simulates trades with fees (0.1%) and slippage (0.05%), returns `BacktestResult` dataclass with win rate, profit factor, Sharpe, drawdown. Also contains `scan_for_patterns()` for candlestick pattern detection.
- **`main.py`** — Orchestrator that combines all strategies, runs backtests across symbols/timeframes, and has parameter optimization (`optimize_strategy`).

**Paper trading layer:**
- **`paper_trading.py`** / **`paper_trading_tui.py`** — Single-coin live paper trading (ETH/USDT, VWAP Reversion strategy).
- **`multi.py`** / **`multi_curses.py`** — Multi-coin paper trading across 20 coins with different strategies per coin. State persists in `trading_state.json`; logs to `trading_log.txt`.
- **`multi_coin_trading.py`** — Earlier multi-coin implementation.

## Key Conventions

- Strategy functions always return `(entry: pd.Series[bool], exit: pd.Series[bool])` tuples.
- DataFrames use OHLCV columns: `open`, `high`, `low`, `close`, `volume` with a `timestamp` index.
- Indicator functions are capitalized (e.g., `RSI()`, `MACD()`, `EMA()`).
- Paper trading state files use absolute paths to `/home/scamarena/ProjectCoin/`.

## Optimization Workflow (RUN Process)

Each optimization experiment follows a numbered RUN (RUN1, RUN2, ... RUN8). A RUN tests one hypothesis about the trading system — e.g., "does a tighter stop loss help?" or "does adding take profit help?"

### RUN Lifecycle

1. **Hypothesize** — Write a plan identifying the parameter or mechanism to test. Define the grid of values, the baseline to compare against, and what "better" means (PF, P&L, WR, MaxDD).

2. **RUN X.1 — Grid Search** — Create `runX_1_*.py`. Backtest all parameter combos across 18 coins using cached 5-month 15m data from `data_cache/`. Includes shadow/counterfactual tracking to understand *why* a change helps or hurts. Outputs `runX_1_results.json`. **Early stop:** if the grid search shows the hypothesis clearly fails (e.g., net_impact < 0 across most coins, PF degrades vs baseline), stop here — do not run steps 3-4.

3. **RUN X.2 — Walk-Forward Validation** — Create `runX_2_*.py`. 3-window walk-forward (train 2mo, test 1mo) to check if in-sample results hold out-of-sample. Compares per-coin optimized params, universal params (from X.1), and baseline. Reports degradation %. Universal params preferred if per-coin degrades >40%.

4. **RUN X.3 — Combined Comparison** — Create `runX_3_*.py`. Side-by-side backtest of current trader.py config vs proposed new config. Per-coin breakdown, portfolio stats, exit reason distribution.

5. **Apply (if beneficial)** — Update `trader.py` with the winning params. Bump the version header (e.g., "COINCLAW v8" → "COINCLAW v9").

6. **Archive** — Move all RUN files to `archive/RUNX/`: scripts, results JSONs, and `RUNX.md` documenting the full experiment with results tables and conclusions.

### Archive Structure

```
archive/
  RUN1/  — Strategy discovery (best strategies per coin)
  RUN2/  — Parameter tuning
  RUN3/  — Mean reversion deep-dive
  RUN4/  — Long directional trading optimization
  RUN5/  — Short (market-dump) strategies
  RUN6/  — ISO short (coin-specific overbought) strategies
  RUN7/  — Stop loss optimization (result: SL 0.5% → 0.3%)
  RUN8/  — Take profit optimization (result: TP does not help)
  RUN10/ — Scalp indicator discovery, F6 filter validation, fee-aware TP/SL optimization
  RUN11/ — STRAT1000 strategy discovery (45 strategies, result: DASH → OuMeanRev)
  RUN12/ — Scalp market mode filter (scalps must match regime direction)
  RUN13/ — STRAT1000 final sweep: complementary signals (Laguerre RSI, Kalman Filter, KST Cross)
  RUN14/ — Indicator library expansion: +18 indicators, requirements.txt, ML infrastructure (feature_engine, monte_carlo, walk_forward, risk, ensemble, dashboard)
  RUN15/ — Feature engineering pipeline: 66-feature ML matrix, disk cache for all 19 coins, 3 indicator bug fixes (doji NaN, CMF propagation, flat RSI)
  RUN16/ — ML feature importance + walk-forward: RF/XGB on 66 features across 19 coins. Result: NEGATIVE — 54.6% IS accuracy doesn't survive OOS (RF −1.9% vs baseline). ML gating helps only 21% of strategies. No COINCLAW changes.
  RUN17/ — Monte Carlo validation of COINCLAW v13 primary long strategies. Result: POSITIVE — 18/18 ROBUST (100% prob_profit, p5_PF = actual PF). Portfolio p50 return +15,045%, typical max DD 4.0%. No COINCLAW changes needed (validation only).
  RUN19/ — Position sizing comparison: Fixed 1%/2%/5% vs Kelly(OOS) vs Half-Kelly(OOS). Result: NEGATIVE — 17/18 strategies lose money in OOS test half; Kelly amplifies losses 15–17× vs Fixed 1% in losing period. No COINCLAW changes. Fixed 2% remains default.
  RUN20/ — Momentum crowding filter (OHLCV proxy for funding-rate pressure). Result: NEGATIVE — Mom>1% filter improves P&L by 8.4pp but stays 10pts below 44% breakeven WR. VolCrowd and AntiMom filters are counterproductive. No COINCLAW changes.
  RUN21/ — Sentiment regime filter (BTC RSI as Fear/Greed proxy). Result: NEGATIVE — Fear regime reduces WR to 30.4% vs 33.6% baseline; opposite of hypothesis. No regime clears 44% breakeven. No COINCLAW changes.
  RUN22/ — Genetic algorithm v2 strategy discovery. Result: NEGATIVE — 0/18 coins achieve >44% WR on OOS hold-out. Corrected Python bug (test-set fitness). GA overfits train regime; evolved rules degrade on test. z20+ADX most selected. No COINCLAW changes.
  RUN23/ — Differential evolution parameter optimization. Result: NEGATIVE — 0/54 strategy-coin pairs achieve >44% WR on OOS. DE improves momentum PF (18/18 coins, +0.216 avg) but from a catastrophic baseline. Mean_reversion DE is neutral vs default (avg PFdelta -0.034). Volatility_breakout DE hurts 15/18 coins. Parameter tuning cannot fix regime mismatch. No COINCLAW changes.
  RUN24/ — Ensemble strategy framework: voting/intersection of top-3 per coin. Result: NEGATIVE — 1/90 combinations (LINK AND) clears 44% WR. Majority vote reduces avg PF vs best single (0.811 vs 0.879). Strategies too correlated (all mean-reversion) for ensemble diversity. AND-intersection wins by trade filtering, not signal quality. No COINCLAW changes.
  RUN25/ — ML regime detection: BTC 4-regime framework (BULL/BEAR × HIGH/LOW vol). Result: NEGATIVE — 1/72 combinations clears 44% WR. BEAR_HIGH_VOL is best (+0.41pp vs baseline at 34.20%). BULL_LOW_VOL worst (28.14%). Dropped circular ML (labels = f(features)). No COINCLAW changes.
```

Each `RUNX.md` contains: goal, method, full results tables, per-coin breakdowns, and conclusions. These serve as the institutional memory of what was tested and why.

### Script Naming Convention

- `runX_1_*.py` — Grid search
- `runX_2_*.py` — Walk-forward validation
- `runX_3_*.py` — Combined comparison
- Templates: use the most recent prior RUN's scripts as templates (e.g., RUN8 scripts were based on `archive/RUN7/run7_*.py`)

### Current System State (COINCLAW v13)

- **coinclaw/** — Rust live trader. All RUN results flow into this codebase.
- Regime trades: SL=0.3% (RUN7), no trailing, no TP (RUN8 confirmed TP hurts)
- Signal exits: SMA20 crossback, Z-score reversion (after MIN_HOLD=2 candles)
- 3-mode market regime: LONG (breadth ≤20%), ISO_SHORT (20-50%), SHORT (≥50%)
- 18 coins, per-coin optimal long/short/ISO-short strategies
- DASH uses OuMeanRev (RUN11c: OU half-life regime detection, dev=2.0σ threshold)
- Complement signals (RUN13): Laguerre RSI, Kalman Filter, KST Cross as secondary long entries when primary doesn't fire. 15/18 coins have complement assignments (+14.2% portfolio P&L, 62.7% WR on 640 complement trades)
- Scalp overlay (RUN10): TP=0.80%, SL=0.10%, F6 filter gate, vol_spike_rev + stoch_cross (bb_squeeze removed)
- Scalp direction must match market mode (RUN12): no shorts in LONG mode, no longs in SHORT mode
- 15m candle fetch: 150 bars (increased from 50 for OU indicator window)

## Backtest/Optimization Script Requirements

All long-running backtest and optimization scripts (runX_*.py) MUST include:
1. **State saving** — Periodically save partial results to a JSON checkpoint file (e.g., `run6_1_checkpoint.json`) so that work is not lost on crash/interrupt. On startup, check for and resume from checkpoint.
2. **Progress bar** — Use `tqdm` or print a progress indicator showing current combo / total combos and estimated time remaining.
3. **Graceful stop/recover** — Handle SIGINT (Ctrl+C) by saving current state before exiting. On restart, resume from the last checkpoint.

## Top Findings (from RUN1.md)

Best strategies achieving ≥70% win rate: Mean Reversion (72-87%), Williams %R (67-87%), ADR Reversal (68-80%). Best overall: Mean Reversion on 15m timeframe. Strategies that don't work well: MACD Crossover, EMA Crossover, Volume Breakout.
