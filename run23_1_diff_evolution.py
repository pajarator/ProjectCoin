"""
RUN23.1 — Differential Evolution Optimization

Uses scipy.optimize.differential_evolution as drop-in replacement for grid search.
Optimizes strategy parameters on first 8mo, evaluates on last 4mo.
Tests all current strategies, compares to grid search results.

Output: run23_1_results.json
Checkpoint: run23_1_checkpoint.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm
from scipy.optimize import differential_evolution

from feature_engine import load_cached_data, COINS
from indicators import add_all_indicators, RSI, BOLLINGER_BANDS, STOCHASTIC, ATR
from backtester import Backtester

CHECKPOINT_FILE = 'run23_1_checkpoint.json'
RESULTS_FILE = 'run23_1_results.json'

shutdown_requested = False
def signal_handler(sig, frame):
    global shutdown_requested
    print('\nShutdown requested, saving checkpoint...')
    shutdown_requested = True
signal.signal(signal.SIGINT, signal_handler)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return {'completed_coins': [], 'results': {}}


def save_checkpoint(state):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(state, f, indent=2, default=str)


# Parameterized strategy templates
def mean_reversion_param(df, rsi_period, rsi_lo, rsi_hi, bb_std):
    """Parameterized mean reversion strategy."""
    rsi = RSI(df['close'], int(rsi_period))
    bb_upper, bb_mid, bb_lower = BOLLINGER_BANDS(df['close'], 20, bb_std)
    entry = (rsi < rsi_lo) & (df['close'] < bb_lower)
    exit_sig = (rsi > rsi_hi) | (df['close'] > bb_mid)
    return entry, exit_sig


def momentum_param(df, rsi_period, rsi_lo, stoch_lo, stoch_hi):
    """Parameterized momentum strategy."""
    rsi = RSI(df['close'], int(rsi_period))
    stoch_k, stoch_d = STOCHASTIC(df['high'], df['low'], df['close'])
    entry = (rsi > rsi_lo) & (stoch_k > stoch_lo) & (stoch_k > stoch_d)
    exit_sig = (rsi < 50) | (stoch_k < stoch_hi)
    return entry, exit_sig


def volatility_breakout_param(df, bb_std, atr_mult, vol_thresh):
    """Parameterized volatility breakout strategy."""
    bb_upper, bb_mid, bb_lower = BOLLINGER_BANDS(df['close'], 20, bb_std)
    atr = ATR(df['high'], df['low'], df['close'], 14)
    vol_sma = df['volume'].rolling(20).mean()
    vol_ratio = df['volume'] / vol_sma

    entry = (df['close'] > bb_upper) & (vol_ratio > vol_thresh)
    exit_sig = df['close'] < bb_mid
    return entry, exit_sig


STRATEGY_CONFIGS = {
    'mean_reversion': {
        'func': mean_reversion_param,
        'bounds': [(7, 21), (20, 35), (65, 80), (1.5, 3.0)],
        'param_names': ['rsi_period', 'rsi_lo', 'rsi_hi', 'bb_std'],
    },
    'momentum': {
        'func': momentum_param,
        'bounds': [(7, 21), (40, 60), (20, 40), (60, 80)],
        'param_names': ['rsi_period', 'rsi_lo', 'stoch_lo', 'stoch_hi'],
    },
    'volatility_breakout': {
        'func': volatility_breakout_param,
        'bounds': [(1.5, 3.0), (1.0, 3.0), (1.0, 3.0)],
        'param_names': ['bb_std', 'atr_mult', 'vol_thresh'],
    },
}


def optimize_coin(coin: str) -> dict:
    """Run DE optimization for all strategy templates on one coin."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)

    split = int(len(df) * 0.67)
    train_df = df.iloc[:split]
    test_df = df.iloc[split:]

    coin_results = {}

    for strat_name, config in STRATEGY_CONFIGS.items():
        func = config['func']
        bounds = config['bounds']
        param_names = config['param_names']

        def objective(params):
            """Negative Sharpe * sqrt(trades) for minimization."""
            try:
                entry, exit_sig = func(train_df, *params)
                bt = Backtester(train_df, fee=0.001, slippage=0.0005)
                result = bt.run(entry, exit_sig, stop_loss=0.003)
                if result.total_trades < 5:
                    return 999
                score = result.Sharpe_ratio * np.sqrt(result.total_trades)
                if result.profit_factor < 0.8:
                    score *= 0.5
                return -score
            except Exception:
                return 999

        try:
            de_result = differential_evolution(
                objective, bounds,
                maxiter=100, seed=42, tol=1e-6,
                mutation=(0.5, 1.5), recombination=0.7,
                popsize=15, polish=False
            )

            best_params = dict(zip(param_names, de_result.x))

            # Evaluate on test set
            entry, exit_sig = func(test_df, *de_result.x)
            bt = Backtester(test_df, fee=0.001, slippage=0.0005)
            test_result = bt.run(entry, exit_sig, stop_loss=0.003)

            # Default params for comparison
            default_params = [b[0] + (b[1] - b[0]) / 2 for b in bounds]
            entry_def, exit_def = func(test_df, *default_params)
            bt_def = Backtester(test_df, fee=0.001, slippage=0.0005)
            default_result = bt_def.run(entry_def, exit_def, stop_loss=0.003)

            coin_results[strat_name] = {
                'de_params': {k: float(v) for k, v in best_params.items()},
                'de_fitness': float(-de_result.fun),
                'de_test': {
                    'trades': test_result.total_trades,
                    'win_rate': test_result.win_rate,
                    'profit_factor': test_result.profit_factor,
                    'sharpe': test_result.Sharpe_ratio,
                    'total_pnl_pct': test_result.total_pnl_pct,
                },
                'default_test': {
                    'trades': default_result.total_trades,
                    'win_rate': default_result.win_rate,
                    'profit_factor': default_result.profit_factor,
                    'sharpe': default_result.Sharpe_ratio,
                    'total_pnl_pct': default_result.total_pnl_pct,
                },
                'improvement': {
                    'pf_delta': test_result.profit_factor - default_result.profit_factor,
                    'wr_delta': test_result.win_rate - default_result.win_rate,
                },
            }
        except Exception as e:
            coin_results[strat_name] = {'error': str(e)}

    return {'coin': coin, 'strategies': coin_results}


def main():
    print('=' * 60)
    print('RUN23.1 — Differential Evolution Optimization')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='DE optimization'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = optimize_coin(coin)
            results[coin] = result
            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)

            for sname, sresult in result['strategies'].items():
                if 'de_test' in sresult:
                    dt = sresult['de_test']
                    imp = sresult['improvement']
                    print(f'  {sname}: PF={dt["profit_factor"]:.2f} (delta={imp["pf_delta"]:+.2f}) '
                          f'WR={dt["win_rate"]:.1f}% {dt["trades"]}t')
        except Exception as e:
            print(f'  FAILED: {e}')

    if len(completed) == len(COINS):
        # Summary
        all_pf_deltas = []
        for r in results.values():
            for s in r.get('strategies', {}).values():
                if 'improvement' in s:
                    all_pf_deltas.append(s['improvement']['pf_delta'])

        final = {
            'per_coin': results,
            'summary': {
                'avg_pf_improvement': float(np.mean(all_pf_deltas)) if all_pf_deltas else 0,
                'pct_improved': float(np.mean([d > 0 for d in all_pf_deltas]) * 100) if all_pf_deltas else 0,
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Avg PF improvement: {final["summary"]["avg_pf_improvement"]:+.3f}')
        print(f'Improved: {final["summary"]["pct_improved"]:.0f}%')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)


if __name__ == '__main__':
    main()
