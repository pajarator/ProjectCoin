"""
RUN17b: Detailed Win/Loss Analysis for NN Scalp Filter

Analyzes: 
1. Baseline: all signals - wins vs losses
2. Filtered OUT: what % would have won vs lost
3. Kept by NN: wins vs losses
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
import warnings
warnings.filterwarnings('ignore')

DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['BTC', 'ETH', 'BNB', 'SOL', 'ADA', 'XRP', 'DOGE', 'LTC', 'LINK', 'DOT']
LOOKAHEAD = 3

FEATURES = ['rsi', 'vol_ratio', 'stoch_k', 'stoch_d', 'stoch_cross', 
            'bb_position', 'roc_3', 'avg_body_3', 'hour_of_day']


def load_1m_data(coin: str) -> pd.DataFrame:
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df.tail(30000)


def compute_features(df: pd.DataFrame) -> pd.DataFrame:
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
    
    feat['roc_3'] = c.pct_change(3)
    body = (c - o).abs()
    feat['avg_body_3'] = body.rolling(3).mean() / c
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
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


def detailed_analysis(coin: str, threshold: float = 0.55) -> dict:
    print(f"  {coin}...", end=" ", flush=True)
    
    df = load_1m_data(coin)
    features = compute_features(df)
    signals = generate_signals(df, features)
    c = df['close']
    
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    long_profitable = future_return > 0
    short_profitable = future_return < 0
    
    valid = ~(features.isna().any(axis=1)) & (signals['direction'] != 0) & future_return.notna()
    signal_idx = valid & (signals['direction'] != 0)
    
    X = features[signal_idx].values
    y_long = long_profitable[signal_idx].astype(int).values
    y_short = short_profitable[signal_idx].astype(int).values
    directions = signals[signal_idx]['direction'].values
    
    if len(X) < 100:
        print(f"skip")
        return None
    
    split = int(len(X) * 0.5)
    
    # === BASELINE: All signals ===
    all_wins = 0
    all_losses = 0
    for i in range(len(X)):
        direction = directions[i]
        if direction == 1:  # Long
            if y_long[i] == 1:
                all_wins += 1
            else:
                all_losses += 1
        else:  # Short
            if y_short[i] == 1:
                all_wins += 1
            else:
                all_losses += 1
    
    # === NN FILTER ===
    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X[:split])
    X_test_s = scaler.transform(X[split:])
    
    model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
    model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
    model_long.fit(X_train_s, y_long[:split])
    model_short.fit(X_train_s, y_short[:split])
    
    prob_long = model_long.predict_proba(X_test_s)[:, 1]
    prob_short = model_short.predict_proba(X_test_s)[:, 1]
    
    # Separate kept vs filtered
    kept_wins = 0
    kept_losses = 0
    filtered_wins = 0
    filtered_losses = 0
    
    directions_test = directions[split:]
    y_long_test = y_long[split:]
    y_short_test = y_short[split:]
    
    for i in range(len(directions_test)):
        direction = directions_test[i]
        is_long = direction == 1
        
        prob = prob_long[i] if is_long else prob_short[i]
        actual_win = (y_long_test[i] == 1) if is_long else (y_short_test[i] == 1)
        
        kept = (is_long and prob > threshold) or (not is_long and prob > threshold)
        
        if kept:
            if actual_win:
                kept_wins += 1
            else:
                kept_losses += 1
        else:
            if actual_win:
                filtered_wins += 1
            else:
                filtered_losses += 1
    
    print(f"Base: {all_wins}W/{all_losses}L | NN kept: {kept_wins}W/{kept_losses}L | NN filtered: {filtered_wins}W/{filtered_losses}L")
    
    return {
        'coin': coin,
        'baseline': {'wins': all_wins, 'losses': all_losses},
        'kept': {'wins': kept_wins, 'losses': kept_losses},
        'filtered': {'wins': filtered_wins, 'losses': filtered_losses},
    }


def main():
    print("=" * 80)
    print("RUN17b: Detailed Win/Loss Analysis - NN Scalp Filter")
    print("=" * 80)
    print()
    print(f"{'Coin':<8} | {'BASELINE':^20} | {'NN KEPT':^20} | {'NN FILTERED':^20}")
    print(f"{'':8} | {'Wins':>8} {'Losses':>8} {'WR%':>8} | {'Wins':>8} {'Losses':>8} {'WR%':>8} | {'Wins':>8} {'Losses':>8} {'WR%':>8}")
    print("-" * 80)
    
    results = []
    total_baseline_wins = 0
    total_baseline_losses = 0
    total_kept_wins = 0
    total_kept_losses = 0
    total_filtered_wins = 0
    total_filtered_losses = 0
    
    for coin in COINS:
        try:
            r = detailed_analysis(coin, threshold=0.55)
            if r:
                results.append(r)
                total_baseline_wins += r['baseline']['wins']
                total_baseline_losses += r['baseline']['losses']
                total_kept_wins += r['kept']['wins']
                total_kept_losses += r['kept']['losses']
                total_filtered_wins += r['filtered']['wins']
                total_filtered_losses += r['filtered']['losses']
        except Exception as e:
            print(f"  {coin}: ERROR - {e}")
    
    print("-" * 80)
    
    base_total = total_baseline_wins + total_baseline_losses
    base_wr = total_baseline_wins / base_total * 100 if base_total > 0 else 0
    
    kept_total = total_kept_wins + total_kept_losses
    kept_wr = total_kept_wins / kept_total * 100 if kept_total > 0 else 0
    
    filtered_total = total_filtered_wins + total_filtered_losses
    filtered_wr = total_filtered_wins / filtered_total * 100 if filtered_total > 0 else 0
    
    print(f"{'TOTAL':<8} | {total_baseline_wins:>8} {total_baseline_losses:>8} {base_wr:>7.1f}% | {total_kept_wins:>8} {total_kept_losses:>8} {kept_wr:>7.1f}% | {total_filtered_wins:>8} {total_filtered_losses:>8} {filtered_wr:>7.1f}%")
    
    print()
    print("=" * 80)
    print("ANALYSIS")
    print("=" * 80)
    print()
    print(f"1. BASELINE (all signals):")
    print(f"   - Total: {base_total} signals")
    print(f"   - Win Rate: {base_wr:.1f}%")
    print()
    print(f"2. NN KEPT (filtered IN):")
    print(f"   - Total: {kept_total} signals ({kept_total/base_total*100:.1f}% of baseline)")
    print(f"   - Win Rate: {kept_wr:.1f}%")
    print()
    print(f"3. NN FILTERED (filtered OUT):")
    print(f"   - Total: {filtered_total} signals ({filtered_total/base_total*100:.1f}% of baseline)")
    print(f"   - Win Rate: {filtered_wr:.1f}%")
    print()
    print(f"4. WHAT THE NN REMOVED:")
    print(f"   - Wins removed: {total_filtered_wins}")
    print(f"   - Losses removed: {total_filtered_losses}")
    print(f"   - Net: NN filtered out {total_filtered_losses - total_filtered_wins} more losses than wins!")
    print()
    
    improvement = kept_wr - base_wr
    print(f"5. RESULT:")
    print(f"   - WR improvement: +{improvement:.1f} pts ({base_wr:.1f}% -> {kept_wr:.1f}%)")
    print(f"   - The NN filtered out {(total_filtered_losses - total_filtered_wins)} more losing trades than winning ones!")
    
    # Save
    output = {
        'results': results,
        'summary': {
            'baseline': {'wins': total_baseline_wins, 'losses': total_baseline_losses, 'wr': base_wr},
            'kept': {'wins': total_kept_wins, 'losses': total_kept_losses, 'wr': kept_wr},
            'filtered': {'wins': total_filtered_wins, 'losses': total_filtered_losses, 'wr': filtered_wr},
        }
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN17', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN17/run17_detailed.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print("\nSaved to archive/RUN17/run17_detailed.json")


if __name__ == '__main__':
    main()
