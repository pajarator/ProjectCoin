"""
RUN16.1 — ML Feature Importance Discovery

For each of 19 coins:
  1. Load 1-year 15m data → build feature matrix
  2. Train/test split (9mo/3mo)
  3. Fit RandomForest + XGBoost classifiers
  4. Extract Gini importance + permutation importance
  5. Find universal features (top-20 in >50% of coins)

Output: run16_1_results.json
Checkpoint: run16_1_checkpoint.json
"""
import json
import os
import signal
import sys
import numpy as np
import pandas as pd
from tqdm import tqdm
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import TimeSeriesSplit
from sklearn.preprocessing import StandardScaler
from sklearn.inspection import permutation_importance
from sklearn.metrics import accuracy_score, f1_score
import xgboost as xgb

from feature_engine import load_cached_data, build_feature_matrix, get_feature_columns, COINS

CHECKPOINT_FILE = 'run16_1_checkpoint.json'
RESULTS_FILE = 'run16_1_results.json'
WARMUP = 200

# Graceful shutdown
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


def process_coin(coin: str) -> dict:
    """Run feature importance analysis for a single coin."""
    df = load_cached_data(coin)
    features = build_feature_matrix(df, include_targets=True)

    # Drop warmup rows and rows where target is NaN
    features = features.iloc[WARMUP:]
    features = features.dropna(subset=['target_1bar'])

    feature_cols = get_feature_columns(include_time=True)
    # Drop any feature columns that don't exist or have all NaN
    feature_cols = [c for c in feature_cols if c in features.columns]
    X = features[feature_cols].copy()
    y = (features['target_1bar'] > 0).astype(int)  # binary: up=1, down=0

    # Fill remaining NaN with 0 (shouldn't happen after warmup, but safety)
    X = X.fillna(0)

    # Replace inf
    X = X.replace([np.inf, -np.inf], 0)

    # Train/test split: 9 months train, 3 months test (time-based)
    split_idx = int(len(X) * 0.75)
    X_train, X_test = X.iloc[:split_idx], X.iloc[split_idx:]
    y_train, y_test = y.iloc[:split_idx], y.iloc[split_idx:]

    # Scale features (fit on train only)
    scaler = StandardScaler()
    X_train_scaled = pd.DataFrame(scaler.fit_transform(X_train), columns=feature_cols, index=X_train.index)
    X_test_scaled = pd.DataFrame(scaler.transform(X_test), columns=feature_cols, index=X_test.index)

    result = {
        'coin': coin,
        'n_train': len(X_train),
        'n_test': len(X_test),
        'n_features': len(feature_cols),
    }

    # === RandomForest ===
    rf = RandomForestClassifier(
        n_estimators=200, max_depth=10, random_state=42,
        n_jobs=-1, min_samples_leaf=20
    )
    rf.fit(X_train_scaled, y_train)

    rf_pred = rf.predict(X_test_scaled)
    result['rf_accuracy'] = float(accuracy_score(y_test, rf_pred))
    result['rf_f1'] = float(f1_score(y_test, rf_pred, zero_division=0))

    # Gini importance
    gini_imp = pd.Series(rf.feature_importances_, index=feature_cols).sort_values(ascending=False)
    result['rf_gini_top20'] = {k: float(v) for k, v in gini_imp.head(20).items()}

    # Permutation importance (on test set)
    perm_imp = permutation_importance(rf, X_test_scaled, y_test, n_repeats=10, random_state=42, n_jobs=-1)
    perm_series = pd.Series(perm_imp.importances_mean, index=feature_cols).sort_values(ascending=False)
    result['rf_perm_top20'] = {k: float(v) for k, v in perm_series.head(20).items()}

    # === XGBoost ===
    xgb_clf = xgb.XGBClassifier(
        n_estimators=200, max_depth=6, learning_rate=0.1,
        random_state=42, n_jobs=-1, eval_metric='logloss',
        min_child_weight=20
    )
    xgb_clf.fit(X_train_scaled, y_train, verbose=False)

    xgb_pred = xgb_clf.predict(X_test_scaled)
    result['xgb_accuracy'] = float(accuracy_score(y_test, xgb_pred))
    result['xgb_f1'] = float(f1_score(y_test, xgb_pred, zero_division=0))

    xgb_imp = pd.Series(xgb_clf.feature_importances_, index=feature_cols).sort_values(ascending=False)
    result['xgb_top20'] = {k: float(v) for k, v in xgb_imp.head(20).items()}

    # === Cross-validation (TimeSeriesSplit) ===
    tscv = TimeSeriesSplit(n_splits=5)
    cv_scores = []
    for train_idx, val_idx in tscv.split(X):
        X_cv_train = X.iloc[train_idx].fillna(0).replace([np.inf, -np.inf], 0)
        X_cv_val = X.iloc[val_idx].fillna(0).replace([np.inf, -np.inf], 0)
        y_cv_train = y.iloc[train_idx]
        y_cv_val = y.iloc[val_idx]

        sc = StandardScaler()
        X_cv_train_s = sc.fit_transform(X_cv_train)
        X_cv_val_s = sc.transform(X_cv_val)

        rf_cv = RandomForestClassifier(n_estimators=100, max_depth=10, random_state=42, n_jobs=-1, min_samples_leaf=20)
        rf_cv.fit(X_cv_train_s, y_cv_train)
        cv_scores.append(float(accuracy_score(y_cv_val, rf_cv.predict(X_cv_val_s))))

    result['cv_scores'] = cv_scores
    result['cv_mean'] = float(np.mean(cv_scores))
    result['cv_std'] = float(np.std(cv_scores))

    return result


