"""
RUN25.1 — ML-Based Regime Detection

1. Define 4 regimes: BULL_HIGH_VOL, BULL_LOW_VOL, BEAR_HIGH_VOL, BEAR_LOW_VOL
2. Label using BTC trend (SMA50 vs SMA200) + volatility quartile
3. Train RF/XGB classifier on features to predict regime
4. Compare accuracy vs current breadth-based method
5. Test: does ML regime → better strategy selection?

Output: run25_1_results.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm
from sklearn.ensemble import RandomForestClassifier
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import accuracy_score, classification_report
from sklearn.model_selection import TimeSeriesSplit
import xgboost as xgb

from feature_engine import load_cached_data, build_feature_matrix, get_feature_columns, COINS
from indicators import SMA, add_all_indicators
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
from backtester import Backtester

RESULTS_FILE = 'run25_1_results.json'
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}
WARMUP = 200

shutdown_requested = False
def signal_handler(sig, frame):
    global shutdown_requested
    shutdown_requested = True
signal.signal(signal.SIGINT, signal_handler)


def label_regimes(df: pd.DataFrame) -> pd.Series:
    """
    Label each bar with a market regime:
    0 = BEAR_LOW_VOL, 1 = BEAR_HIGH_VOL, 2 = BULL_LOW_VOL, 3 = BULL_HIGH_VOL
    """
    sma50 = SMA(df['close'], 50)
    sma200 = SMA(df['close'], 200)
    vol_20 = df['close'].pct_change().rolling(20).std()

    # Trend: bull if SMA50 > SMA200
    is_bull = sma50 > sma200

    # Volatility: high if above median
    vol_median = vol_20.rolling(500).median()
    is_high_vol = vol_20 > vol_median

    regime = pd.Series(0, index=df.index)
    regime[~is_bull & ~is_high_vol] = 0  # BEAR_LOW_VOL
    regime[~is_bull & is_high_vol] = 1   # BEAR_HIGH_VOL
    regime[is_bull & ~is_high_vol] = 2   # BULL_LOW_VOL
    regime[is_bull & is_high_vol] = 3    # BULL_HIGH_VOL

    return regime


REGIME_NAMES = {0: 'BEAR_LOW_VOL', 1: 'BEAR_HIGH_VOL', 2: 'BULL_LOW_VOL', 3: 'BULL_HIGH_VOL'}


def main():
    print('=' * 60)
    print('RUN25.1 — ML-Based Regime Detection')
    print('=' * 60)

    # Use BTC as primary regime reference
    btc_df = load_cached_data('BTC')
    btc_regimes = label_regimes(btc_df)

    # Build features for BTC
    features = build_feature_matrix(btc_df, include_targets=False)
    features = features.iloc[WARMUP:]
    regimes = btc_regimes.iloc[WARMUP:]

    feature_cols = get_feature_columns(include_time=True)
    valid_cols = [c for c in feature_cols if c in features.columns]
    X = features[valid_cols].fillna(0).replace([np.inf, -np.inf], 0)
    y = regimes.reindex(X.index)

    # Remove NaN targets
    valid_mask = ~y.isna()
    X = X[valid_mask]
    y = y[valid_mask].astype(int)

    print(f'\nBTC regime distribution:')
    for val, name in REGIME_NAMES.items():
        count = (y == val).sum()
        print(f'  {name}: {count} ({count / len(y) * 100:.1f}%)')

    # Train/test split
    split = int(len(X) * 0.75)
    X_train, X_test = X.iloc[:split], X.iloc[split:]
    y_train, y_test = y.iloc[:split], y.iloc[split:]

    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X_train)
    X_test_s = scaler.transform(X_test)

    # RF
    rf = RandomForestClassifier(n_estimators=200, max_depth=10, random_state=42, n_jobs=-1)
    rf.fit(X_train_s, y_train)
    rf_pred = rf.predict(X_test_s)
    rf_acc = accuracy_score(y_test, rf_pred)
    print(f'\nRF accuracy: {rf_acc:.3f}')

    # XGB
    xgb_clf = xgb.XGBClassifier(n_estimators=200, max_depth=6, learning_rate=0.1,
                                  random_state=42, n_jobs=-1, eval_metric='mlogloss')
    xgb_clf.fit(X_train_s, y_train, verbose=False)
    xgb_pred = xgb_clf.predict(X_test_s)
    xgb_acc = accuracy_score(y_test, xgb_pred)
    print(f'XGB accuracy: {xgb_acc:.3f}')

    # Cross-validation
    tscv = TimeSeriesSplit(n_splits=5)
    cv_scores = []
    for train_idx, val_idx in tscv.split(X):
        sc = StandardScaler()
        X_cv_train = sc.fit_transform(X.iloc[train_idx])
        X_cv_val = sc.transform(X.iloc[val_idx])
        rf_cv = RandomForestClassifier(n_estimators=100, max_depth=10, random_state=42, n_jobs=-1)
        rf_cv.fit(X_cv_train, y.iloc[train_idx])
        cv_scores.append(accuracy_score(y.iloc[val_idx], rf_cv.predict(X_cv_val)))
    print(f'CV accuracy: {np.mean(cv_scores):.3f} ± {np.std(cv_scores):.3f}')

    # Feature importance for regime prediction
    imp = pd.Series(rf.feature_importances_, index=valid_cols).sort_values(ascending=False)
    print(f'\nTop-10 regime features:')
    for feat, score in imp.head(10).items():
        print(f'  {feat}: {score:.4f}')

    # Baseline: simple SMA crossover regime detection
    btc_test = btc_df.iloc[WARMUP:].iloc[split:]
    sma50_test = SMA(btc_test['close'], 50)
    sma200_test = SMA(btc_test['close'], 200)
    vol_test = btc_test['close'].pct_change().rolling(20).std()
    vol_med_test = vol_test.rolling(min(500, len(vol_test))).median()

    baseline_bull = sma50_test > sma200_test
    baseline_highvol = vol_test > vol_med_test
    baseline_pred = pd.Series(0, index=btc_test.index)
    baseline_pred[~baseline_bull & ~baseline_highvol] = 0
    baseline_pred[~baseline_bull & baseline_highvol] = 1
    baseline_pred[baseline_bull & ~baseline_highvol] = 2
    baseline_pred[baseline_bull & baseline_highvol] = 3
    baseline_pred = baseline_pred.reindex(y_test.index).fillna(0).astype(int)
    baseline_acc = accuracy_score(y_test, baseline_pred)
    print(f'\nBaseline (SMA cross) accuracy: {baseline_acc:.3f}')
    print(f'ML improvement: {(rf_acc - baseline_acc) * 100:+.1f}%')

    # Test regime-based strategy selection across coins
    print(f'\n--- Testing regime-based strategy selection ---')
    coin_results = {}

    for coin in tqdm(COINS, desc='Regime test'):
        if shutdown_requested:
            break

        try:
            df = load_cached_data(coin)
            df = add_all_indicators(df)
            test_portion = df.iloc[split:]

            # Apply BTC regime to this coin's test data
            ml_regimes = pd.Series(rf_pred, index=y_test.index)

            # Find best strategy per regime
            regime_strategies = {}
            for regime_val in range(4):
                regime_mask = ml_regimes == regime_val
                regime_dates = ml_regimes[regime_mask].index
                if len(regime_dates) < 50:
                    continue

                regime_df = test_portion.reindex(regime_dates).dropna()
                if len(regime_df) < 50:
                    continue

                best_pf = 0
                best_name = None
                for sname, sfunc in ALL_STRATEGIES.items():
                    try:
                        entry, exit_sig = sfunc(regime_df)
                        bt = Backtester(regime_df, fee=0.001, slippage=0.0005)
                        result = bt.run(entry, exit_sig, stop_loss=0.003)
                        if result.total_trades >= 5 and result.profit_factor > best_pf:
                            best_pf = result.profit_factor
                            best_name = sname
                    except Exception:
                        continue

                if best_name:
                    regime_strategies[REGIME_NAMES[regime_val]] = {
                        'strategy': best_name,
                        'profit_factor': best_pf,
                    }

            coin_results[coin] = regime_strategies
        except Exception as e:
            coin_results[coin] = {'error': str(e)}

    results = {
        'regime_detection': {
            'rf_accuracy': float(rf_acc),
            'xgb_accuracy': float(xgb_acc),
            'cv_mean': float(np.mean(cv_scores)),
            'cv_std': float(np.std(cv_scores)),
            'baseline_accuracy': float(baseline_acc),
            'ml_improvement': float(rf_acc - baseline_acc),
        },
        'top_features': {k: float(v) for k, v in imp.head(20).items()},
        'regime_distribution': {REGIME_NAMES[v]: int((y == v).sum()) for v in range(4)},
        'per_coin_regime_strategies': coin_results,
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(results, f, indent=2, default=str)

    print(f'\n{"=" * 60}')
    print(f'RESULTS: {RESULTS_FILE}')
    print(f'ML regime accuracy: {rf_acc:.3f} vs baseline: {baseline_acc:.3f}')


if __name__ == '__main__':
    main()
