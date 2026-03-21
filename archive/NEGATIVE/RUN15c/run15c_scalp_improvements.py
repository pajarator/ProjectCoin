"""
RUN15c: 5 Ways to Improve Scalping Win Rate

Tests 5 different approaches to beat the baseline NN (Logistic Regression):
1. GradientBoosting classifier
2. RandomForest classifier  
3. More features (add EMA, MACD, ATR)
4. Optimal threshold search
5. Walk-forward training (rolling windows)
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.linear_model import LogisticRegression
from sklearn.ensemble import GradientBoostingClassifier, RandomForestClassifier
from sklearn.preprocessing import StandardScaler
import warnings
warnings.filterwarnings('ignore')

DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['BTC', 'ETH', 'BNB', 'SOL', 'ADA', 'XRP', 'DOGE', 'LTC', 'LINK', 'DOT']
LOOKAHEAD = 3

# Approach 3: More features (extended set)
EXTENDED_FEATURES = [
    'rsi', 'vol_ratio', 'stoch_k', 'stoch_d', 'stoch_cross',
    'bb_position', 'roc_3', 'avg_body_3', 'hour_of_day',
    # New features
    'ema9', 'ema21', 'ema_diff', 'macd_hist', 'atr_pct',
    'high_low_pct', 'candle_strength', 'volume_steepness'
]

BASIC_FEATURES = ['rsi', 'vol_ratio', 'stoch_k', 'stoch_d', 'stoch_cross', 
                  'bb_position', 'roc_3', 'avg_body_3', 'hour_of_day']


def load_1m_data(coin: str) -> pd.DataFrame:
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df.tail(30000)


def compute_basic_features(df: pd.DataFrame) -> pd.DataFrame:
    feat = pd.DataFrame(index=df.index)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']
    
    delta = c.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    feat['rsi'] = (100 - (100 / (1 + rs))).fillna(50)
    
    v_ma = v.rolling(20).mean()
    feat['vol_ratio'] = v / v_ma
    
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    feat['stoch_d'] = feat['stoch_k'].rolling(3).mean()
    
    feat['stoch_cross'] = 0
    feat.loc[feat['stoch_k'] > feat['stoch_d'], 'stoch_cross'] = 1
    feat.loc[feat['stoch_k'] < feat['stoch_d'], 'stoch_cross'] = -1
    
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    feat['bb_width'] = (bb_upper - bb_lower) / c
    
    feat['roc_3'] = c.pct_change(3)
    body = (c - o).abs()
    feat['avg_body_3'] = body.rolling(3).mean() / c
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
    return feat


def compute_extended_features(df: pd.DataFrame) -> pd.DataFrame:
    """Extended feature set for approach 3."""
    feat = compute_basic_features(df)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']
    
    # EMA
    feat['ema9'] = c.ewm(span=9).mean() / c - 1
    feat['ema21'] = c.ewm(span=21).mean() / c - 1
    feat['ema_diff'] = feat['ema9'] - feat['ema21']
    
    # MACD
    ema12 = c.ewm(span=12).mean()
    ema26 = c.ewm(span=26).mean()
    macd = ema12 - ema26
    signal = macd.ewm(span=9).mean()
    feat['macd_hist'] = (macd - signal) / c
    
    # ATR
    high_low = h - l
    high_close = (h - c.shift(1)).abs()
    low_close = (l - c.shift(1)).abs()
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    feat['atr_pct'] = tr.rolling(14).mean() / c
    
    # High/Low percentage
    feat['high_low_pct'] = (h - l) / c
    
    # Candle strength
    feat['candle_strength'] = (c - o) / (h - l).replace(0, np.nan)
    
    # Volume steepness
    v_ma = v.rolling(10).mean()
    feat['volume_steepness'] = (v - v_ma) / v_ma
    
    return feat


def generate_signals(df: pd.DataFrame, features: pd.DataFrame) -> pd.DataFrame:
    signals = pd.DataFrame(index=df.index)
    c = df['close']
    
    vol_spike_long = (features['vol_ratio'] > 3.5) & (features['rsi'] < 20)
    vol_spike_short = (features['vol_ratio'] > 3.5) & (features['rsi'] > 80)
    
    stoch_k = features['stoch_k']
    stoch_d = features['stoch_d']
    stoch_k_prev = stoch_k.shift(1)
    stoch_d_prev = stoch_d.shift(1)
    stoch_cross_long = (stoch_k_prev <= stoch_d_prev) & (stoch_k > stoch_d) & (stoch_k < 20)
    stoch_cross_short = (stoch_k_prev >= stoch_d_prev) & (stoch_k < stoch_d) & (stoch_k > 80)
    
    signals['long'] = vol_spike_long | stoch_cross_long
    signals['short'] = vol_spike_short | stoch_cross_short
    
    signals['direction'] = 0
    signals.loc[signals['long'], 'direction'] = 1
    signals.loc[signals['short'], 'direction'] = -1
    
    return signals


def test_approach(coin: str, approach: str, features_list: list) -> dict:
    """Test a specific approach for a coin."""
    
    df = load_1m_data(coin)
    
    if approach == "more_features":
        features = compute_extended_features(df)
    else:
        features = compute_basic_features(df)
    
    signals = generate_signals(df, features)
    c = df['close']
    
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    long_profitable = future_return > 0
    short_profitable = future_return < 0
    
    valid = ~(features[features_list].isna().any(axis=1)) & (signals['direction'] != 0) & future_return.notna()
    signal_idx = valid & (signals['direction'] != 0)
    
    X = features[signal_idx][features_list].values
    y_long = long_profitable[signal_idx].astype(int).values
    y_short = short_profitable[signal_idx].astype(int).values
    directions = signals[signal_idx]['direction'].values
    
    if len(X) < 200:
        return {'wr': 0, 'signals': 0}
    
    # Use walk-forward for more robust results
    split1 = int(len(X) * 0.33)
    split2 = int(len(X) * 0.66)
    
    results = []
    thresholds_to_try = [0.50, 0.55, 0.60]
    
    for train_start, test_start, test_end in [(0, split1, split2), (split1, split2, len(X))]:
        X_train = X[train_start:test_start]
        X_test = X[test_start:test_end]
        y_long_train = y_long[train_start:test_start]
        y_short_train = y_short[train_start:test_start]
        directions_test = directions[test_start:test_end]
        y_long_test = y_long[test_start:test_end]
        y_short_test = y_short[test_start:test_end]
        
        if len(X_train) < 50 or len(X_test) < 50:
            continue
        
        scaler = StandardScaler()
        X_train_s = scaler.fit_transform(X_train)
        X_test_s = scaler.transform(X_test)
        
        # Select model based on approach
        if approach == "logreg":
            model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
            model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        elif approach == "gradient_boosting":
            model_long = GradientBoostingClassifier(n_estimators=50, max_depth=3, random_state=42)
            model_short = GradientBoostingClassifier(n_estimators=50, max_depth=3, random_state=42)
        elif approach == "random_forest":
            model_long = RandomForestClassifier(n_estimators=50, max_depth=5, random_state=42)
            model_short = RandomForestClassifier(n_estimators=50, max_depth=5, random_state=42)
        else:
            model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
            model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        
        model_long.fit(X_train_s, y_long_train)
        model_short.fit(X_train_s, y_short_train)
        
        prob_long = model_long.predict_proba(X_test_s)[:, 1]
        prob_short = model_short.predict_proba(X_test_s)[:, 1]
        
        for threshold in thresholds_to_try:
            wins = 0
            losses = 0
            for i in range(len(directions_test)):
                direction = directions_test[i]
                prob = prob_long[i] if direction == 1 else prob_short[i]
                
                if (direction == 1 and prob > threshold) or (direction == -1 and prob > threshold):
                    if (direction == 1 and y_long_test[i] == 1) or (direction == -1 and y_short_test[i] == 1):
                        wins += 1
                    else:
                        losses += 1
            
            if wins + losses > 0:
                results.append({'threshold': threshold, 'wins': wins, 'losses': losses})
    
    if not results:
        return {'wr': 0, 'signals': 0}
    
    # Find best threshold
    best = max(results, key=lambda x: x['wins'] / (x['wins'] + x['losses']) if (x['wins'] + x['losses']) > 0 else 0)
    
    total_wins = sum(r['wins'] for r in results)
    total_losses = sum(r['losses'] for r in results)
    total_signals = total_wins + total_losses
    
    wr = total_wins / total_signals * 100 if total_signals > 0 else 0
    
    return {'wr': wr, 'signals': total_signals, 'best_threshold': best['threshold']}


def run_all_approaches():
    print("=" * 80)
    print("RUN15c: 5 Ways to Improve Scalping")
    print("=" * 80)
    print()
    
    approaches = [
        ("logreg", "1. Logistic Regression (baseline)", BASIC_FEATURES),
        ("gradient_boosting", "2. Gradient Boosting", BASIC_FEATURES),
        ("random_forest", "3. Random Forest", BASIC_FEATURES),
        ("more_features", "4. More Features (18)", EXTENDED_FEATURES),
        ("walkforward", "5. Walk-Forward (2 windows)", BASIC_FEATURES),
    ]
    
    all_results = {}
    
    for approach_id, approach_name, features_list in approaches:
        print(f"\n{approach_name}")
        print("-" * 60)
        
        coin_results = []
        total_wins = 0
        total_losses = 0
        
        for coin in COINS:
            try:
                r = test_approach(coin, approach_id, features_list)
                if r['signals'] > 0:
                    print(f"  {coin}: WR={r['wr']:.1f}%, {r['signals']} signals")
                    coin_results.append({'coin': coin, 'wr': r['wr'], 'signals': r['signals']})
                    total_wins += int(r['wr'] * r['signals'] / 100)
                    total_losses += int((100 - r['wr']) * r['signals'] / 100)
            except Exception as e:
                print(f"  {coin}: ERROR - {e}")
        
        if total_wins + total_losses > 0:
            avg_wr = total_wins / (total_wins + total_losses) * 100
            print(f"  → AVG: {avg_wr:.1f}% WR, {total_wins + total_losses} signals")
            all_results[approach_name] = {
                'wr': avg_wr,
                'signals': total_wins + total_losses,
                'per_coin': coin_results
            }
    
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print()
    print(f"{'Approach':<35} {'WR%':>10} {'Signals':>12}")
    print("-" * 60)
    
    sorted_results = sorted(all_results.items(), key=lambda x: x[1]['wr'], reverse=True)
    for name, data in sorted_results:
        print(f"{name:<35} {data['wr']:>9.1f}% {data['signals']:>12}")
    
    # Save
    output = {
        'experiment': 'RUN15c',
        'approaches': all_results,
        'best': sorted_results[0][0] if sorted_results else None
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN15c', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN15c/run15c_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print(f"\nBest approach: {sorted_results[0][0]} with {sorted_results[0][1]['wr']:.1f}% WR")
    print("\nSaved to archive/RUN15c/run15c_results.json")
    
    return output


if __name__ == '__main__':
    run_all_approaches()
