"""
RUN16.2 — Walk-Forward Validation of ML Feature Importance

Takes top-20 universal features from run16_1_results.json.
3-window walk-forward (train 4mo, test 2mo).
Generates buy signals at >60% model probability → backtest via Backtester.
Compares win rates vs baseline (random / always-buy).

Output: run16_2_results.json
Checkpoint: run16_2_checkpoint.json
"""
import json
import os
import signal
import sys
import numpy as np
import pandas as pd
from tqdm import tqdm
from sklearn.ensemble import RandomForestClassifier
from sklearn.preprocessing import StandardScaler
import xgboost as xgb

from feature_engine import load_cached_data, build_feature_matrix, get_feature_columns, COINS
from backtester import Backtester

RESULTS_16_1 = 'run16_1_results.json'
CHECKPOINT_FILE = 'run16_2_checkpoint.json'
RESULTS_FILE = 'run16_2_results.json'
WARMUP = 200
PROB_THRESHOLD = 0.60

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


def get_universal_features() -> list:
    """Load top-20 universal features from RUN16.1 results."""
    if not os.path.exists(RESULTS_16_1):
        # Fallback: use all features if 16.1 hasn't been run yet
        print('WARNING: run16_1_results.json not found, using all features')
        return get_feature_columns(include_time=True)

    with open(RESULTS_16_1, 'r') as f:
        data = json.load(f)

    universal = data.get('universal_features', {})
    if not universal:
        return get_feature_columns(include_time=True)

    # Return top 20 by count then importance
    return list(universal.keys())[:20]


def walk_forward_coin(coin: str, feature_cols: list) -> dict:
    """Run 3-window walk-forward for a single coin."""
    df = load_cached_data(coin)
    features = build_feature_matrix(df, include_targets=True)
    features = features.iloc[WARMUP:]
    features = features.dropna(subset=['target_1bar'])

    # Ensure feature columns exist
    valid_cols = [c for c in feature_cols if c in features.columns]
    X = features[valid_cols].fillna(0).replace([np.inf, -np.inf], 0)
    y = (features['target_1bar'] > 0).astype(int)

    n = len(X)
    # 3 walk-forward windows: each train=4mo (~11520 bars), test=2mo (~5760 bars)
    window_size = n // 3
    train_size = int(window_size * 0.67)
    test_size = window_size - train_size

    windows_results = []

    for w in range(3):
        start = w * window_size
        train_end = start + train_size
        test_end = min(start + window_size, n)

        if test_end <= train_end:
            continue

        X_train = X.iloc[start:train_end]
        y_train = y.iloc[start:train_end]
        X_test = X.iloc[train_end:test_end]
        y_test = y.iloc[train_end:test_end]

        scaler = StandardScaler()
        X_train_s = scaler.fit_transform(X_train)
        X_test_s = scaler.transform(X_test)

        # RF model
        rf = RandomForestClassifier(
            n_estimators=200, max_depth=10, random_state=42,
            n_jobs=-1, min_samples_leaf=20
        )
        rf.fit(X_train_s, y_train)
        rf_proba = rf.predict_proba(X_test_s)[:, 1]

        # XGB model
        xgb_clf = xgb.XGBClassifier(
            n_estimators=200, max_depth=6, learning_rate=0.1,
            random_state=42, n_jobs=-1, eval_metric='logloss',
            min_child_weight=20
        )
        xgb_clf.fit(X_train_s, y_train, verbose=False)
        xgb_proba = xgb_clf.predict_proba(X_test_s)[:, 1]

        # Generate signals at >60% probability
        test_df = df.iloc[WARMUP:].iloc[train_end:test_end].copy()

        for model_name, proba in [('rf', rf_proba), ('xgb', xgb_proba)]:
            entry = pd.Series(proba > PROB_THRESHOLD, index=test_df.index)
            # Exit: when probability drops below 0.45
            exit_sig = pd.Series(proba < 0.45, index=test_df.index)

            bt = Backtester(test_df, fee=0.001, slippage=0.0005)
            result = bt.run(entry, exit_sig, direction='long', stop_loss=0.003)

            windows_results.append({
                'window': w,
                'model': model_name,
                'train_start': str(X_train.index[0]),
                'test_start': str(X_test.index[0]),
                'test_end': str(X_test.index[-1]),
                'n_train': len(X_train),
                'n_test': len(X_test),
                'total_trades': result.total_trades,
                'win_rate': result.win_rate,
                'profit_factor': result.profit_factor,
                'max_drawdown': result.max_drawdown,
                'sharpe': result.Sharpe_ratio,
                'avg_win': result.avg_win,
                'avg_loss': result.avg_loss,
                'signal_rate': float(entry.sum() / len(entry)),
            })

    # Baseline: buy every bar
    test_df_full = df.iloc[WARMUP:].iloc[-test_size:].copy()
    entry_baseline = pd.Series(True, index=test_df_full.index)
    # Exit after 5 bars
    exit_baseline = pd.Series(False, index=test_df_full.index)
    exit_baseline.iloc[::5] = True

    bt_base = Backtester(test_df_full, fee=0.001, slippage=0.0005)
    base_result = bt_base.run(entry_baseline, exit_baseline, direction='long', stop_loss=0.003)

    return {
        'coin': coin,
        'n_features': len(valid_cols),
        'windows': windows_results,
        'baseline': {
            'total_trades': base_result.total_trades,
            'win_rate': base_result.win_rate,
            'profit_factor': base_result.profit_factor,
        }
    }


def main():
    print('=' * 60)
    print('RUN16.2 — Walk-Forward Validation')
    print('=' * 60)

    feature_cols = get_universal_features()
    print(f'Using {len(feature_cols)} features: {feature_cols[:5]}...\n')

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Walk-forward'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = walk_forward_coin(coin, feature_cols)
            results[coin] = result
            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)

            # Print summary
            for w in result['windows']:
                print(f'  W{w["window"]} {w["model"]}: {w["total_trades"]} trades, '
                      f'WR={w["win_rate"]:.1f}%, PF={w["profit_factor"]:.2f}')
            print(f'  Baseline: WR={result["baseline"]["win_rate"]:.1f}%')
        except Exception as e:
            print(f'  FAILED: {e}')
            import traceback; traceback.print_exc()

    if len(results) == len(COINS):
        # Aggregate
        rf_wrs = []
        xgb_wrs = []
        base_wrs = []
        for coin, r in results.items():
            for w in r['windows']:
                if w['model'] == 'rf':
                    rf_wrs.append(w['win_rate'])
                else:
                    xgb_wrs.append(w['win_rate'])
            base_wrs.append(r['baseline']['win_rate'])

        final = {
            'feature_cols': feature_cols,
            'n_features': len(feature_cols),
            'per_coin': results,
            'summary': {
                'rf_avg_wr': float(np.mean(rf_wrs)) if rf_wrs else 0,
                'xgb_avg_wr': float(np.mean(xgb_wrs)) if xgb_wrs else 0,
                'baseline_avg_wr': float(np.mean(base_wrs)) if base_wrs else 0,
                'rf_vs_baseline': float(np.mean(rf_wrs) - np.mean(base_wrs)) if rf_wrs and base_wrs else 0,
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'RF avg WR:       {final["summary"]["rf_avg_wr"]:.1f}%')
        print(f'XGB avg WR:      {final["summary"]["xgb_avg_wr"]:.1f}%')
        print(f'Baseline avg WR: {final["summary"]["baseline_avg_wr"]:.1f}%')
        print(f'RF vs baseline:  {final["summary"]["rf_vs_baseline"]:+.1f}%')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(results)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
