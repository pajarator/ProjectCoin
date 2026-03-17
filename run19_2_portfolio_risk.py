"""
RUN19.2 — Portfolio Risk Analysis

Correlation matrix across 19 coins.
Test max-3-simultaneous-correlated constraint.
Compare portfolio Sharpe with and without constraints.

Output: run19_2_results.json
"""
import json
import numpy as np
import pandas as pd

from feature_engine import load_cached_data, COINS
from risk import portfolio_var, conditional_var, correlation_risk, max_correlated_positions


def main():
    print('=' * 60)
    print('RUN19.2 — Portfolio Risk Analysis')
    print('=' * 60)

    # Build returns matrix
    returns = {}
    for coin in COINS:
        df = load_cached_data(coin)
        returns[coin] = df['close'].pct_change()

    returns_df = pd.DataFrame(returns).dropna()
    print(f'Returns matrix: {returns_df.shape}')

    # Correlation matrix
    corr = correlation_risk(returns_df)
    print(f'\nCorrelation matrix computed')

    # Find highly correlated pairs
    high_corr = []
    for i in range(len(corr)):
        for j in range(i + 1, len(corr)):
            if abs(corr.iloc[i, j]) >= 0.7:
                high_corr.append({
                    'pair': f'{corr.index[i]}-{corr.columns[j]}',
                    'correlation': float(corr.iloc[i, j]),
                })
    high_corr.sort(key=lambda x: -abs(x['correlation']))

    print(f'\nHighly correlated pairs (>0.7):')
    for p in high_corr[:10]:
        print(f'  {p["pair"]}: {p["correlation"]:.3f}')

    # Correlated groups
    groups = max_correlated_positions(corr, threshold=0.7, max_positions=3)
    print(f'\nCorrelated groups: {groups["correlated_groups"]}')
    print(f'Uncorrelated coins: {groups["uncorrelated"]}')

    # VaR analysis
    var_95 = portfolio_var(returns_df, 0.95)
    var_99 = portfolio_var(returns_df, 0.99)
    cvar_95 = conditional_var(returns_df, 0.95)
    print(f'\nPortfolio VaR (95%): {var_95:.4f}')
    print(f'Portfolio VaR (99%): {var_99:.4f}')
    print(f'CVaR/ES (95%):       {cvar_95:.4f}')

    # Compare: full portfolio vs uncorrelated subset
    if groups['uncorrelated']:
        uncorr_returns = returns_df[groups['uncorrelated']]
        uncorr_var = portfolio_var(uncorr_returns, 0.95)
        print(f'\nUncorrelated subset VaR (95%): {uncorr_var:.4f}')
        print(f'Risk reduction: {(1 - uncorr_var / var_95) * 100:.1f}%')
    else:
        uncorr_var = var_95

    # Portfolio Sharpe
    port_returns = returns_df.mean(axis=1)
    port_sharpe = float(port_returns.mean() / port_returns.std() * np.sqrt(252 * 96))  # 96 = 15m bars per day
    print(f'\nPortfolio Sharpe (equal weight): {port_sharpe:.3f}')

    # Per-coin metrics
    per_coin = {}
    for coin in COINS:
        r = returns_df[coin]
        per_coin[coin] = {
            'annual_return': float(r.mean() * 252 * 96 * 100),
            'annual_vol': float(r.std() * np.sqrt(252 * 96) * 100),
            'sharpe': float(r.mean() / r.std() * np.sqrt(252 * 96)) if r.std() > 0 else 0,
            'var_95': float(-np.percentile(r.dropna(), 5)),
            'max_daily_loss': float(-r.min() * 100),
        }

    results = {
        'correlation_matrix': corr.to_dict(),
        'high_corr_pairs': high_corr,
        'correlated_groups': groups,
        'portfolio_var_95': var_95,
        'portfolio_var_99': var_99,
        'portfolio_cvar_95': cvar_95,
        'portfolio_sharpe': port_sharpe,
        'per_coin': per_coin,
    }

    with open('run19_2_results.json', 'w') as f:
        json.dump(results, f, indent=2, default=str)

    print(f'\nResults saved to run19_2_results.json')


if __name__ == '__main__':
    main()
