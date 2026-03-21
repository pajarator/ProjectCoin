"""
RUN15d v2: Test 7 Ideas - FULL YEAR Backtest

Tests the full 1 year of data for all coins.
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


def load_1m_data_full(coin: str) -> pd.DataFrame:
    """Load FULL year of 1m data."""
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    print(f"    {coin}: {len(df)} bars (full year)")
    return df  # FULL YEAR - no tail!


def compute_features(df: pd.DataFrame) -> pd.DataFrame:
    feat = pd.DataFrame(index=df.index)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']
    
    # RSI
    delta = c.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    feat['rsi'] = (100 - (100 / (1 + rs))).fillna(50)
    
    # Volume
    v_ma = v.rolling(20).mean()
    feat['vol_ratio'] = v / v_ma
    
    # Stochastic
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    feat['stoch_d'] = feat['stoch_k'].rolling(3).mean()
    
    # Time
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
    return feat


def generate_signals(features: pd.DataFrame) -> pd.Series:
    signals = pd.Series(0, index=features.index)
    
    vol_spike_long = (features['vol_ratio'] > 3.5) & (features['rsi'] < 20)
    vol_spike_short = (features['vol_ratio'] > 3.5) & (features['rsi'] > 80)
    
    stoch_k = features['stoch_k']
    stoch_d = features['stoch_d']
    stoch_k_prev = stoch_k.shift(1)
    stoch_d_prev = stoch_d.shift(1)
    stoch_cross_long = (stoch_k_prev <= stoch_d_prev) & (stoch_k > stoch_d) & (stoch_k < 20)
    stoch_cross_short = (stoch_k_prev >= stoch_d_prev) & (stoch_k < stoch_d) & (stoch_k > 80)
    
    signals[vol_spike_long | stoch_cross_long] = 1
    signals[vol_spike_short | stoch_cross_short] = -1
    
    return signals


def run_test(coin: str, idea: int, use_nn: bool = False) -> dict:
    """Test one coin with one idea."""
    
    df = load_1m_data_full(coin)
    features = compute_features(df)
    signals = generate_signals(features)
    c = df['close']
    
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    
    # Apply filters based on idea
    if idea == 1:  # Time-of-Day
        hour = features['hour_of_day']
        signals[(hour < 14) | (hour > 21)] = 0
    elif idea == 3:  # Liquidity
        vol = features['vol_ratio']
        signals[vol < 2.0] = 0
    
    # Clean
    valid = ~(features.isna().any(axis=1)) & (signals != 0) & future_return.notna()
    X = features[valid].values
    dirs = signals[valid].values
    rets = future_return[valid].values
    
    if len(X) < 100:
        return {'wr': 0, 'signals': 0, 'pnl': 0}
    
    # Walk-forward: train first 50%, test second 50%
    split = int(len(X) * 0.5)
    
    wins = 0
    losses = 0
    pnl = 0
    
    for i in range(split, len(X)):
        d = dirs[i]
        r = rets[i]
        if d == 1 and r > 0:
            wins += 1
            pnl += r
        elif d == 1 and r <= 0:
            losses += 1
        elif d == -1 and r < 0:
            wins += 1
            pnl -= r
        elif d == -1 and r >= 0:
            losses += 1
    
    total = wins + losses
    wr = wins / total * 100 if total > 0 else 0
    
    # With NN filter
    if use_nn and total > 50:
        y_long = (rets[:split] > 0).astype(int)
        y_short = (rets[:split] < 0).astype(int)
        
        scaler = StandardScaler()
        X_s = scaler.fit_transform(X[:split])
        X_test_s = scaler.transform(X[split:])
        
        model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        model_long.fit(X_s, y_long)
        model_short.fit(X_s, y_short)
        
        prob_long = model_long.predict_proba(X_test_s)[:, 1]
        prob_short = model_short.predict_proba(X_test_s)[:, 1]
        
        nn_wins = 0
        nn_losses = 0
        nn_pnl = 0
        
        for i in range(len(prob_long)):
            idx = i + split
            d = dirs[idx]
            r = rets[idx]
            prob = prob_long[i] if d == 1 else prob_short[i]
            
            if prob > 0.55:
                if d == 1 and r > 0:
                    nn_wins += 1
                    nn_pnl += r
                elif d == 1 and r <= 0:
                    nn_losses += 1
                elif d == -1 and r < 0:
                    nn_wins += 1
                    nn_pnl -= r
                elif d == -1 and r >= 0:
                    nn_losses += 1
        
        nn_total = nn_wins + nn_losses
        nn_wr = nn_wins / nn_total * 100 if nn_total > 0 else 0
        
        return {'wr': wr, 'signals': total, 'pnl': pnl*100, 
                'nn_wr': nn_wr, 'nn_signals': nn_total, 'nn_pnl': nn_pnl*100}
    
    return {'wr': wr, 'signals': total, 'pnl': pnl*100}


def main():
    print("=" * 80)
    print("RUN15d v2: FULL YEAR Backtest - All Ideas")
    print("=" * 80)
    print()
    
    ideas = [
        (0, "Baseline"),
        (1, "Time-of-Day"),
        (3, "Liquidity"),
    ]
    
    results = {}
    nn_results = {}
    
    # Test WITHOUT NN
    print("=== WITHOUT NN FILTER ===\n")
    for idea_id, idea_name in ideas:
        print(f"{idea_name}:")
        tw = tl = tp = 0
        for coin in COINS:
            try:
                r = run_test(coin, idea_id, use_nn=False)
                print(f"  {coin}: WR={r['wr']:.1f}%, {r['signals']} signals")
                tl += r['signals']
                tw += int(r['wr'] * r['signals'] / 100)
                tp += r['pnl']
            except Exception as e:
                print(f"  {coin}: ERROR")
        
        if tl > 0:
            print(f"  → TOTAL: WR={tw/tl*100:.1f}%, {tl} signals, PnL={tp:+.1f}%\n")
            results[idea_name] = {'wr': tw/tl*100, 'signals': tl, 'pnl': tp}
    
    # Test WITH NN
    print("=== WITH NN FILTER (threshold=0.55) ===\n")
    for idea_id, idea_name in ideas:
        print(f"{idea_name} + NN:")
        tw = tl = tp = 0
        for coin in COINS:
            try:
                r = run_test(coin, idea_id, use_nn=True)
                if r.get('nn_signals', 0) > 0:
                    print(f"  {coin}: WR={r['nn_wr']:.1f}%, {r['nn_signals']} signals")
                    tl += r['nn_signals']
                    tw += int(r['nn_wr'] * r['nn_signals'] / 100)
                    tp += r['nn_pnl']
            except Exception as e:
                print(f"  {coin}: ERROR")
        
        if tl > 0:
            print(f"  → TOTAL: WR={tw/tl*100:.1f}%, {tl} signals, PnL={tp:+.1f}%\n")
            nn_results[idea_name + " + NN"] = {'wr': tw/tl*100, 'signals': tl, 'pnl': tp}
    
    # Summary
    print("=" * 80)
    print("FULL YEAR SUMMARY")
    print("=" * 80)
    print()
    print(f"{'Approach':<25} {'WR%':>10} {'Signals':>12} {'PnL%':>12}")
    print("-" * 60)
    
    all_results = {**results, **nn_results}
    sorted_results = sorted(all_results.items(), key=lambda x: x[1]['wr'], reverse=True)
    
    for name, data in sorted_results:
        print(f"{name:<25} {data['wr']:>9.1f}% {data['signals']:>12} {data['pnl']:>+11.1f}%")
    
    # Save
    output = {
        'version': 'v2_full_year',
        'without_nn': results,
        'with_nn': nn_results,
        'best': sorted_results[0][0] if sorted_results else None
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN15d', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN15d/run15d_full_year.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print(f"\nBest: {sorted_results[0][0]}")
    print("Saved to archive/RUN15d/run15d_full_year.json")


if __name__ == '__main__':
    main()
