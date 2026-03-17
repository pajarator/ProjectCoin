"""
RUN20.1 — Derivatives Features Analysis

Adds funding rate and OI features to the feature matrix.
Re-runs ML importance analysis.
Tests as filter on existing strategies.

Requires: fetch_derivatives.py to have been run first.

Output: run20_1_results.json
Checkpoint: run20_1_checkpoint.json
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

DERIV_DIR = 'data_cache/derivatives'
CHECKPOINT_FILE = 'run20_1_checkpoint.json'
RESULTS_FILE = 'run20_1_results.json'
WARMUP = 200

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


def load_derivatives(coin: str) -> tuple:
    """Load funding rate and OI data for a coin."""
    fr_path = os.path.join(DERIV_DIR, f'{coin}_USDT_funding.csv')
    oi_path = os.path.join(DERIV_DIR, f'{coin}_USDT_oi.csv')

    fr = None
    oi = None

    if os.path.exists(fr_path):
        fr = pd.read_csv(fr_path, index_col=0, parse_dates=True)
    if os.path.exists(oi_path):
        oi = pd.read_csv(oi_path, index_col=0, parse_dates=True)

    return fr, oi


def add_derivatives_features(features: pd.DataFrame, fr: pd.DataFrame,
                               oi: pd.DataFrame) -> pd.DataFrame:
    """Add derivatives-based features to feature matrix."""
    feat = features.copy()

    if fr is not None and 'funding_rate' in fr.columns and len(fr) > 0:
        # Resample funding rate to match features index (forward-fill 8h -> 15m)
        fr_resampled = fr['funding_rate'].resample('15min').ffill()
        fr_aligned = fr_resampled.reindex(feat.index, method='ffill')

        feat['funding_rate'] = fr_aligned
        feat['funding_zscore'] = (fr_aligned - fr_aligned.rolling(30 * 3).mean()) / \
                                  fr_aligned.rolling(30 * 3).std().replace(0, np.nan)
        feat['funding_extreme'] = (abs(feat['funding_zscore']) > 2).astype(int)
        feat['funding_positive'] = (fr_aligned > 0).astype(int)

    if oi is not None and 'open_interest' in oi.columns and len(oi) > 0:
        oi_resampled = oi['open_interest'].resample('15min').ffill()
        oi_aligned = oi_resampled.reindex(feat.index, method='ffill')

        feat['oi_change_pct'] = oi_aligned.pct_change(periods=96)  # 1-day change
        # OI divergence: price up + OI down = bearish divergence
        if 'returns_1' in feat.columns:
            price_up = feat['returns_1'].rolling(96).sum() > 0
            oi_down = feat['oi_change_pct'] < 0
            feat['oi_divergence'] = (price_up & oi_down).astype(int)

    return feat


def process_coin(coin: str) -> dict:
    """Analyze derivatives features for one coin."""
    df = load_cached_data(coin)
    features = build_feature_matrix(df, include_targets=True)
    fr, oi = load_derivatives(coin)

    has_fr = fr is not None and len(fr) > 0
    has_oi = oi is not None and len(oi) > 0

    if not has_fr and not has_oi:
        return {'coin': coin, 'status': 'no_derivatives_data'}

    # Add derivatives features
    feat_ext = add_derivatives_features(features, fr, oi)
    feat_ext = feat_ext.iloc[WARMUP:]
    feat_ext = feat_ext.dropna(subset=['target_1bar'])

    # Get all feature columns including new ones
    base_cols = get_feature_columns(include_time=True)
    deriv_cols = [c for c in ['funding_rate', 'funding_zscore', 'funding_extreme',
                               'funding_positive', 'oi_change_pct', 'oi_divergence']
                  if c in feat_ext.columns]
    all_cols = base_cols + deriv_cols
    valid_cols = [c for c in all_cols if c in feat_ext.columns]

    X = feat_ext[valid_cols].fillna(0).replace([np.inf, -np.inf], 0)
    y = (feat_ext['target_1bar'] > 0).astype(int)

    split_idx = int(len(X) * 0.75)
    X_train, X_test = X.iloc[:split_idx], X.iloc[split_idx:]
    y_train, y_test = y.iloc[:split_idx], y.iloc[split_idx:]

    scaler = StandardScaler()
    X_train_s = pd.DataFrame(scaler.fit_transform(X_train), columns=valid_cols, index=X_train.index)
    X_test_s = pd.DataFrame(scaler.transform(X_test), columns=valid_cols, index=X_test.index)

    # RF with all features
    rf = RandomForestClassifier(n_estimators=200, max_depth=10, random_state=42,
                                 n_jobs=-1, min_samples_leaf=20)
    rf.fit(X_train_s, y_train)

    imp = pd.Series(rf.feature_importances_, index=valid_cols).sort_values(ascending=False)

    # Check where derivatives features rank
    deriv_ranks = {}
    for dc in deriv_cols:
        if dc in imp.index:
            rank = list(imp.index).index(dc) + 1
            deriv_ranks[dc] = {'rank': rank, 'importance': float(imp[dc])}

    from sklearn.metrics import accuracy_score
    acc_full = float(accuracy_score(y_test, rf.predict(X_test_s)))

    # RF without derivatives features
    base_valid = [c for c in base_cols if c in feat_ext.columns]
    X_train_base = X_train[base_valid]
    X_test_base = X_test[base_valid]
    scaler2 = StandardScaler()
    X_train_base_s = scaler2.fit_transform(X_train_base)
    X_test_base_s = scaler2.transform(X_test_base)

    rf_base = RandomForestClassifier(n_estimators=200, max_depth=10, random_state=42,
                                      n_jobs=-1, min_samples_leaf=20)
    rf_base.fit(X_train_base_s, y_train)
    acc_base = float(accuracy_score(y_test, rf_base.predict(X_test_base_s)))

    return {
        'coin': coin,
        'has_funding': has_fr,
        'has_oi': has_oi,
        'funding_records': len(fr) if has_fr else 0,
        'oi_records': len(oi) if has_oi else 0,
        'deriv_feature_ranks': deriv_ranks,
        'accuracy_with_deriv': acc_full,
        'accuracy_without_deriv': acc_base,
        'accuracy_delta': acc_full - acc_base,
        'top_10_features': {k: float(v) for k, v in imp.head(10).items()},
    }


def main():
    print('=' * 60)
    print('RUN20.1 — Derivatives Features Analysis')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Derivatives'):
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

            if 'accuracy_delta' in result:
                print(f'  Acc with deriv: {result["accuracy_with_deriv"]:.3f}, '
                      f'without: {result["accuracy_without_deriv"]:.3f}, '
                      f'delta: {result["accuracy_delta"]:+.3f}')
                if result['deriv_feature_ranks']:
                    for f_name, f_info in result['deriv_feature_ranks'].items():
                        print(f'  {f_name}: rank {f_info["rank"]}')
            else:
                print(f'  {result.get("status", "unknown")}')
        except Exception as e:
            print(f'  FAILED: {e}')

    if len(completed) == len(COINS):
        valid = [r for r in results.values() if 'accuracy_delta' in r]
        avg_delta = np.mean([r['accuracy_delta'] for r in valid]) if valid else 0

        # Check if any deriv feature lands in top-20
        deriv_in_top20 = sum(1 for r in valid
                              for f_info in r.get('deriv_feature_ranks', {}).values()
                              if f_info['rank'] <= 20)

        final = {
            'per_coin': results,
            'summary': {
                'coins_with_data': len(valid),
                'avg_accuracy_delta': float(avg_delta),
                'deriv_features_in_top20': deriv_in_top20,
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Avg accuracy delta: {avg_delta:+.3f}')
        print(f'Deriv features in top-20: {deriv_in_top20}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)


if __name__ == '__main__':
    main()
