"""
RUN16: Neural Network Mean Reversion (v3 - Walk-Forward)

Key differences from v2:
1. Walk-forward training: Train on rolling windows, test on next window
2. Per-coin models (each coin has different behavior)
3. Uses XGBoost-style approach via sklearn
4. Focus on HIGH CONFIDENCE predictions only
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.ensemble import RandomForestClassifier, GradientBoostingClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
from collections import defaultdict
import warnings
warnings.filterwarnings('ignore')

# Config
DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['DASH', 'UNI', 'NEAR', 'ADA', 'LTC', 'SHIB', 'LINK', 'ETH',
         'BTC', 'BNB', 'XRP', 'SOL', 'DOT', 'AVAX', 'ATOM', 'DOGE']

LOOKAHEAD = 4
Z_THRESHOLD = -1.5

# Simpler, more robust feature set
FEATURES = [
    'z_score', 'rsi_14', 'bb_position', 'volume_ratio',
    'stoch_k', 'momentum_10', 'hour_of_day', 'day_of_week'
]


def load_coin_data(coin: str) -> pd.DataFrame:
    path = f'{DATA_PATH}/{coin}_USDT_15m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df


def compute_features(df: pd.DataFrame) -> pd.DataFrame:
    """Compute minimal feature set."""
    feat = pd.DataFrame(index=df.index)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']
    
    # Z-SCORE
    sma20 = c.rolling(20).mean()
    std20 = c.rolling(20).std()
    feat['z_score'] = (c - sma20) / std20
    
    # RSI
    delta = c.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    feat['rsi_14'] = (100 - (100 / (1 + rs))).fillna(50)
    
    # BB position
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    
    # Volume
    v_ma = v.rolling(20).mean()
    feat['volume_ratio'] = v / v_ma
    
    # Stochastic
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    
    # Momentum
    feat['momentum_10'] = c.pct_change(10)
    
    # Time
    feat['hour_of_day'] = df['timestamp'].dt.hour
    feat['day_of_week'] = df['timestamp'].dt.dayofweek
    
    return feat


def walk_forward_backtest(coin: str) -> dict:
    """Walk-forward backtest with proper temporal separation."""
    print(f"  {coin}...", end=" ", flush=True)
    
    df = load_coin_data(coin)
    features = compute_features(df)
    c = df['close']
    
    # Create target
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    target = (future_return > 0).astype(int)  # 1 = up, 0 = down
    
    # Get MR signals
    mr_signal = features['z_score'] < Z_THRESHOLD
    
    # Clean data
    valid_idx = ~(features.isna().any(axis=1)) & mr_signal & target.notna()
    X = features[valid_idx].values
    y = target[valid_idx].values
    times = features[valid_idx].index
    
    if len(X) < 500:
        print(f"skip ({len(X)} signals)")
        return None
    
    # Walk-forward: 3 windows
    # Window 1: train months 1-3, test month 4
    # Window 2: train months 2-4, test month 5
    # Window 3: train months 3-5, test month 6
    n = len(X)
    window_size = n // 6
    
    results = []
    
    for w in range(3):
        train_end = (w + 2) * window_size
        test_start = train_end
        test_end = min(test_start + window_size, n)
        
        if test_end - test_start < 50:
            continue
        
        X_train, X_test = X[:train_end], X[test_start:test_end]
        y_train, y_test = y[:train_end], y[test_start:test_end]
        
        # Scale
        scaler = StandardScaler()
        X_train_s = scaler.fit_transform(X_train)
        X_test_s = scaler.transform(X_test)
        
        # Train simple model
        model = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        model.fit(X_train_s, y_train)
        
        # Predict probabilities
        probs = model.predict_proba(X_test_s)[:, 1]
        
        # High confidence: only take when P > 0.55 or P < 0.45
        high_conf = (probs > 0.55) | (probs < 0.45)
        
        if high_conf.sum() > 0:
            y_hc = y_test[high_conf]
            wr = y_hc.mean() * 100
            results.append({
                'window': w + 1,
                'signals': int(high_conf.sum()),
                'wr': wr
            })
    
    if not results:
        print("no results")
        return {'coin': coin, 'windows': 0}
    
    avg_wr = np.mean([r['wr'] for r in results])
    total_signals = sum(r['signals'] for r in results)
    
    print(f"{total_signals} signals, WR={avg_wr:.1f}%")
    
    return {
        'coin': coin,
        'windows': len(results),
        'avg_wr': round(avg_wr, 1),
        'total_signals': total_signals,
        'per_window': results
    }


def baseline_backtest(coin: str) -> dict:
    """Baseline: plain MR without any filter."""
    df = load_coin_data(coin)
    c = df['close']
    
    sma20 = c.rolling(20).mean()
    std20 = c.rolling(20).std()
    z = (c - sma20) / std20
    
    mr_signal = z < Z_THRESHOLD
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    target = (future_return > 0).astype(int)
    
    valid = mr_signal & target.notna()
    y = target[valid].values
    
    if len(y) == 0:
        return {'wr': 0, 'signals': 0}
    
    return {'wr': y.mean() * 100, 'signals': len(y)}


def main():
    print("=" * 70)
    print("RUN16 v3: Walk-Forward Neural Network Mean Reversion")
    print("=" * 70)
    print()
    print("Approach: Walk-forward validation, high-confidence predictions only")
    print()
    
    # Run walk-forward tests
    nn_results = []
    for coin in COINS:
        try:
            result = walk_forward_backtest(coin)
            if result and result.get('windows', 0) > 0:
                nn_results.append(result)
        except Exception as e:
            print(f"  {coin}: ERROR - {e}")
    
    # Baseline
    print("\n--- Baseline (no filter) ---")
    baseline_results = []
    for coin in COINS:
        try:
            result = baseline_backtest(coin)
            baseline_results.append(result)
        except:
            pass
    
    baseline_avg_wr = np.mean([r['wr'] for r in baseline_results if r['signals'] > 0])
    baseline_total = sum(r['signals'] for r in baseline_results)
    
    nn_avg_wr = np.mean([r['avg_wr'] for r in nn_results])
    nn_total = sum(r['total_signals'] for r in nn_results)
    
    print(f"\n{'COIN':<8} {'NN_WR':>10} {'NN_SIG':>10} {'BASE_WR':>10} {'BASE_SIG':>10}")
    print("-" * 50)
    
    for coin in COINS:
        nn_r = next((r for r in nn_results if r['coin'] == coin), None)
        base_r = next((r for r in baseline_results if r['signals'] > 0), None)
        
        nn_wr = nn_r['avg_wr'] if nn_r else 0
        nn_sig = nn_r['total_signals'] if nn_r else 0
        base_wr = base_r['wr'] if base_r else 0
        base_sig = base_r['signals'] if base_r else 0
        
        print(f"{coin:<8} {nn_wr:>9.1f}% {nn_sig:>10} {base_wr:>9.1f}% {base_sig:>10}")
    
    print("-" * 50)
    print(f"{'AVG':<8} {nn_avg_wr:>9.1f}% {nn_total:>10} {baseline_avg_wr:>9.1f}% {baseline_total:>10}")
    
    print(f"\n=== SUMMARY ===")
    print(f"NN (walk-forward): {nn_avg_wr:.1f}% WR, {nn_total} signals")
    print(f"Baseline:          {baseline_avg_wr:.1f}% WR, {baseline_total} signals")
    print(f"Delta:             {nn_avg_wr - baseline_avg_wr:+.1f} pts")
    print(f"Signal reduction: {(1 - nn_total/baseline_total)*100:.0f}%")
    
    # Save
    output = {
        'experiment': 'RUN16',
        'version': 'v3 - Walk-Forward',
        'results': nn_results,
        'baseline': {
            'avg_wr': baseline_avg_wr,
            'total_signals': baseline_total
        },
        'summary': {
            'nn_avg_wr': nn_avg_wr,
            'nn_total_signals': nn_total,
            'delta': nn_avg_wr - baseline_avg_wr
        }
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN16', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN16/run16_v3_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print("\nSaved to archive/RUN16/run16_v3_results.json")


if __name__ == '__main__':
    main()
