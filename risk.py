"""
Position sizing and portfolio risk management.
"""
import numpy as np
import pandas as pd
from scipy.optimize import minimize_scalar


def kelly_criterion(win_rate: float, avg_win: float, avg_loss: float) -> float:
    """
    Kelly criterion for optimal bet sizing. Capped at 25%.

    Args:
        win_rate: Win rate as decimal (e.g., 0.60)
        avg_win: Average win amount (positive)
        avg_loss: Average loss amount (positive, will be negated)

    Returns:
        Optimal fraction of bankroll to risk (0 to 0.25)
    """
    if avg_win <= 0 or avg_loss <= 0:
        return 0.0
    b = avg_win / avg_loss  # win/loss ratio
    p = win_rate
    q = 1 - p
    kelly = (b * p - q) / b
    return max(0.0, min(kelly, 0.25))


def half_kelly(win_rate: float, avg_win: float, avg_loss: float) -> float:
    """Half-Kelly: more conservative, less volatile."""
    return kelly_criterion(win_rate, avg_win, avg_loss) / 2


def optimal_f(trade_pnls: list) -> float:
    """
    Ralph Vince's optimal f — fraction that maximizes terminal wealth.

    Args:
        trade_pnls: List of trade P&L percentages

    Returns:
        Optimal fraction (0 to 1)
    """
    pnls = np.array(trade_pnls) / 100  # convert to decimals
    if len(pnls) < 5:
        return 0.0

    max_loss = abs(min(pnls))
    if max_loss == 0:
        return 0.0

    def neg_twr(f):
        """Negative terminal wealth relative — minimize to maximize."""
        if f <= 0 or f >= 1:
            return 0
        hpr = 1 + f * pnls / max_loss
        if np.any(hpr <= 0):
            return 0
        return -np.prod(hpr) ** (1 / len(pnls))

    result = minimize_scalar(neg_twr, bounds=(0.01, 0.99), method='bounded')
    return float(result.x) if result.success else 0.0


def atr_position_size(atr: float, risk_per_trade: float, account_size: float,
                      atr_mult: float = 2.0) -> float:
    """
    ATR-based position sizing.

    Args:
        atr: Current ATR value (in price units)
        risk_per_trade: Max risk per trade as decimal (e.g., 0.01 = 1%)
        account_size: Current account balance
        atr_mult: ATR multiplier for stop distance

    Returns:
        Position size in base currency units
    """
    if atr <= 0:
        return 0.0
    dollar_risk = account_size * risk_per_trade
    stop_distance = atr * atr_mult
    return dollar_risk / stop_distance


def fixed_fraction_size(account_size: float, fraction: float = 0.02) -> float:
    """Simple fixed fraction of account."""
    return account_size * fraction


def portfolio_var(returns_matrix: pd.DataFrame, confidence: float = 0.95) -> float:
    """
    Historical Value at Risk for a portfolio.

    Args:
        returns_matrix: DataFrame where each column is a coin's returns
        confidence: Confidence level (0.95 = 95%)

    Returns:
        VaR as a positive percentage (e.g., 2.5 means 2.5% loss at 95% CI)
    """
    # Equal-weight portfolio returns
    portfolio_returns = returns_matrix.mean(axis=1)
    var = -np.percentile(portfolio_returns.dropna(), (1 - confidence) * 100)
    return float(var)


def conditional_var(returns_matrix: pd.DataFrame, confidence: float = 0.95) -> float:
    """
    Conditional VaR (Expected Shortfall) — average loss beyond VaR.
    """
    portfolio_returns = returns_matrix.mean(axis=1).dropna()
    var_threshold = np.percentile(portfolio_returns, (1 - confidence) * 100)
    tail_losses = portfolio_returns[portfolio_returns <= var_threshold]
    return float(-tail_losses.mean()) if len(tail_losses) > 0 else 0.0


def correlation_risk(returns_matrix: pd.DataFrame) -> pd.DataFrame:
    """
    Compute correlation matrix across coins.

    Returns:
        Correlation matrix DataFrame
    """
    return returns_matrix.corr()


def max_correlated_positions(corr_matrix: pd.DataFrame, threshold: float = 0.7,
                              max_positions: int = 3) -> dict:
    """
    Identify groups of highly correlated coins.
    Recommends max simultaneous positions per group.

    Args:
        corr_matrix: Correlation matrix
        threshold: Correlation threshold to consider "highly correlated"
        max_positions: Max simultaneous positions in a correlated group

    Returns:
        dict with correlated groups and recommendations
    """
    coins = corr_matrix.columns.tolist()
    groups = []
    assigned = set()

    for i, coin in enumerate(coins):
        if coin in assigned:
            continue
        group = [coin]
        assigned.add(coin)
        for j, other in enumerate(coins):
            if other in assigned or i == j:
                continue
            if abs(corr_matrix.iloc[i, j]) >= threshold:
                group.append(other)
                assigned.add(other)
        if len(group) > 1:
            groups.append(group)

    return {
        'correlated_groups': groups,
        'max_per_group': max_positions,
        'uncorrelated': [c for c in coins if c not in assigned],
    }
