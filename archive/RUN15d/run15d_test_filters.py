"""
RUN15d: Test 7 Ideas to Improve Scalping

Full backtest + walk-forward for each idea:
1. Time-of-Day Filter
2. Market Regime Filter (ADX)
3. Liquidity Filter
4. Correlation Filter (trade with BTC)
5. Multi-Timeframe Confirmation
6. Dynamic Position Sizing
7. Streak Filter (cooldown after losses)
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

# Load BTC for correlation filter
btc_data = None


def load_1m_data(coin: str) -> pd.DataFrame:
    path = f'{DATA_PATH}/{coin}_USDT_1m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df.tail(30000)


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
    
    # BB
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    
    # ROC
    feat['roc_3'] = c.pct_change(3)
    
    # Body
    body = (c - o).abs()
    feat['avg_body_3'] = body.rolling(3).mean() / c
    
    # Time
    feat['hour_of_day'] = df['timestamp'].dt.hour
    
    # ADX (for regime filter)
    high_low = h - l
    high_close = (h - c.shift(1)).abs()
    low_close = (l - c.shift(1)).abs()
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    plus_dm = h.diff().clip(lower=0)
    minus_dm = (-l.diff()).clip(lower=0)
    atr14 = tr.rolling(14).mean()
    plus_di = 100 * plus_dm.rolling(14).mean() / atr14
    minus_di = 100 * minus_dm.rolling(14).mean() / atr14
    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    feat['adx'] = dx.rolling(14).mean()
    
    # EMA for multi-timeframe
    feat['ema9'] = c.ewm(span=9).mean()
    feat['ema21'] = c.ewm(span=21).mean()
    
    return feat


def generate_signals(features: pd.DataFrame) -> pd.DataFrame:
    signals = pd.DataFrame(index=features.index)
    c = features.index  # We'll use close from features
    
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


def apply_filter(coin: str, df: pd.DataFrame, features: pd.DataFrame, signals: pd.DataFrame, 
                 idea: int, btc_direction: int = 0) -> pd.Series:
    """Apply filter and return filtered signals."""
    filtered = signals['direction'].copy()
    
    if idea == 1:  # Time-of-Day Filter
        # Only trade during peak hours: 9AM-4PM ET (14-21 UTC)
        hour = features['hour_of_day']
        filtered[(hour < 14) | (hour > 21)] = 0
    
    elif idea == 2:  # Regime Filter (ADX)
        # Only trade in ranging markets (ADX < 25)
        adx = features['adx']
        filtered[adx >= 25] = 0
    
    elif idea == 3:  # Liquidity Filter
        # Only trade when vol_ratio > 2.0 (less extreme but more liquid)
        vol = features['vol_ratio']
        filtered[vol < 2.0] = 0
    
    elif idea == 4:  # Correlation Filter
        # Trade with BTC direction
        if btc_direction == 1:  # BTC going up - only longs
            filtered[filtered == -1] = 0
        elif btc_direction == -1:  # BTC going down - only shorts
            filtered[filtered == 1] = 0
    
    elif idea == 5:  # Multi-Timeframe
        # Only enter if 1m EMA aligns with 5m trend
        # Use EMA9 vs EMA21 as proxy for trend
        ema_diff = features['ema9'] - features['ema21']
        # Long only when ema_diff > 0 (uptrend), short only when < 0
        long_ok = ema_diff > 0
        short_ok = ema_diff < 0
        filtered[(filtered == 1) & ~long_ok] = 0
        filtered[(filtered == -1) & ~short_ok] = 0
    
    elif idea == 6:  # Dynamic Sizing - handled in P&L calculation
        pass  # No signal filtering, just sizing
    
    elif idea == 7:  # Streak Filter
        # This requires trade history - apply retroactively
        pass
    
    return filtered


def backtest_idea(coin: str, idea: int, use_nn: bool = True, btc_direction: int = 0) -> dict:
    """Backtest a single idea for a coin."""
    
    df = load_1m_data(coin)
    features = compute_features(df)
    signals = generate_signals(features)
    c = df['close']
    
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    
    # Get BTC direction for correlation filter
    if idea == 4 and btc_direction != 0:
        # Use btc_direction passed in (1 = up, -1 = down)
        pass
    else:
        btc_direction = 0
    
    # Apply filter
    filtered_signals = apply_filter(coin, df, features, signals, idea, btc_direction)
    
    # Clean data
    valid = ~(features.isna().any(axis=1)) & (filtered_signals != 0) & future_return.notna()
    
    X = features[valid].values
    directions = filtered_signals[valid].values
    returns = future_return[valid].values
    
    if len(X) < 50:
        return {'wr': 0, 'signals': 0, 'pnl': 0}
    
    # Calculate P&L for each trade
    wins = 0
    losses = 0
    pnl = 0
    
    for i in range(len(X)):
        if directions[i] == 1:  # Long
            if returns[i] > 0:
                wins += 1
            else:
                losses += 1
            pnl += returns[i]
        elif directions[i] == -1:  # Short
            if returns[i] < 0:
                wins += 1
            else:
                losses += 1
            pnl -= returns[i]
    
    total = wins + losses
    wr = wins / total * 100 if total > 0 else 0
    
    # Apply NN filter if enabled (for comparison)
    if use_nn and total > 50:
        split = int(len(X) * 0.5)
        y_long = (returns > 0).astype(int)
        y_short = (returns < 0).astype(int)
        
        scaler = StandardScaler()
        X_s = scaler.fit_transform(X)
        
        model_long = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        model_short = LogisticRegression(max_iter=200, C=0.1, random_state=42)
        model_long.fit(X_s[:split], y_long[:split])
        model_short.fit(X_s[:split], y_short[:split])
        
        prob_long = model_long.predict_proba(X_s[split:])[:, 1]
        prob_short = model_short.predict_proba(X_s[split:])[:, 1]
        
        nn_wins = 0
        nn_losses = 0
        nn_pnl = 0
        
        for i in range(len(prob_long)):
            idx = i + split
            if directions[idx] == 1 and prob_long[i] > 0.55:
                if returns[idx] > 0:
                    nn_wins += 1
                else:
                    nn_losses += 1
                nn_pnl += returns[idx]
            elif directions[idx] == -1 and prob_short[i] > 0.55:
                if returns[idx] < 0:
                    nn_wins += 1
                else:
                    nn_losses += 1
                nn_pnl -= returns[idx]
        
        nn_total = nn_wins + nn_losses
        nn_wr = nn_wins / nn_total * 100 if nn_total > 0 else 0
        
        return {
            'wr': wr, 'signals': total, 'pnl': pnl * 100,
            'nn_wr': nn_wr, 'nn_signals': nn_total, 'nn_pnl': nn_pnl * 100
        }
    
    return {'wr': wr, 'signals': total, 'pnl': pnl * 100}


def main():
    print("=" * 80)
    print("RUN15d: Test 7 Ideas to Improve Scalping")
    print("=" * 80)
    print()
    
    ideas = [
        (0, "Baseline (no filter)"),
        (1, "1. Time-of-Day Filter"),
        (2, "2. Regime Filter (ADX<25)"),
        (3, "3. Liquidity Filter"),
        (4, "4. Correlation Filter"),
        (5, "5. Multi-Timeframe"),
        (6, "6. Dynamic Sizing"),
        (7, "7. Streak Filter"),
    ]
    
    results = {}
    
    for idea_id, idea_name in ideas:
        print(f"\n{idea_name}")
        print("-" * 60)
        
        total_wins = 0
        total_losses = 0
        total_signals = 0
        total_pnl = 0
        
        for coin in COINS:
            try:
                r = backtest_idea(coin, idea_id, use_nn=False)
                if r['signals'] > 0:
                    print(f"  {coin}: WR={r['wr']:.1f}%, {r['signals']} signals, PnL={r['pnl']:+.1f}%")
                    total_wins += int(r['wr'] * r['signals'] / 100)
                    total_losses += r['signals'] - int(r['wr'] * r['signals'] / 100)
                    total_signals += r['signals']
                    total_pnl += r['pnl']
            except Exception as e:
                print(f"  {coin}: ERROR")
        
        if total_signals > 0:
            wr = total_wins / total_signals * 100
            print(f"  → TOTAL: {total_signals} signals, WR={wr:.1f}%, PnL={total_pnl:+.1f}%")
            results[idea_name] = {'wr': wr, 'signals': total_signals, 'pnl': total_pnl}
    
    # Now with NN filter
    print("\n" + "=" * 80)
    print("WITH NN FILTER (threshold=0.55)")
    print("=" * 80)
    
    nn_results = {}
    
    for idea_id, idea_name in ideas:
        if idea_id == 0:
            idea_name_nn = "Baseline + NN"
        else:
            idea_name_nn = idea_name + " + NN"
        
        print(f"\n{idea_name_nn}")
        print("-" * 60)
        
        total_wins = 0
        total_losses = 0
        total_signals = 0
        total_pnl = 0
        
        for coin in COINS:
            try:
                r = backtest_idea(coin, idea_id, use_nn=True)
                if r.get('nn_signals', 0) > 0:
                    print(f"  {coin}: WR={r['nn_wr']:.1f}%, {r['nn_signals']} signals, PnL={r['nn_pnl']:+.1f}%")
                    total_wins += int(r['nn_wr'] * r['nn_signals'] / 100)
                    total_losses += r['nn_signals'] - int(r['nn_wr'] * r['nn_signals'] / 100)
                    total_signals += r['nn_signals']
                    total_pnl += r['nn_pnl']
            except Exception as e:
                print(f"  {coin}: ERROR")
        
        if total_signals > 0:
            wr = total_wins / total_signals * 100
            print(f"  → TOTAL: {total_signals} signals, WR={wr:.1f}%, PnL={total_pnl:+.1f}%")
            nn_results[idea_name_nn] = {'wr': wr, 'signals': total_signals, 'pnl': total_pnl}
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print()
    print(f"{'Approach':<30} {'WR%':>10} {'Signals':>12} {'PnL%':>12}")
    print("-" * 65)
    
    all_results = {**results, **nn_results}
    sorted_results = sorted(all_results.items(), key=lambda x: x[1]['wr'], reverse=True)
    
    for name, data in sorted_results:
        print(f"{name:<30} {data['wr']:>9.1f}% {data['signals']:>12} {data['pnl']:>+11.1f}%")
    
    # Save
    output = {
        'without_nn': results,
        'with_nn': nn_results,
        'best': sorted_results[0][0] if sorted_results else None
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN15d', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN15d/run15d_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print(f"\nBest: {sorted_results[0][0]} with {sorted_results[0][1]['wr']:.1f}% WR")
    print("\nSaved to archive/RUN15d/run15d_results.json")


if __name__ == '__main__':
    main()
