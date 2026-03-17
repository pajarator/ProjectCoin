"""
RUN17.1 — Monte Carlo Validation of COINCLAW v13 Strategies

Applies MC simulation to all current strategies across 19 coins.
Reports 5th percentile outcomes (worst realistic case).
Flags strategies where 5th-percentile PF < 1.0.

Output: run17_1_results.json
Checkpoint: run17_1_checkpoint.json
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
from monte_carlo import monte_carlo_trades, confidence_interval

CHECKPOINT_FILE = 'run17_1_checkpoint.json'
RESULTS_FILE = 'run17_1_results.json'
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
    return {'completed': [], 'results': []}


def save_checkpoint(state):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(state, f, indent=2, default=str)


def validate_strategy(coin: str, strat_name: str, strat_func) -> dict:
    """Run MC validation for one coin/strategy combo."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)

    try:
        entry, exit_sig = strat_func(df)
    except Exception:
        return None

    bt = Backtester(df, fee=0.001, slippage=0.0005)
    result = bt.run(entry, exit_sig, stop_loss=0.003)

    if result.total_trades < 5:
        return None

    pnls = [t.pnl_pct for t in result.trades if t.pnl_pct is not None]
    if len(pnls) < 5:
        return None

    mc = monte_carlo_trades(pnls, n_simulations=10000)
    ci = confidence_interval(pnls, confidence=0.95)

    return {
        'coin': coin,
        'strategy': strat_name,
        'actual': {
            'trades': result.total_trades,
            'win_rate': result.win_rate,
            'profit_factor': result.profit_factor,
            'sharpe': result.Sharpe_ratio,
            'max_drawdown': result.max_drawdown,
        },
        'mc': {
            'p5_return': mc['return_percentiles']['p5'],
            'p50_return': mc['return_percentiles']['p50'],
            'p95_return': mc['return_percentiles']['p95'],
            'p5_drawdown': mc['drawdown_percentiles']['p5'],
            'p95_drawdown': mc['drawdown_percentiles']['p95'],
            'prob_profit': mc['prob_profit'],
        },
        'ci_95': {
            'win_rate': ci['win_rate'],
            'profit_factor': ci['profit_factor'],
            'sharpe': ci['sharpe'],
        },
        'flagged': ci['profit_factor']['lower'] < 1.0,
    }


def main():
    print('=' * 60)
    print('RUN17.1 — Monte Carlo Validation')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed'])
    all_results = state['results']

    combos = [(coin, name) for coin in COINS for name in ALL_STRATEGIES]
    remaining = [(c, n) for c, n in combos if f'{c}_{n}' not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin, strat_name in tqdm(remaining, desc='MC validation'):
        if shutdown_requested:
            break

        key = f'{coin}_{strat_name}'
        result = validate_strategy(coin, strat_name, ALL_STRATEGIES[strat_name])

        if result:
            all_results.append(result)
            flag = ' ⚠️ FLAGGED' if result['flagged'] else ''
            if result['actual']['trades'] >= 20:
                tqdm.write(f'  {coin}/{strat_name}: {result["actual"]["trades"]}t '
                          f'WR={result["actual"]["win_rate"]:.1f}% '
                          f'PF={result["actual"]["profit_factor"]:.2f} '
                          f'MC-p5={result["mc"]["p5_return"]:.1f}%{flag}')

        completed.add(key)
        state['completed'] = sorted(list(completed))
        state['results'] = all_results
        # Save every 10
        if len(completed) % 10 == 0:
            save_checkpoint(state)

    save_checkpoint(state)

    if len(completed) == len(combos):
        # Summarize
        valid_results = [r for r in all_results if r is not None]
        flagged = [r for r in valid_results if r['flagged']]

        summary = {
            'total_combos': len(combos),
            'valid_combos': len(valid_results),
            'flagged_count': len(flagged),
            'flagged_strategies': [
                {'coin': r['coin'], 'strategy': r['strategy'],
                 'pf_lower': r['ci_95']['profit_factor']['lower'],
                 'actual_pf': r['actual']['profit_factor']}
                for r in flagged
            ],
            'avg_prob_profit': float(np.mean([r['mc']['prob_profit'] for r in valid_results])),
        }

        final = {
            'results': valid_results,
            'summary': summary,
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Valid combos: {len(valid_results)}')
        print(f'Flagged (95% CI PF < 1.0): {len(flagged)}')
        print(f'Avg prob profit: {summary["avg_prob_profit"]:.1%}')

        if flagged:
            print(f'\nFlagged strategies:')
            for f_item in flagged[:20]:
                print(f'  {f_item["coin"]}/{f_item["strategy"]}: '
                      f'PF={f_item["actual"]["profit_factor"]:.2f}, '
                      f'CI lower={f_item["ci_95"]["profit_factor"]["lower"]:.2f}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(completed)}/{len(combos)} done. Run again to resume.')


if __name__ == '__main__':
    main()
