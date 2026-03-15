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

## Backtest/Optimization Script Requirements

All long-running backtest and optimization scripts (run*_.py) MUST include:
1. **State saving** — Periodically save partial results to a JSON checkpoint file (e.g., `run6_1_checkpoint.json`) so that work is not lost on crash/interrupt. On startup, check for and resume from checkpoint.
2. **Progress bar** — Use `tqdm` or print a progress indicator showing current combo / total combos and estimated time remaining.
3. **Graceful stop/recover** — Handle SIGINT (Ctrl+C) by saving current state before exiting. On restart, resume from the last checkpoint.

## Top Findings (from RUN1.md)

Best strategies achieving ≥70% win rate: Mean Reversion (72-87%), Williams %R (67-87%), ADR Reversal (68-80%). Best overall: Mean Reversion on 15m timeframe. Strategies that don't work well: MACD Crossover, EMA Crossover, Volume Breakout.
