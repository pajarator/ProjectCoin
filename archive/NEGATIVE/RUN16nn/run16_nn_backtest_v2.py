"""
RUN16: Neural Network Mean Reversion Filter (v2)

Key insight: Instead of predicting direction from scratch, use the NN to FILTER
the existing mean reversion signals (z < -1.5). The NN learns which signal
instances are more likely to succeed.

Architecture:
- Input: 20 features when mean reversion signal is present
- Train: Binary classification (will this MR signal be profitable?)
- Output: Probability of success - only enter if P(success) > threshold

This is a meta-learner that enhances the existing strategy.
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.neural_network import MLPClassifier
from sklearn.preprocessing import StandardScaler
from collections import defaultdict
import warnings
warnings.filterwarnings('ignore')

# Config
DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['DASH', 'UNI', 'NEAR', 'ADA', 'LTC', 'SHIB', 'LINK', 'ETH',
         'BTC', 'BNB', 'XRP', 'SOL', 'DOT', 'AVAX', 'ATOM', 'DOGE']
TRAIN_RATIO = 0.5
LOOKAHEAD = 4  # Predict 4 bars ahead (1 hour)
Z_THRESHOLD = -1.5  # Mean reversion threshold

# Feature columns
FEATURES = [
    'z_score', 'rsi_14', 'rsi_7', 'bb_position', 'atr_pct',
    'volatility_20', 'stoch_k', 'stoch_d', 'macd_hist_norm',
    'cci_20', 'momentum_10', 'volume_ratio', 'obv_slope', 'cmf_20',
    'adx_14', 'aroon_up', 'aroon_down', 'laguerre_rsi', 'kalman_dist',
    'kst', 'kst_signal', 'hour_of_day', 'day_of_week'
]


def load_coin_data(coin: str) -> pd.DataFrame:
    """Load cached OHLCV data."""
    path = f'{DATA_PATH}/{coin}_USDT_15m_1year.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    return df


def compute_features(df: pd.DataFrame) -> pd.DataFrame:
    """Compute features for the dataframe."""
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
    
    gain7 = delta.where(delta > 0, 0).rolling(7).mean()
    loss7 = (-delta.where(delta < 0, 0)).rolling(7).mean()
    rs7 = gain7 / loss7.replace(0, np.nan)
    feat['rsi_7'] = (100 - (100 / (1 + rs7))).fillna(50)
    
    # BB
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    
    # ATR
    high_low = h - l
    high_close = (h - c.shift(1)).abs()
    low_close = (l - c.shift(1)).abs()
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    atr = tr.rolling(14).mean()
    feat['atr_pct'] = atr / c
    
    # Volatility
    feat['volatility_20'] = c.pct_change().rolling(20).std()
    
    # Stochastic
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    feat['stoch_d'] = feat['stoch_k'].rolling(3).mean()
    
    # MACD
    ema12 = c.ewm(span=12).mean()
    ema26 = c.ewm(span=26).mean()
    macd = ema12 - ema26
    signal = macd.ewm(span=9).mean()
    feat['macd_hist_norm'] = (macd - signal) / c
    
    # CCI
    tp = (h + l + c) / 3
    sma_tp = tp.rolling(20).mean()
    mad = (tp - sma_tp).abs().rolling(20).mean()
    feat['cci_20'] = (tp - sma_tp) / (0.015 * mad).replace(0, np.nan)
    
    # Momentum
    feat['momentum_10'] = c.pct_change(10)
    
    # Volume
    v_ma = v.rolling(20).mean()
    feat['volume_ratio'] = v / v_ma
    
    obv = (np.sign(c.diff()) * v).cumsum()
    feat['obv_slope'] = obv.rolling(5).apply(lambda x: np.polyfit(range(len(x)), x, 1)[0] if len(x) > 1 else 0)
    
    mf = ((c - l) - (h - c)) / (h - l).replace(0, np.nan)
    mf = mf.fillna(0) * v
    feat['cmf_20'] = mf.rolling(20).sum() / v.rolling(20).sum()
    
    # ADX
    plus_dm = h.diff()
    minus_dm = -l.diff()
    plus_dm = plus_dm.where((plus_dm > minus_dm) & (plus_dm > 0), 0)
    minus_dm = minus_dm.where((minus_dm > plus_dm) & (minus_dm > 0), 0)
    atr14 = tr.rolling(14).mean()
    plus_di = 100 * plus_dm.rolling(14).mean() / atr14
    minus_di = 100 * minus_dm.rolling(14).mean() / atr14
    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    feat['adx_14'] = dx.rolling(14).mean()
    
    # Aroon
    aroon_up = c.rolling(25).apply(lambda x: np.argmax(x) / 24 * 100, raw=True)
    aroon_down = c.rolling(25).apply(lambda x: np.argmin(x) / 24 * 100, raw=True)
    feat['aroon_up'] = aroon_up
    feat['aroon_down'] = aroon_down
    
    # Laguerre RSI
    gamma = 0.8
    lrs = pd.Series(index=c.index, dtype=float)
    lrs.iloc[0] = 0.5
    for i in range(1, len(c)):
        lrs.iloc[i] = gamma * lrs.iloc[i-1] + (1 - gamma) * (c.iloc[i] - c.iloc[i-1]) / c.iloc[i-1]
    feat['laguerre_rsi'] = (lrs - lrs.rolling(8).min()) / (lrs.rolling(8).max() - lrs.rolling(8).min()).replace(0, np.nan)
    feat['laguerre_rsi'] = feat['laguerre_rsi'].fillna(0.5)
    
    # Kalman
    kf_est = pd.Series(index=c.index, dtype=float)
    kf_err = pd.Series(index=c.index, dtype=float)
    kf_est.iloc[0] = c.iloc[0]
    kf_err.iloc[0] = 1.0
    Q = 0.0001
    R = 0.01
    for i in range(1, len(c)):
        pred = kf_est.iloc[i-1]
        err = np.sqrt(kf_err.iloc[i-1] + Q)
        k = err / (err + R)
        kf_est.iloc[i] = pred + k * (c.iloc[i] - pred)
        kf_err.iloc[i] = (1 - k) * err
    feat['kalman_dist'] = (c - kf_est) / kf_err
    
    # KST
    rocma1 = c.pct_change(10).rolling(10).mean()
    rocma2 = c.pct_change(15).rolling(10).mean()
    rocma3 = c.pct_change(20).rolling(10).mean()
    rocma4 = c.pct_change(30).rolling(10).mean()
    feat['kst'] = (rocma1 * 1 + rocma2 * 2 + rocma3 * 3 + rocma4 * 4) / 10
    feat['kst_signal'] = feat['kst'].rolling(9).mean()
    
    # Time
    feat['hour_of_day'] = df['timestamp'].dt.hour
    feat['day_of_week'] = df['timestamp'].dt.dayofweek
    
    return feat


def run_backtest_v2(coin: str, prob_threshold: float = 0.55) -> dict:
    """Run backtest for a single coin - NN filters MR signals."""
    print(f"  {coin}...", end=" ")
    
    # Load and compute features
    df = load_coin_data(coin)
    features = compute_features(df)
    c = df['close']
    
    # Create signals and targets
    mr_signal = features['z_score'] < Z_THRESHOLD  # Original MR signal
    
    # Future return
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    profitable = (future_return > 0)  # Was the trade profitable?
    
    # Clean data - only keep rows where MR signal fires
    valid = ~(features.isna().any(axis=1)) & mr_signal & future_return.notna()
    
    X = features[valid].values
    y = profitable[valid].astype(int).values  # 1 = profitable, 0 = not
    
    if len(X) < 200:
        print(f"skip (only {len(X)} MR signals)")
        return None
    
    # Split: train on first half, test on second half
    split = int(len(X) * TRAIN_RATIO)
    X_train, X_test = X[:split], X[split:]
    y_train, y_test = y[:split], y[split:]
    
    # Scale features
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)
    
    # Train model - predict probability of success
    model = MLPClassifier(
        hidden_layer_sizes=(32, 16),
        activation='relu',
        solver='adam',
        max_iter=300,
        early_stopping=True,
        validation_fraction=0.15,
        random_state=42,
        verbose=False
    )
    model.fit(X_train_scaled, y_train)
    
    # Get probabilities for test set
    probs = model.predict_proba(X_test_scaled)[:, 1]  # P(profitable)
    
    # Filter by threshold
    filtered = probs >= prob_threshold
    
    if filtered.sum() == 0:
        print(f"no signals above threshold {prob_threshold}")
        return {
            'coin': coin,
            'mr_signals': int(len(X)),
            'nn_filtered': 0,
            'win_rate': 0,
            'pnl_pct': 0,
        }
    
    y_filtered = y_test[filtered]
    win_rate = y_filtered.mean() * 100
    
    # P&L simulation
    test_prices = c.values[valid][split:][filtered]
    initial_balance = 10000
    balance = initial_balance
    
    for i, price in enumerate(test_prices):
        if i < len(test_prices) - 1:
            # Simple: assume we hold for LOOKAHEAD bars
            next_price = test_prices[i + 1] if i + 1 < len(test_prices) else test_prices[-1]
            ret = (next_price - price) / price
            balance *= (1 + ret)
    
    pnl_pct = (balance / initial_balance - 1) * 100
    
    print(f"MR: {len(X)}, NN pass: {filtered.sum()}, WR: {win_rate:.1f}%, PnL: {pnl_pct:+.1f}%")
    
    return {
        'coin': coin,
        'mr_signals': int(len(X)),
        'nn_filtered': int(filtered.sum()),
        'nn_rejected': int(len(X) - split - filtered.sum()),
        'win_rate': round(win_rate, 1),
        'pnl_pct': round(pnl_pct, 1),
    }


def run_baseline(coin: str) -> dict:
    """Baseline: plain mean reversion without NN filter."""
    df = load_coin_data(coin)
    c = df['close']
    
    sma20 = c.rolling(20).mean()
    std20 = c.rolling(20).std()
    z = (c - sma20) / std20
    
    mr_signal = z < Z_THRESHOLD
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    profitable = future_return > 0
    
    valid = mr_signal & future_return.notna()
    
    y = profitable[valid].astype(int).values
    
    if len(y) == 0:
        return {'wr': 0, 'signals': 0}
    
    return {'wr': y.mean() * 100, 'signals': len(y)}


def main():
    print("=" * 70)
    print("RUN16 v2: Neural Network Mean Reversion Filter")
    print("=" * 70)
    print()
    print("Strategy: NN filters existing MR signals (z < -1.5)")
    print(f"Training: First 50% of data, Testing: Second 50%")
    print()
    
    results_nn = []
    results_baseline = []
    
    # Test different thresholds
    thresholds = [0.50, 0.55, 0.60]
    
    for threshold in thresholds:
        print(f"\n=== Threshold: {threshold} ===")
        results = []
        for coin in COINS:
            try:
                result = run_backtest_v2(coin, prob_threshold=threshold)
                if result:
                    results.append(result)
            except Exception as e:
                print(f"  {coin}: ERROR - {e}")
        
        if results:
            avg_wr = np.mean([r['win_rate'] for r in results])
            total_signals = sum(r['nn_filtered'] for r in results)
            avg_pnl = np.mean([r['pnl_pct'] for r in results])
            print(f"  → Avg WR: {avg_wr:.1f}%, Signals: {total_signals}, Avg PnL: {avg_pnl:+.1f}%")
            results_nn.append({
                'threshold': threshold,
                'avg_wr': avg_wr,
                'total_signals': total_signals,
                'avg_pnl': avg_pnl,
                'per_coin': results
            })
    
    # Baseline comparison
    print("\n=== BASELINE (no NN filter) ===")
    for coin in COINS:
        try:
            baseline = run_baseline(coin)
            results_baseline.append({
                'coin': coin,
                'wr': baseline['wr'],
                'signals': baseline['signals']
            })
        except:
            pass
    
    baseline_avg_wr = np.mean([r['wr'] for r in results_baseline if r['signals'] > 0])
    baseline_total_signals = sum(r['signals'] for r in results_baseline)
    print(f"  → Avg WR: {baseline_avg_wr:.1f}%, Signals: {baseline_total_signals}")
    
    # Best threshold
    best = max(results_nn, key=lambda x: x['avg_wr'])
    print(f"\n=== BEST: threshold={best['threshold']} ===")
    print(f"  NN Filter WR: {best['avg_wr']:.1f}% vs Baseline: {baseline_avg_wr:.1f}%")
    print(f"  Delta: {best['avg_wr'] - baseline_avg_wr:+.1f} pts")
    print(f"  Signals: {best['total_signals']} (reduced from {baseline_total_signals})")
    
    # Per-coin table for best threshold
    print(f"\n{'COIN':<8} {'MR_SIGNALS':>12} {'NN_PASS':>10} {'WR%':>8} {'PNL%':>10}")
    print("-" * 50)
    for r in sorted(best['per_coin'], key=lambda x: x['win_rate'], reverse=True):
        print(f"{r['coin']:<8} {r['mr_signals']:>12} {r['nn_filtered']:>10} {r['win_rate']:>7.1f}% {r['pnl_pct']:>+9.1f}%")
    
    # Save results
    output = {
        'experiment': 'RUN16',
        'version': 'v2 - NN as MR filter',
        'description': 'NN filters mean reversion signals, only enters when P(success) > threshold',
        'z_threshold': Z_THRESHOLD,
        'lookahead': LOOKAHEAD,
        'results_nn': results_nn,
        'baseline': {
            'avg_wr': baseline_avg_wr,
            'total_signals': baseline_total_signals
        },
        'best_threshold': best['threshold'],
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN16', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN16/run16_v2_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print()
    print("Results saved to /home/scamarena/ProjectCoin/archive/RUN16/run16_v2_results.json")
    
    return output


if __name__ == '__main__':
    main()
