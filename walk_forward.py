"""
Reusable walk-forward validation framework.
Consolidates the walk-forward pattern from RUN X.2 scripts.
"""
import pandas as pd
import numpy as np
from typing import Callable, Iterator, Optional
from backtester import Backtester, BacktestResult


class WalkForward:
    """
    Walk-forward validation engine.

    Splits data into sequential train/test windows and evaluates
    strategy performance on out-of-sample data.
    """

    def __init__(self, df: pd.DataFrame, n_windows: int = 3, train_ratio: float = 0.67):
        """
        Args:
            df: Full OHLCV DataFrame
            n_windows: Number of walk-forward windows
            train_ratio: Fraction of each window used for training
        """
        self.df = df
        self.n_windows = n_windows
        self.train_ratio = train_ratio

    def windows(self) -> Iterator[tuple]:
        """
        Yield (train_df, test_df) pairs for each walk-forward window.
        """
        n = len(self.df)
        window_size = n // self.n_windows

        for w in range(self.n_windows):
            start = w * window_size
            end = min((w + 1) * window_size, n)
            train_end = start + int((end - start) * self.train_ratio)

            train_df = self.df.iloc[start:train_end]
            test_df = self.df.iloc[train_end:end]

            if len(train_df) > 0 and len(test_df) > 0:
                yield train_df, test_df

    def run(self, strategy_fn: Callable, optimize_fn: Optional[Callable] = None,
            backtest_fn: Optional[Callable] = None,
            direction: str = 'long', stop_loss: Optional[float] = None) -> dict:
        """
        Run walk-forward validation.

        Args:
            strategy_fn: fn(df, **params) -> (entry, exit) signals
            optimize_fn: Optional fn(df) -> dict of optimized params.
                         If None, uses default params.
            backtest_fn: Optional custom backtest function fn(df, entry, exit) -> BacktestResult.
                         If None, uses default Backtester.
            direction: 'long' or 'short'
            stop_loss: Optional stop loss percentage

        Returns:
            dict with per-window results and aggregate stats
        """
        window_results = []
        is_results = []  # in-sample
        oos_results = []  # out-of-sample

        for w, (train_df, test_df) in enumerate(self.windows()):
            # Optimize on train (if optimizer provided)
            params = optimize_fn(train_df) if optimize_fn else {}

            # In-sample backtest
            try:
                is_entry, is_exit = strategy_fn(train_df, **params)
                bt_is = Backtester(train_df, fee=0.001, slippage=0.0005)
                is_result = bt_is.run(is_entry, is_exit, direction=direction, stop_loss=stop_loss)
                is_results.append(is_result)
            except Exception:
                is_result = None

            # Out-of-sample backtest
            try:
                oos_entry, oos_exit = strategy_fn(test_df, **params)
                if backtest_fn:
                    oos_result = backtest_fn(test_df, oos_entry, oos_exit)
                else:
                    bt_oos = Backtester(test_df, fee=0.001, slippage=0.0005)
                    oos_result = bt_oos.run(oos_entry, oos_exit, direction=direction, stop_loss=stop_loss)
                oos_results.append(oos_result)
            except Exception:
                oos_result = None

            window_results.append({
                'window': w,
                'train_period': f'{train_df.index[0]} to {train_df.index[-1]}',
                'test_period': f'{test_df.index[0]} to {test_df.index[-1]}',
                'params': params,
                'in_sample': _result_to_dict(is_result) if is_result else None,
                'out_of_sample': _result_to_dict(oos_result) if oos_result else None,
            })

        # Aggregate
        degradation = walk_forward_degradation(is_results, oos_results)

        return {
            'n_windows': len(window_results),
            'windows': window_results,
            'aggregate': {
                'oos_avg_win_rate': _safe_mean([r.win_rate for r in oos_results]),
                'oos_avg_pf': _safe_mean([r.profit_factor for r in oos_results]),
                'oos_avg_sharpe': _safe_mean([r.Sharpe_ratio for r in oos_results]),
                'oos_total_trades': sum(r.total_trades for r in oos_results),
                'is_avg_win_rate': _safe_mean([r.win_rate for r in is_results]),
                'is_avg_pf': _safe_mean([r.profit_factor for r in is_results]),
                'degradation_wr': degradation['win_rate'],
                'degradation_pf': degradation['profit_factor'],
            }
        }


def walk_forward_degradation(is_results: list, oos_results: list) -> dict:
    """
    Compute degradation between in-sample and out-of-sample results.
    Positive = degradation (OOS worse than IS).
    """
    if not is_results or not oos_results:
        return {'win_rate': 0, 'profit_factor': 0}

    is_wr = _safe_mean([r.win_rate for r in is_results])
    oos_wr = _safe_mean([r.win_rate for r in oos_results])
    is_pf = _safe_mean([r.profit_factor for r in is_results])
    oos_pf = _safe_mean([r.profit_factor for r in oos_results])

    wr_deg = ((is_wr - oos_wr) / is_wr * 100) if is_wr > 0 else 0
    pf_deg = ((is_pf - oos_pf) / is_pf * 100) if is_pf > 0 else 0

    return {
        'win_rate': float(wr_deg),
        'profit_factor': float(pf_deg),
    }


def _result_to_dict(result: BacktestResult) -> dict:
    return {
        'total_trades': result.total_trades,
        'win_rate': result.win_rate,
        'profit_factor': result.profit_factor,
        'sharpe': result.Sharpe_ratio,
        'max_drawdown': result.max_drawdown,
        'avg_win': result.avg_win,
        'avg_loss': result.avg_loss,
    }


def _safe_mean(values: list) -> float:
    if not values:
        return 0.0
    return float(np.mean(values))
