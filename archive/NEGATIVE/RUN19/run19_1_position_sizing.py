"""
RUN19.1 — Position Sizing Comparison

Grid search: fixed fraction vs Kelly vs half-Kelly vs ATR-based vs fixed-dollar
across 19 coins using best strategy per coin.

Output: run19_1_results.json
Checkpoint: run19_1_checkpoint.json
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
from risk import kelly_criterion, half_kelly, optimal_f, atr_position_size, fixed_fraction_size

CHECKPOINT_FILE = 'run19_1_checkpoint.json'
RESULTS_FILE = 'run19_1_results.json'
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}

# Position sizing methods to test
SIZING_METHODS = {
    'fixed_1pct': 0.01,
    'fixed_2pct': 0.02,
    'fixed_5pct': 0.05,
}

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


def find_best_strategy(coin: str) -> tuple:
    """Find best strategy for a coin by win rate."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)

    best_name = None
    best_wr = 0
    best_result = None

    for name, func in ALL_STRATEGIES.items():
        try:
            entry, exit_sig = func(df)
            bt = Backtester(df, fee=0.001, slippage=0.0005)
            result = bt.run(entry, exit_sig, stop_loss=0.003)
            if result.total_trades >= 10 and result.win_rate > best_wr:
                best_wr = result.win_rate
                best_name = name
                best_result = result
        except Exception:
            continue

    return best_name, best_result


def simulate_sizing(trades_pnls: list, method: str, account: float = 10000) -> dict:
    """Simulate account growth with different position sizing."""
    pnls = np.array(trades_pnls)
    if len(pnls) < 5:
        return None

    wins = pnls[pnls > 0]
    losses = pnls[pnls <= 0]
    wr = len(wins) / len(pnls)
    avg_w = float(np.mean(wins)) if len(wins) > 0 else 0
    avg_l = float(np.mean(np.abs(losses))) if len(losses) > 0 else 1

    # Determine fraction based on method
    if method == 'kelly':
        frac = kelly_criterion(wr, avg_w, avg_l)
    elif method == 'half_kelly':
        frac = half_kelly(wr, avg_w, avg_l)
    elif method == 'optimal_f':
        frac = min(optimal_f(trades_pnls), 0.25)
    elif method.startswith('fixed_'):
        frac = SIZING_METHODS.get(method, 0.02)
    else:
        frac = 0.02

    if frac <= 0:
        frac = 0.01  # minimum

    # Simulate
    equity = account
    peak = equity
    max_dd = 0
    equity_curve = [equity]

    for pnl_pct in pnls:
        trade_size = equity * frac
        pnl_dollar = trade_size * pnl_pct / 100
        equity += pnl_dollar
        equity_curve.append(equity)
        if equity > peak:
            peak = equity
        dd = (peak - equity) / peak * 100 if peak > 0 else 0
        max_dd = max(max_dd, dd)

    total_return = (equity - account) / account * 100

    return {
        'method': method,
        'fraction': float(frac),
        'total_return_pct': float(total_return),
        'max_drawdown_pct': float(max_dd),
        'final_equity': float(equity),
        'sharpe': float(np.mean(np.diff(equity_curve) / np.array(equity_curve[:-1])) /
                       max(np.std(np.diff(equity_curve) / np.array(equity_curve[:-1])), 1e-10) *
                       np.sqrt(252)) if len(equity_curve) > 2 else 0,
    }


def process_coin(coin: str) -> dict:
    """Test all sizing methods on best strategy for a coin."""
    best_name, best_result = find_best_strategy(coin)
    if not best_result or best_result.total_trades < 10:
        return None

    pnls = [t.pnl_pct for t in best_result.trades if t.pnl_pct is not None]

    methods = list(SIZING_METHODS.keys()) + ['kelly', 'half_kelly', 'optimal_f']
    sizing_results = {}

    for method in methods:
        result = simulate_sizing(pnls, method)
        if result:
            sizing_results[method] = result

    # Find best method by risk-adjusted return (return / max_dd)
    best_method = max(sizing_results.items(),
                      key=lambda x: x[1]['total_return_pct'] / max(x[1]['max_drawdown_pct'], 0.1)
                      if x[1]['max_drawdown_pct'] < 50 else -999)

    return {
        'coin': coin,
        'best_strategy': best_name,
        'n_trades': len(pnls),
        'base_win_rate': best_result.win_rate,
        'sizing_results': sizing_results,
        'recommended': best_method[0],
        'recommended_return': best_method[1]['total_return_pct'],
        'recommended_dd': best_method[1]['max_drawdown_pct'],
    }


def main():
    print('=' * 60)
    print('RUN19.1 — Position Sizing Comparison')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Position sizing'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = process_coin(coin)
            if result:
                results[coin] = result
                print(f'  Strategy: {result["best_strategy"]} ({result["n_trades"]} trades)')
                print(f'  Recommended: {result["recommended"]} '
                      f'(return={result["recommended_return"]:.1f}%, DD={result["recommended_dd"]:.1f}%)')
            else:
                print('  No valid strategy found')

            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)
        except Exception as e:
            print(f'  FAILED: {e}')

    if len(completed) == len(COINS):
        # Summary
        method_wins = {}
        for r in results.values():
            m = r['recommended']
            method_wins[m] = method_wins.get(m, 0) + 1

        final = {
            'per_coin': results,
            'summary': {
                'method_recommendation_counts': method_wins,
                'best_overall': max(method_wins, key=method_wins.get) if method_wins else 'fixed_2pct',
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Method recommendations: {method_wins}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(completed)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
