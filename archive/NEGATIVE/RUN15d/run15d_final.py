"""
RUN15d v4: FULL YEAR - PROPER PnL

Win: +1% * 3x leverage = +3%
Loss: -1%
Expected with 50% WR: +1% per trade
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
LEVERAGE = 3
RISK_PCT = 0.01  # 1% risk per trade


def load_1m_data(coin: str) -> pd.DataFrame:
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df


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
    
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
    return feat


def generate_signals(features: pd.DataFrame) -> pd.Series:
    signals = pd.Series(0, index=features.index)
    
    vol_spike_long = (features['vol_ratio'] > 3.5) & (features['rsi'] < 20)
    vol_spike_short = (features['vol_ratio'] > 3.5) & (features['rsi'] > 80)
    
    stoch_k = features['stoch_k']
    stoch_d = stoch_k.rolling(3).mean()
    stoch_k_prev = stoch_k.shift(1)
    stoch_d_prev = stoch_d.shift(1)
    
    stoch_cross_long = (stoch_k_prev <= stoch_d_prev) & (stoch_k > stoch_d) & (stoch_k < 20)
    stoch_cross_short = (stoch_k_prev >= stoch_d_prev) & (stoch_k < stoch_d) & (stoch_k > 80)
    
    signals[vol_spike_long | stoch_cross_long] = 1
    signals[vol_spike_short | stoch_cross_short] = -1
    
    return signals


def run_test(coin: str, idea: int, use_nn: bool = False) -> dict:
    df = load_1m_data(coin)
    features = compute_features(df)
    signals = generate_signals(features)
    c = df['close']
    
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    
    # Apply filters
    if idea == 1:
        hour = features['hour_of_day']
        signals[(hour < 14) | (hour > 21)] = 0
    elif idea == 3:
        vol = features['vol_ratio']
        signals[vol < 2.0] = 0
    
    valid = ~(features.isna().any(axis=1)) & (signals != 0) & future_return.notna()
    X = features[valid].values
    dirs = signals[valid].values
    rets = future_return[valid].values
    
    if len(X) < 100:
        return {'wr': 0, 'signals': 0, 'pnl': 0}
    
    split = int(len(X) * 0.5)
    
    wins = losses = 0
    
    # Simple PnL: each trade risks 1%, wins get 3% (3x leverage)
    for i in range(split, len(X)):
        d = dirs[i]
        r = rets[i]
        
        is_win = (d == 1 and r > 0) or (d == -1 and r < 0)
        
        if is_win:
            wins += 1
        else:
            losses += 1
    
    total = wins + losses
    wr = wins / total * 100 if total > 0 else 0
    
    # PnL: assume avg win = 1% * 3x = 3%, avg loss = 1%
    avg_win_pct = 1.0 * LEVERAGE  # 3%
    avg_loss_pct = RISK_PCT * 100  # 1%
    
    pnl = (wins * avg_win_pct) - (losses * avg_loss_pct)
    
    # With NN
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
        
        nn_wins = nn_losses = 0
        
        for i in range(len(prob_long)):
            idx = i + split
            d = dirs[idx]
            prob = prob_long[i] if d == 1 else prob_short[i]
            
            if prob > 0.55:
                is_win = (d == 1 and rets[idx] > 0) or (d == -1 and rets[idx] < 0)
                if is_win:
                    nn_wins += 1
                else:
                    nn_losses += 1
        
        nn_total = nn_wins + nn_losses
        nn_wr = nn_wins / nn_total * 100 if nn_total > 0 else 0
        nn_pnl = (nn_wins * avg_win_pct) - (nn_losses * avg_loss_pct)
        
        return {'wr': wr, 'signals': total, 'pnl': pnl,
                'nn_wr': nn_wr, 'nn_signals': nn_total, 'nn_pnl': nn_pnl}
    
    return {'wr': wr, 'signals': total, 'pnl': pnl}


def main():
    print("=" * 70)
    print("RUN15d v4: FULL YEAR - PROPER PnL")
    print("=" * 70)
    print("Win: +3% (1% * 3x), Loss: -1%")
    print()
    
    ideas = [(0, "Baseline"), (1, "Time-of-Day"), (3, "Liquidity")]
    
    results = {}
    nn_results = {}
    
    # Without NN
    print("=== WITHOUT NN ===\n")
    for idea_id, idea_name in ideas:
        print(f"{idea_name}:")
        tw = tl = tp = 0
        for coin in COINS:
            try:
                r = run_test(coin, idea_id, use_nn=False)
                if r['signals'] > 0:
                    print(f"  {coin}: WR={r['wr']:.1f}%, {r['signals']} sig, PnL={r['pnl']:+.1f}%")
                    tl += r['signals']
                    tw += int(r['wr'] * r['signals'] / 100)
                    tp += r['pnl']
            except:
                pass
        if tl > 0:
            print(f"  → TOT: WR={tw/tl*100:.1f}%, {tl} sig, PnL={tp:+.1f}%\n")
            results[idea_name] = {'wr': tw/tl*100, 'signals': tl, 'pnl': tp}
    
    # With NN
    print("=== WITH NN ===\n")
    for idea_id, idea_name in ideas:
        name = idea_name + " + NN"
        print(f"{name}:")
        tw = tl = tp = 0
        for coin in COINS:
            try:
                r = run_test(coin, idea_id, use_nn=True)
                if r.get('nn_signals', 0) > 0:
                    print(f"  {coin}: WR={r['nn_wr']:.1f}%, {r['nn_signals']} sig, PnL={r['nn_pnl']:+.1f}%")
                    tl += r['nn_signals']
                    tw += int(r['nn_wr'] * r['nn_signals'] / 100)
                    tp += r['nn_pnl']
            except:
                pass
        if tl > 0:
            print(f"  → TOT: WR={tw/tl*100:.1f}%, {tl} sig, PnL={tp:+.1f}%\n")
            nn_results[name] = {'wr': tw/tl*100, 'signals': tl, 'pnl': tp}
    
    # Summary
    print("=" * 70)
    print("CORRECTED RESULTS")
    print("=" * 70)
    print(f"{'Approach':<25} {'WR%':>8} {'Signals':>10} {'PnL%':>10}")
    print("-" * 55)
    
    all_res = {**results, **nn_results}
    for name, data in sorted(all_res.items(), key=lambda x: x[1]['wr'], reverse=True):
        print(f"{name:<25} {data['wr']:>7.1f}% {data['signals']:>10} {data['pnl']:>+9.1f}%")
    
    output = {'without_nn': results, 'with_nn': nn_results}
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN15d', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN15d/run15d_final.json', 'w') as f:
        json.dump(output, f, indent=2)
    print("\nSaved!")


if __name__ == '__main__':
    main()
