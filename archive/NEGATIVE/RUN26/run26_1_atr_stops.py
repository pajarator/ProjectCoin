"""
RUN26.1 — ATR-Based Dynamic Stops Grid Search

Grid search:
  ATR mult: [0.5, 0.75, 1.0, 1.5, 2.0]
  ATR period: [7, 14, 21]
  With/without trailing
  Trailing activation: [0.3%, 0.5%, 0.8%]
  Trailing distance: [0.3%, 0.5%, 0.75%, 1.0%]

Compare to current fixed 0.3% SL.

Output: run26_1_results.json
Checkpoint: run26_1_checkpoint.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm
from itertools import product

from feature_engine import load_cached_data, COINS
from indicators import add_all_indicators, ATR
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
from backtester import Backtester

CHECKPOINT_FILE = 'run26_1_checkpoint.json'
RESULTS_FILE = 'run26_1_results.json'
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}

# Grid parameters
ATR_MULTS = [0.5, 0.75, 1.0, 1.5, 2.0]
ATR_PERIODS = [7, 14, 21]
TRAILING_PCTS = [None, 0.003, 0.005, 0.0075, 0.01]
TRAILING_ACTIVATIONS = [None, 0.003, 0.005, 0.008]

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


def find_best_strategy(df_ind: pd.DataFrame) -> tuple:
    """Find best strategy by win rate."""
    best_name = None
    best_wr = 0
    best_entry = None
    best_exit = None

    for name, func in ALL_STRATEGIES.items():
        try:
            entry, exit_sig = func(df_ind)
            bt = Backtester(df_ind, fee=0.001, slippage=0.0005)
            result = bt.run(entry, exit_sig, stop_loss=0.003)
            if result.total_trades >= 10 and result.win_rate > best_wr:
                best_wr = result.win_rate
                best_name = name
                best_entry = entry
                best_exit = exit_sig
        except Exception:
            continue

    return best_name, best_entry, best_exit


def process_coin(coin: str) -> dict:
    """Grid search stop variants for one coin."""
    df = load_cached_data(coin)
    df_ind = add_all_indicators(df.copy())

    strat_name, entry, exit_sig = find_best_strategy(df_ind)
    if strat_name is None:
        return None

    # Baseline: fixed 0.3% SL
    bt_base = Backtester(df_ind, fee=0.001, slippage=0.0005)
    base_result = bt_base.run(entry, exit_sig, stop_loss=0.003)

    baseline = {
        'trades': base_result.total_trades,
        'win_rate': base_result.win_rate,
        'profit_factor': base_result.profit_factor,
        'total_pnl_pct': base_result.total_pnl_pct,
        'max_drawdown': base_result.max_drawdown,
        'sharpe': base_result.Sharpe_ratio,
    }

    # Pre-compute ATR for different periods
    atr_cache = {}
    for period in ATR_PERIODS:
        atr_cache[period] = ATR(df_ind['high'], df_ind['low'], df_ind['close'], period)

    # Grid search
    grid_results = []
    combos = list(product(ATR_MULTS, ATR_PERIODS, TRAILING_PCTS, TRAILING_ACTIVATIONS))

    # Filter out invalid combos (activation without trailing)
    combos = [(m, p, t, a) for m, p, t, a in combos
              if not (t is None and a is not None)]

    for atr_mult, atr_period, trail_pct, trail_act in combos:
        df_test = df_ind.copy()
        df_test['ATR'] = atr_cache[atr_period]

        bt = Backtester(df_test, fee=0.001, slippage=0.0005)
        result = bt.run(
            entry, exit_sig,
            atr_stop_mult=atr_mult,
            trailing_stop_pct=trail_pct,
            trailing_activation_pct=trail_act,
        )

        if result.total_trades >= 5:
            grid_results.append({
                'atr_mult': atr_mult,
                'atr_period': atr_period,
                'trailing_pct': trail_pct,
                'trailing_activation': trail_act,
                'trades': result.total_trades,
                'win_rate': result.win_rate,
                'profit_factor': result.profit_factor,
                'total_pnl_pct': result.total_pnl_pct,
                'max_drawdown': result.max_drawdown,
                'sharpe': result.Sharpe_ratio,
            })

    # Sort by risk-adjusted metric: PF * sqrt(trades) / max(DD, 1)
    for r in grid_results:
        r['score'] = r['profit_factor'] * np.sqrt(r['trades']) / max(r['max_drawdown'], 1)

    grid_results.sort(key=lambda x: -x['score'])

    best = grid_results[0] if grid_results else None
    improvement = None
    if best:
        improvement = {
            'wr_delta': best['win_rate'] - baseline['win_rate'],
            'pf_delta': best['profit_factor'] - baseline['profit_factor'],
            'pnl_delta': best['total_pnl_pct'] - baseline['total_pnl_pct'],
            'dd_delta': best['max_drawdown'] - baseline['max_drawdown'],
        }

    return {
        'coin': coin,
        'strategy': strat_name,
        'baseline': baseline,
        'best': best,
        'improvement': improvement,
        'top_5': grid_results[:5],
        'total_combos_tested': len(grid_results),
    }


def main():
    print('=' * 60)
    print('RUN26.1 — ATR-Based Dynamic Stops')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='ATR stops'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = process_coin(coin)
            if result:
                results[coin] = result
                b = result['baseline']
                if result['best']:
                    best = result['best']
                    imp = result['improvement']
                    print(f'  Baseline: WR={b["win_rate"]:.1f}% PF={b["profit_factor"]:.2f} DD={b["max_drawdown"]:.1f}%')
                    print(f'  Best ATR: mult={best["atr_mult"]} period={best["atr_period"]} '
                          f'trail={best["trailing_pct"]} act={best["trailing_activation"]}')
                    print(f'  Improvement: WR{imp["wr_delta"]:+.1f}% PF{imp["pf_delta"]:+.2f} DD{imp["dd_delta"]:+.1f}%')
                else:
                    print(f'  No improvement over baseline')
            else:
                print(f'  No valid strategy found')

            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)
        except Exception as e:
            print(f'  FAILED: {e}')
            import traceback; traceback.print_exc()

    if len(completed) == len(COINS):
        # Summary
        improved = sum(1 for r in results.values()
                       if r.get('improvement') and r['improvement']['pf_delta'] > 0)

        final = {
            'per_coin': results,
            'summary': {
                'coins_improved': improved,
                'coins_tested': len(results),
                'avg_pf_delta': float(np.mean([
                    r['improvement']['pf_delta'] for r in results.values()
                    if r.get('improvement')
                ])),
                'avg_dd_delta': float(np.mean([
                    r['improvement']['dd_delta'] for r in results.values()
                    if r.get('improvement')
                ])),
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Coins improved: {improved}/{len(results)}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(completed)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
