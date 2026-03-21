"""
RUN16.3 — COINCLAW vs ML Comparison

Side-by-side comparison:
  1. Current COINCLAW strategy signals vs ML model signals
  2. ML as secondary filter (gate) on existing strategy entries
  3. Per-coin breakdown of where ML helps

Output: run16_3_results.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm
from sklearn.ensemble import RandomForestClassifier
from sklearn.preprocessing import StandardScaler

from feature_engine import load_cached_data, build_feature_matrix, get_feature_columns, COINS
from indicators import add_all_indicators
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
from backtester import Backtester

RESULTS_16_1 = 'run16_1_results.json'
CHECKPOINT_FILE = 'run16_3_checkpoint.json'
RESULTS_FILE = 'run16_3_results.json'
WARMUP = 200
PROB_THRESHOLD = 0.55  # lower threshold for gating

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


def get_top_features() -> list:
    """Load top features from RUN16.1."""
    if os.path.exists(RESULTS_16_1):
        with open(RESULTS_16_1, 'r') as f:
            data = json.load(f)
        universal = data.get('universal_features', {})
        if universal:
            return list(universal.keys())[:20]
    return get_feature_columns(include_time=True)


def compare_coin(coin: str, feature_cols: list) -> dict:
    """Compare COINCLAW strategies vs ML signals for one coin."""
    df = load_cached_data(coin)
    df_ind = add_all_indicators(df.copy())
    features = build_feature_matrix(df, include_targets=True)

    # Use last 25% as test (same as walk-forward test period)
    split_idx = int(len(df) * 0.75)
    train_df = df.iloc[WARMUP:split_idx]
    test_df = df.iloc[split_idx:]

    features_clean = features.iloc[WARMUP:].dropna(subset=['target_1bar'])
    valid_cols = [c for c in feature_cols if c in features_clean.columns]
    X = features_clean[valid_cols].fillna(0).replace([np.inf, -np.inf], 0)
    y = (features_clean['target_1bar'] > 0).astype(int)

    X_train = X.iloc[:split_idx - WARMUP]
    y_train = y.iloc[:split_idx - WARMUP]
    X_test = X.iloc[split_idx - WARMUP:]

    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X_train)
    X_test_s = scaler.transform(X_test)

    rf = RandomForestClassifier(
        n_estimators=200, max_depth=10, random_state=42,
        n_jobs=-1, min_samples_leaf=20
    )
    rf.fit(X_train_s, y_train)
    ml_proba = rf.predict_proba(X_test_s)[:, 1]
    ml_proba_series = pd.Series(ml_proba, index=X_test.index)

    results = {'coin': coin, 'strategies': {}}

    # Test each strategy on test period
    test_df_ind = add_all_indicators(test_df.copy())

    for strat_name, strat_func in ALL_STRATEGIES.items():
        try:
            entry, exit_sig = strat_func(test_df_ind)
            # Reindex to match test_df
            entry = entry.reindex(test_df.index).fillna(False)
            exit_sig = exit_sig.reindex(test_df.index).fillna(False)

            if entry.sum() < 3:
                continue

            # 1. Strategy alone
            bt = Backtester(test_df, fee=0.001, slippage=0.0005)
            strat_result = bt.run(entry, exit_sig, stop_loss=0.003)

            # 2. ML alone (>60% prob)
            ml_entry = pd.Series(False, index=test_df.index)
            ml_exit = pd.Series(False, index=test_df.index)
            common_idx = ml_proba_series.index.intersection(test_df.index)
            if len(common_idx) > 0:
                ml_entry.loc[common_idx] = ml_proba_series.loc[common_idx] > 0.60
                ml_exit.loc[common_idx] = ml_proba_series.loc[common_idx] < 0.45

            # 3. Strategy + ML gate (entry only when both agree)
            gated_entry = entry & False  # start with all False
            common_idx2 = ml_proba_series.index.intersection(entry.index)
            if len(common_idx2) > 0:
                ml_gate = ml_proba_series.reindex(entry.index).fillna(0.5) > PROB_THRESHOLD
                gated_entry = entry & ml_gate

            if gated_entry.sum() >= 1:
                bt_gated = Backtester(test_df, fee=0.001, slippage=0.0005)
                gated_result = bt_gated.run(gated_entry, exit_sig, stop_loss=0.003)
            else:
                gated_result = None

            results['strategies'][strat_name] = {
                'strategy_alone': {
                    'trades': strat_result.total_trades,
                    'win_rate': strat_result.win_rate,
                    'profit_factor': strat_result.profit_factor,
                    'sharpe': strat_result.Sharpe_ratio,
                },
                'ml_gated': {
                    'trades': gated_result.total_trades if gated_result else 0,
                    'win_rate': gated_result.win_rate if gated_result else 0,
                    'profit_factor': gated_result.profit_factor if gated_result else 0,
                    'sharpe': gated_result.Sharpe_ratio if gated_result else 0,
                } if gated_result else None,
                'ml_helped': (
                    gated_result is not None and
                    gated_result.win_rate > strat_result.win_rate and
                    gated_result.total_trades >= 3
                ),
            }
        except Exception:
            continue

    # Count improvements
    helped = sum(1 for s in results['strategies'].values() if s.get('ml_helped'))
    total = len(results['strategies'])
    results['ml_helped_count'] = helped
    results['strategies_tested'] = total
    results['ml_help_pct'] = (helped / total * 100) if total > 0 else 0

    return results


def main():
    print('=' * 60)
    print('RUN16.3 — COINCLAW vs ML Comparison')
    print('=' * 60)

    feature_cols = get_top_features()
    print(f'Using {len(feature_cols)} features\n')

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Comparing'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = compare_coin(coin, feature_cols)
            results[coin] = result
            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)

            print(f'  Tested {result["strategies_tested"]} strategies, '
                  f'ML helped in {result["ml_helped_count"]} ({result["ml_help_pct"]:.0f}%)')
        except Exception as e:
            print(f'  FAILED: {e}')

    if len(results) == len(COINS):
        avg_help = np.mean([r['ml_help_pct'] for r in results.values()])

        final = {
            'per_coin': results,
            'summary': {
                'avg_ml_help_pct': float(avg_help),
                'coins_where_ml_helps_majority': sum(1 for r in results.values() if r['ml_help_pct'] > 50),
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Avg ML help: {avg_help:.1f}% of strategies improved')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(results)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
