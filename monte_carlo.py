"""
Monte Carlo simulation and bootstrap confidence intervals for trading strategies.
"""
import numpy as np
from typing import Optional


def monte_carlo_trades(trade_pnls: list, n_simulations: int = 10000,
                       initial_balance: float = 10000) -> dict:
    """
    Monte Carlo by reshuffling trade P&L order.
    Tests whether results depend on trade sequence.

    Args:
        trade_pnls: list of trade P&L percentages
        n_simulations: number of reshuffles
        initial_balance: starting capital

    Returns:
        dict with percentile outcomes (5th, 25th, 50th, 75th, 95th)
    """
    pnls = np.array(trade_pnls)
    n_trades = len(pnls)
    if n_trades < 2:
        return {'error': 'Need at least 2 trades'}

    final_balances = np.empty(n_simulations)
    max_drawdowns = np.empty(n_simulations)

    for i in range(n_simulations):
        shuffled = np.random.permutation(pnls)
        equity = initial_balance
        peak = equity
        max_dd = 0.0
        for pnl in shuffled:
            equity *= (1 + pnl / 100)
            if equity > peak:
                peak = equity
            dd = (peak - equity) / peak
            if dd > max_dd:
                max_dd = dd
        final_balances[i] = equity
        max_drawdowns[i] = max_dd * 100

    total_returns = (final_balances - initial_balance) / initial_balance * 100
    percentiles = [5, 25, 50, 75, 95]

    return {
        'n_trades': n_trades,
        'n_simulations': n_simulations,
        'return_percentiles': {
            f'p{p}': float(np.percentile(total_returns, p)) for p in percentiles
        },
        'drawdown_percentiles': {
            f'p{p}': float(np.percentile(max_drawdowns, p)) for p in percentiles
        },
        'actual_return': float(total_returns[0]) if n_simulations > 0 else 0,
        'mean_return': float(np.mean(total_returns)),
        'std_return': float(np.std(total_returns)),
        'prob_profit': float(np.mean(total_returns > 0)),
    }


def monte_carlo_paths(returns: np.ndarray, n_simulations: int = 1000,
                      n_days: int = 252, initial_balance: float = 10000) -> dict:
    """
    Monte Carlo equity curve paths by resampling daily returns.

    Args:
        returns: array of period returns (as decimals, e.g., 0.01 = 1%)
        n_simulations: number of paths
        n_days: number of days to simulate
        initial_balance: starting capital

    Returns:
        dict with path statistics and percentile curves
    """
    returns = np.array(returns)
    returns = returns[~np.isnan(returns)]
    if len(returns) < 10:
        return {'error': 'Need at least 10 returns'}

    paths = np.empty((n_simulations, n_days + 1))
    paths[:, 0] = initial_balance

    for i in range(n_simulations):
        sampled = np.random.choice(returns, size=n_days, replace=True)
        paths[i, 1:] = initial_balance * np.cumprod(1 + sampled)

    final_values = paths[:, -1]
    total_returns = (final_values - initial_balance) / initial_balance * 100

    percentiles = [5, 25, 50, 75, 95]
    percentile_curves = {}
    for p in percentiles:
        percentile_curves[f'p{p}'] = np.percentile(paths, p, axis=0).tolist()

    return {
        'n_simulations': n_simulations,
        'n_days': n_days,
        'return_percentiles': {
            f'p{p}': float(np.percentile(total_returns, p)) for p in percentiles
        },
        'mean_final': float(np.mean(final_values)),
        'prob_profit': float(np.mean(total_returns > 0)),
        'percentile_curves': percentile_curves,
    }


def confidence_interval(trade_pnls: list, confidence: float = 0.95,
                         n_bootstrap: int = 10000) -> dict:
    """
    Bootstrap confidence intervals for key trading metrics.

    Args:
        trade_pnls: list of trade P&L percentages
        confidence: confidence level (0.95 = 95%)
        n_bootstrap: number of bootstrap samples

    Returns:
        dict with CIs for win_rate, profit_factor, sharpe, avg_pnl
    """
    pnls = np.array(trade_pnls)
    n = len(pnls)
    if n < 5:
        return {'error': 'Need at least 5 trades'}

    alpha = (1 - confidence) / 2

    win_rates = np.empty(n_bootstrap)
    profit_factors = np.empty(n_bootstrap)
    avg_pnls = np.empty(n_bootstrap)
    sharpes = np.empty(n_bootstrap)

    for i in range(n_bootstrap):
        sample = np.random.choice(pnls, size=n, replace=True)
        wins = sample[sample > 0]
        losses = sample[sample <= 0]

        win_rates[i] = len(wins) / n * 100
        total_win = wins.sum() if len(wins) > 0 else 0
        total_loss = abs(losses.sum()) if len(losses) > 0 else 1e-10
        profit_factors[i] = total_win / total_loss
        avg_pnls[i] = sample.mean()
        std = sample.std()
        sharpes[i] = (sample.mean() / std * np.sqrt(252)) if std > 0 else 0

    def ci(arr):
        return {
            'mean': float(np.mean(arr)),
            'lower': float(np.percentile(arr, alpha * 100)),
            'upper': float(np.percentile(arr, (1 - alpha) * 100)),
        }

    return {
        'confidence': confidence,
        'n_trades': n,
        'n_bootstrap': n_bootstrap,
        'win_rate': ci(win_rates),
        'profit_factor': ci(profit_factors),
        'avg_pnl': ci(avg_pnls),
        'sharpe': ci(sharpes),
    }
