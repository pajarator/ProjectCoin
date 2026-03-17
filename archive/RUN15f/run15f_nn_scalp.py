"""
RUN17: Neural Network Scalp Trading Enhancement

Goal: Improve 1m scalp trading using NN to filter signals.
Uses 1m data from data_cache.

Scalp strategies to improve:
1. vol_spike_rev: volume spike + RSI extreme
2. stoch_cross: stochastic crossover at extremes
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
import warnings
warnings.filterwarnings('ignore')

# Config
DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['BTC', 'ETH', 'BNB', 'SOL', 'ADA', 'XRP', 'DOGE', 'LTC', 'LINK', 'DOT']
LOOKAHEAD = 3  # Predict 3 bars ahead (~3 min)

# 1m-specific features
FEATURES = [
    'rsi',           # RSI(14)
    'vol_ratio',     # volume / vol_ma
    'stoch_k',       # Stochastic %K
    'stoch_d',       # Stochastic %D
    'stoch_cross',   # K crossing D
    'bb_position',   # position within BB
    'roc_3',         # 3-bar rate of change
    'avg_body_3',    # avg candle body size
    'hour_of_day',   # time of day
]


def load_1m_data(coin: str) -> pd.DataFrame:
    """Load 1m data."""
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    # Use subset for faster processing (last 3 months)
    return df.tail(30000)  # ~21 days of 1m data


def compute_1m_features(df: pd.DataFrame) -> pd.DataFrame:
    """Compute 1m-specific features."""
    feat = pd.DataFrame(index=df.index)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']
    
    # RSI
    delta = c.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    feat['rsi'] = (100 - (100 / (1 + rs))).fillna(50)
    
    # Volume ratio
    v_ma = v.rolling(20).mean()
    feat['vol_ratio'] = v / v_ma
    
    # Stochastic
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    feat['stoch_d'] = feat['stoch_k'].rolling(3).mean()
    
    # Stochastic cross signal
    feat['stoch_cross'] = 0
    feat.loc[feat['stoch_k'] > feat['stoch_d'], 'stoch_cross'] = 1  # bullish
    feat.loc[feat['stoch_k'] < feat['stoch_d'], 'stoch_cross'] = -1  # bearish
    
    # Bollinger position
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    
    # ROC 3
    feat['roc_3'] = c.pct_change(3)
    
    # Avg body 3
    body = (c - o).abs()
    feat['avg_body_3'] = body.rolling(3).mean() / c
    
    # Time
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
    return feat


def generate_scalp_signals(df: pd.DataFrame, features: pd.DataFrame) -> pd.DataFrame:
    """
    Generate scalp signals based on CoinClaw logic:
    1. vol_spike_rev: vol > 3.5x MA + RSI < 20 (long) or RSI > 80 (short)
    2. stoch_cross: stoch K/D cross at extreme (< 20 or > 80)
    """
    signals = pd.DataFrame(index=df.index)
    c = df['close']
    
    # vol_spike_rev LONG: vol > 3.5x, RSI < 20
    vol_spike_long = (features['vol_ratio'] > 3.5) & (features['rsi'] < 20)
    
    # vol_spike_rev SHORT: vol > 3.5x, RSI > 80
    vol_spike_short = (features['vol_ratio'] > 3.5) & (features['rsi'] > 80)
    
    # stoch_cross LONG: K crosses above D at < 20
    stoch_k = features['stoch_k']
    stoch_d = features['stoch_d']
    stoch_k_prev = stoch_k.shift(1)
    stoch_d_prev = stoch_d.shift(1)
    stoch_cross_long = (stoch_k_prev <= stoch_d_prev) & (stoch_k > stoch_d) & (stoch_k < 20)
    
    # stoch_cross SHORT: K crosses below D at > 80
    stoch_cross_short = (stoch_k_prev >= stoch_d_prev) & (stoch_k < stoch_d) & (stoch_k > 80)
    
    # Combined signals
    signals['long'] = vol_spike_long | stoch_cross_long
    signals['short'] = vol_spike_short | stoch_cross_short
    
    # Direction: 1 = long, -1 = short, 0 = no signal
    signals['direction'] = 0
    signals.loc[signals['long'], 'direction'] = 1
    signals.loc[signals['short'], 'direction'] = -1
    
    return signals


def run_scalp_backtest(coin: str, use_nn: bool = False, threshold: float = 0.55) -> dict:
    """Backtest scalp strategy for a coin."""
    print(f"  {coin}...", end=" ", flush=True)
    
    df = load_1m_data(coin)
    features = compute_1m_features(df)
    signals = generate_scalp_signals(df, features)
    c = df['close']
    
    # Target: did price move in our favor in next 3 bars?
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    
    # For long signals: did price go up?
    long_profitable = future_return > 0
    # For short signals: did price go down?
    short_profitable = future_return < 0
    
    # Clean data
    valid = ~(features.isna().any(axis=1)) & (signals['direction'] != 0) & future_return.notna()
    
    if valid.sum() < 100:
        print(f"skip ({valid.sum()} signals)")
        return None
    
    # Get signal indices
    signal_idx = valid & (signals['direction'] != 0)
    X = features[signal_idx].values
    y_long = long_profitable[signal_idx].astype(int).values
    y_short = short_profitable[signal_idx].astype(int).values
    directions = signals[signal_idx]['direction'].values
    
    if len(X) < 100:
        print(f"skip ({len(X)} signals)")
        return None
    
    # Split: first half train, second half test
    split = int(len(X) * 0.5)
    
    if not use_nn:
        # Baseline: take all signals
        long_correct = (directions[:split] == 1) & (y_long[:split] == 1)
        short_correct = (directions[:split] == -1) & (y_short[:split] == 1)
        train_wr = (long_correct.sum() + short_correct.sum()) / split * 100
        
        long_correct_test = (directions[split:] == 1) & (y_long[split:] == 1)
        short_correct_test = (directions[split:] == -1) & (y_short[split:] == 1)
        test_wr = (long_correct_test.sum() + short_correct_test.sum()) / (len(X) - split) * 100
        
        print(f"{len(X)} signals, Train WR: {train_wr:.1f}%, Test WR: {test_wr:.1f}%")
        
        return {
            'coin': coin,
            'signals': len(X),
            'train_wr': round(train_wr, 1),
            'test_wr': round(test_wr, 1),
        }
    
    # NN: train model to predict profitability
    X_train, X_test = X[:split], X[split:]
    y_train_long = y_long[:split]
    y_train_short = y_short[:split]
    
    # Scale
    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X_train)
    X_test_s = scaler.transform(X_test)
    
    # Train two models: one for longs, one for shorts
    model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
    model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
    
    model_long.fit(X_train_s, y_train_long)
    model_short.fit(X_train_s, y_train_short)
    
    # Predict probabilities
    prob_long = model_long.predict_proba(X_test_s)[:, 1]
    prob_short = model_short.predict_proba(X_test_s)[:, 1]
    
    # Apply NN filter
    filtered = np.zeros(len(X_test), dtype=bool)
    for i in range(len(X_test)):
        if directions[split + i] == 1:  # Long signal
            if prob_long[i] > threshold:
                filtered[i] = True
        else:  # Short signal
            if prob_short[i] > threshold:
                filtered[i] = True
    
    if filtered.sum() == 0:
        print(f"no signals above threshold")
        return {
            'coin': coin,
            'nn_signals': 0,
            'nn_wr': 0,
        }
    
    # Calculate NN win rate
    y_test_long = y_long[split:]
    y_test_short = y_short[split:]
    
    long_correct = (directions[split:][filtered] == 1) & (y_test_long[filtered] == 1)
    short_correct = (directions[split:][filtered] == -1) & (y_test_short[filtered] == 1)
    nn_wr = (long_correct.sum() + short_correct.sum()) / filtered.sum() * 100
    
    print(f"{len(X)} -> {filtered.sum()} signals, NN WR: {nn_wr:.1f}%")
    
    return {
        'coin': coin,
        'signals': len(X),
        'nn_signals': int(filtered.sum()),
        'nn_wr': round(nn_wr, 1),
    }


def main():
    print("=" * 70)
    print("RUN17: Neural Network Scalp Trading Enhancement")
    print("=" * 70)
    print()
    print("Data: 1m timeframe from data_cache")
    print("Strategies: vol_spike_rev + stoch_cross")
    print()
    
    # Baseline
    print("=== BASELINE (all signals) ===")
    baseline_results = []
    for coin in COINS:
        try:
            result = run_scalp_backtest(coin, use_nn=False)
            if result:
                baseline_results.append(result)
        except Exception as e:
            print(f"  {coin}: ERROR - {e}")
    
    baseline_avg_wr = np.mean([r['test_wr'] for r in baseline_results])
    baseline_total = sum(r['signals'] for r in baseline_results)
    print(f"Baseline avg WR: {baseline_avg_wr:.1f}%, Total signals: {baseline_total}")
    
    # NN with different thresholds
    print("\n=== NN FILTER ===")
    thresholds = [0.50, 0.55, 0.60]
    best_threshold = 0.55
    best_avg_wr = 0
    
    for threshold in thresholds:
        print(f"\n--- Threshold: {threshold} ---")
        nn_results = []
        for coin in COINS:
            try:
                result = run_scalp_backtest(coin, use_nn=True, threshold=threshold)
                if result and result.get('nn_signals', 0) > 0:
                    nn_results.append(result)
            except Exception as e:
                print(f"  {coin}: ERROR - {e}")
        
        if nn_results:
            avg_wr = np.mean([r['nn_wr'] for r in nn_results])
            total_signals = sum(r['nn_signals'] for r in nn_results)
            print(f"  → Avg WR: {avg_wr:.1f}%, Signals: {total_signals}")
            
            if avg_wr > best_avg_wr:
                best_avg_wr = avg_wr
                best_threshold = threshold
    
    print(f"\n=== SUMMARY ===")
    print(f"Baseline WR: {baseline_avg_wr:.1f}%")
    print(f"Best NN WR:  {best_avg_wr:.1f}% (threshold={best_threshold})")
    print(f"Delta:       {best_avg_wr - baseline_avg_wr:+.1f} pts")
    
    # Save results
    output = {
        'experiment': 'RUN17',
        'description': 'NN Scalp Trading on 1m',
        'features': FEATURES,
        'lookahead': LOOKAHEAD,
        'baseline': {
            'avg_wr': baseline_avg_wr,
            'total_signals': baseline_total,
            'per_coin': baseline_results
        },
        'best_threshold': best_threshold,
        'best_nn_wr': best_avg_wr,
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN17', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN17/run15f_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print("\nSaved to archive/RUN17/run15f_results.json")


if __name__ == '__main__':
    main()
