"""
RUN24.1 — Ensemble Strategy Testing

Test ensembles of top-3 strategies per coin:
  - Equal-weight voting
  - PF-weighted voting
  - Stacking with logistic regression
Compare to best single strategy per coin.

Output: run24_1_results.json
Checkpoint: run24_1_checkpoint.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm

from feature_engine import load_cached_data, COINS
from indicators import add_all_indicators
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
from backtester import Backtester
from ensemble import ensemble_vote, ensemble_stacking

CHECKPOINT_FILE = 'run24_1_checkpoint.json'
RESULTS_FILE = 'run24_1_results.json'
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}

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


def find_top_strategies(df: pd.DataFrame, n: int = 3) -> list:
    """Find top-n strategies by profit factor for this data."""
    scores = []
    for name, func in ALL_STRATEGIES.items():
        try:
            entry, exit_sig = func(df)
            bt = Backtester(df, fee=0.001, slippage=0.0005)
            result = bt.run(entry, exit_sig, stop_loss=0.003)
            if result.total_trades >= 10:
                scores.append((name, func, result.profit_factor, result.win_rate))
        except Exception:
            continue

    scores.sort(key=lambda x: -x[2])
    return scores[:n]


def process_coin(coin: str) -> dict:
    """Test ensemble methods for one coin."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)

    # Use last 33% as test
    split = int(len(df) * 0.67)
    test_df = df.iloc[split:]

    # Find top-3 strategies on full data (slight look-ahead, but acceptable for comparison)
    top3 = find_top_strategies(df, n=3)
    if len(top3) < 2:
        return {'coin': coin, 'error': 'Not enough valid strategies'}

    strat_names = [t[0] for t in top3]
    strat_funcs = [t[1] for t in top3]
    strat_pfs = [t[2] for t in top3]

    # Best single strategy on test set
    best_entry, best_exit = strat_funcs[0](test_df)
    bt_best = Backtester(test_df, fee=0.001, slippage=0.0005)
    best_result = bt_best.run(best_entry, best_exit, stop_loss=0.003)

    results = {
        'coin': coin,
        'top_strategies': strat_names,
        'best_single': {
            'strategy': strat_names[0],
            'trades': best_result.total_trades,
            'win_rate': best_result.win_rate,
            'profit_factor': best_result.profit_factor,
            'sharpe': best_result.Sharpe_ratio,
        },
    }

    # Equal-weight voting
    try:
        eq_entry, eq_exit = ensemble_vote(strat_funcs, test_df)
        bt_eq = Backtester(test_df, fee=0.001, slippage=0.0005)
        eq_result = bt_eq.run(eq_entry, eq_exit, stop_loss=0.003)
        results['equal_vote'] = {
            'trades': eq_result.total_trades,
            'win_rate': eq_result.win_rate,
            'profit_factor': eq_result.profit_factor,
            'sharpe': eq_result.Sharpe_ratio,
        }
    except Exception as e:
        results['equal_vote'] = {'error': str(e)}

    # PF-weighted voting
    try:
        total_pf = sum(strat_pfs)
        pf_weights = [pf / total_pf for pf in strat_pfs]
        pf_entry, pf_exit = ensemble_vote(strat_funcs, test_df, weights=pf_weights)
        bt_pf = Backtester(test_df, fee=0.001, slippage=0.0005)
        pf_result = bt_pf.run(pf_entry, pf_exit, stop_loss=0.003)
        results['pf_weighted_vote'] = {
            'trades': pf_result.total_trades,
            'win_rate': pf_result.win_rate,
            'profit_factor': pf_result.profit_factor,
            'sharpe': pf_result.Sharpe_ratio,
        }
    except Exception as e:
        results['pf_weighted_vote'] = {'error': str(e)}

    # Stacking
    try:
        target = (df['close'].shift(-1) > df['close']).astype(int)
        stack_entry, stack_exit = ensemble_stacking(
            strat_funcs, test_df, target.reindex(test_df.index),
            train_ratio=0.5, meta_learner='logistic'
        )
        bt_stack = Backtester(test_df, fee=0.001, slippage=0.0005)
        stack_result = bt_stack.run(stack_entry, stack_exit, stop_loss=0.003)
        results['stacking'] = {
            'trades': stack_result.total_trades,
            'win_rate': stack_result.win_rate,
            'profit_factor': stack_result.profit_factor,
            'sharpe': stack_result.Sharpe_ratio,
        }
    except Exception as e:
        results['stacking'] = {'error': str(e)}

    # Determine best method
    methods = {}
    for key in ['best_single', 'equal_vote', 'pf_weighted_vote', 'stacking']:
        if key in results and 'profit_factor' in results[key]:
            methods[key] = results[key]['profit_factor']

    results['best_method'] = max(methods, key=methods.get) if methods else 'best_single'
    results['ensemble_helps'] = results['best_method'] != 'best_single'

    return results


def main():
    print('=' * 60)
    print('RUN24.1 — Ensemble Strategy Testing')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Ensemble'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = process_coin(coin)
            results[coin] = result
            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)

            if 'best_single' in result and 'trades' in result['best_single']:
                bs = result['best_single']
                print(f'  Best single: {bs["strategy"]} PF={bs["profit_factor"]:.2f}')
                for method in ['equal_vote', 'pf_weighted_vote', 'stacking']:
                    if method in result and 'profit_factor' in result[method]:
                        print(f'  {method}: PF={result[method]["profit_factor"]:.2f}')
                print(f'  Winner: {result["best_method"]}')
        except Exception as e:
            print(f'  FAILED: {e}')

    if len(completed) == len(COINS):
        valid = {k: v for k, v in results.items() if 'ensemble_helps' in v}
        helped = sum(1 for v in valid.values() if v['ensemble_helps'])

        final = {
            'per_coin': results,
            'summary': {
                'coins_tested': len(valid),
                'ensemble_helps': helped,
                'best_method_counts': {},
            }
        }

        method_counts = {}
        for v in valid.values():
            m = v.get('best_method', 'unknown')
            method_counts[m] = method_counts.get(m, 0) + 1
        final['summary']['best_method_counts'] = method_counts

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Ensemble helps in {helped}/{len(valid)} coins')
        print(f'Method wins: {method_counts}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)


if __name__ == '__main__':
    main()