def find_universal_features(all_results: dict) -> dict:
    """Find features that appear in top-20 for >50% of coins."""
    feature_counts = {}
    feature_scores = {}
    coins_processed = len(all_results)

    for coin, result in all_results.items():
        for source in ['rf_gini_top20', 'rf_perm_top20', 'xgb_top20']:
            if source in result:
                for feat, score in result[source].items():
                    if feat not in feature_counts:
                        feature_counts[feat] = set()
                        feature_scores[feat] = []
                    feature_counts[feat].add(coin)
                    feature_scores[feat].append(score)

    universal = {}
    threshold = coins_processed * 0.5
    for feat, coins in feature_counts.items():
        if len(coins) >= threshold:
            universal[feat] = {
                'count': len(coins),
                'pct_coins': len(coins) / coins_processed * 100,
                'avg_importance': float(np.mean(feature_scores[feat])),
                'coins': sorted(list(coins))
            }

    # Sort by count then avg importance
    universal = dict(sorted(universal.items(), key=lambda x: (-x[1]['count'], -x[1]['avg_importance'])))
    return universal


def main():
    print('=' * 60)
    print('RUN16.1 — ML Feature Importance Discovery')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'Coins: {len(COINS)} total, {len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Processing coins'):
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

            print(f'  RF acc={result["rf_accuracy"]:.3f}  XGB acc={result["xgb_accuracy"]:.3f}  CV={result["cv_mean"]:.3f}±{result["cv_std"]:.3f}')
            top3_rf = list(result['rf_gini_top20'].keys())[:3]
            print(f'  Top-3 RF Gini: {top3_rf}')
        except Exception as e:
            print(f'  FAILED: {e}')

    # Final analysis
    if len(results) == len(COINS):
        universal = find_universal_features(results)

        final = {
            'coins_processed': len(results),
            'per_coin': results,
            'universal_features': universal,
            'summary': {
                'avg_rf_accuracy': float(np.mean([r['rf_accuracy'] for r in results.values()])),
                'avg_xgb_accuracy': float(np.mean([r['xgb_accuracy'] for r in results.values()])),
                'avg_cv_mean': float(np.mean([r['cv_mean'] for r in results.values()])),
                'n_universal_features': len(universal),
                'universal_feature_names': list(universal.keys())
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS SAVED: {RESULTS_FILE}')
        print(f'{"=" * 60}')
        print(f'Avg RF accuracy:  {final["summary"]["avg_rf_accuracy"]:.3f}')
        print(f'Avg XGB accuracy: {final["summary"]["avg_xgb_accuracy"]:.3f}')
        print(f'Universal features ({len(universal)}):')
        for feat, info in list(universal.items())[:10]:
            print(f'  {feat:25s} — in {info["count"]}/{len(results)} coins ({info["pct_coins"]:.0f}%)')

        # Cleanup checkpoint
        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(results)}/{len(COINS)} coins done. Run again to resume.')


if __name__ == '__main__':
    main()
