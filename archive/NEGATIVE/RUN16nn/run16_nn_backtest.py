"""
RUN16: Neural Network Mean Reversion Enhancer

Goal: Improve mean reversion strategy using a simple neural network that takes
indicator features as input and outputs buy/hold/sell signals.

Architecture:
- Input: 20-30 selected features (indicators)
- Hidden: 2 layers of 64 neurons with ReLU
- Output: 3 classes (buy/hold/sell) - softmax
- Train: Walk-forward (train on first 6mo, test on next 6mo)

The NN learns patterns in the data that predict whether mean reversion will work.
"""

import pandas as pd
import numpy as np
import json
import os
from sklearn.neural_network import MLPClassifier
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import classification_report
from collections import defaultdict
import warnings
warnings.filterwarnings('ignore')

# Config
DATA_PATH = '/home/scamarena/ProjectCoin/data_cache'
COINS = ['DASH', 'UNI', 'NEAR', 'ADA', 'LTC', 'SHIB', 'LINK', 'ETH',
         'BTC', 'BNB', 'XRP', 'SOL', 'DOT', 'AVAX', 'ATOM', 'DOGE']
TRAIN_RATIO = 0.5
LOOKAHEAD = 4  # Predict 4 bars ahead (1 hour)

# Feature columns to use (subset of full feature matrix)
FEATURES = [
    # Core mean reversion features
    'z_score',
    'rsi_14', 'rsi_7',
    'bb_position',
    'atr_pct',
    'volatility_20',
    
    # Momentum
    'stoch_k', 'stoch_d',
    'macd_hist_norm',
    'cci_20',
    'momentum_10',
    
    # Volume
    'volume_ratio',
    'obv_slope',
    'cmf_20',
    
    # Trend
    'adx_14',
    'aroon_up', 'aroon_down',
    
    # Advanced
    'laguerre_rsi',
    'kalman_dist',
    'kst', 'kst_signal',
    
    # Time
    'hour_of_day',
    'day_of_week',
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
    
    # === Z-SCORE (core mean reversion) ===
    sma20 = c.rolling(20).mean()
    std20 = c.rolling(20).std()
    feat['z_score'] = (c - sma20) / std20
    
    # === RSI ===
    delta = c.diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    feat['rsi_14'] = (100 - (100 / (1 + rs))).fillna(50)
    
    # RSI 7
    gain7 = delta.where(delta > 0, 0).rolling(7).mean()
    loss7 = (-delta.where(delta < 0, 0)).rolling(7).mean()
    rs7 = gain7 / loss7.replace(0, np.nan)
    feat['rsi_7'] = (100 - (100 / (1 + rs7))).fillna(50)
    
    # === BOLLINGER BANDS ===
    bb_sma = c.rolling(20).mean()
    bb_std = c.rolling(20).std()
    bb_upper = bb_sma + 2 * bb_std
    bb_lower = bb_sma - 2 * bb_std
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower).replace(0, np.nan)
    
    # === ATR ===
    high_low = h - l
    high_close = (h - c.shift(1)).abs()
    low_close = (l - c.shift(1)).abs()
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    atr = tr.rolling(14).mean()
    feat['atr_pct'] = atr / c
    
    # === VOLATILITY ===
    feat['volatility_20'] = c.pct_change().rolling(20).std()
    
    # === STOCHASTIC ===
    low14 = l.rolling(14).min()
    high14 = h.rolling(14).max()
    feat['stoch_k'] = 100 * (c - low14) / (high14 - low14).replace(0, np.nan)
    feat['stoch_d'] = feat['stoch_k'].rolling(3).mean()
    
    # === MACD ===
    ema12 = c.ewm(span=12).mean()
    ema26 = c.ewm(span=26).mean()
    macd = ema12 - ema26
    signal = macd.ewm(span=9).mean()
    feat['macd_hist_norm'] = (macd - signal) / c
    
    # === CCI ===
    tp = (h + l + c) / 3
    sma_tp = tp.rolling(20).mean()
    mad = (tp - sma_tp).abs().rolling(20).mean()
    feat['cci_20'] = (tp - sma_tp) / (0.015 * mad).replace(0, np.nan)
    
    # === MOMENTUM ===
    feat['momentum_10'] = c.pct_change(10)
    
    # === VOLUME ===
    v_ma = v.rolling(20).mean()
    feat['volume_ratio'] = v / v_ma
    
    # OBV
    obv = (np.sign(c.diff()) * v).cumsum()
    feat['obv_slope'] = obv.rolling(5).apply(lambda x: np.polyfit(range(len(x)), x, 1)[0] if len(x) > 1 else 0)
    
    # CMF
    mf = ((c - l) - (h - c)) / (h - l).replace(0, np.nan)
    mf = mf.fillna(0) * v
    feat['cmf_20'] = mf.rolling(20).sum() / v.rolling(20).sum()
    
    # === ADX ===
    plus_dm = h.diff()
    minus_dm = -l.diff()
    plus_dm = plus_dm.where((plus_dm > minus_dm) & (plus_dm > 0), 0)
    minus_dm = minus_dm.where((minus_dm > plus_dm) & (minus_dm > 0), 0)
    atr14 = tr.rolling(14).mean()
    plus_di = 100 * plus_dm.rolling(14).mean() / atr14
    minus_di = 100 * minus_dm.rolling(14).mean() / atr14
    dx = 100 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    feat['adx_14'] = dx.rolling(14).mean()
    
    # === AROON ===
    aroon_up = c.rolling(25).apply(lambda x: np.argmax(x) / 24 * 100, raw=True)
    aroon_down = c.rolling(25).apply(lambda x: np.argmin(x) / 24 * 100, raw=True)
    feat['aroon_up'] = aroon_up
    feat['aroon_down'] = aroon_down
    
    # === LAGUERRE RSI ===
    gamma = 0.8
    lrs = pd.Series(index=c.index, dtype=float)
    lrs.iloc[0] = 0.5
    for i in range(1, len(c)):
        lrs.iloc[i] = gamma * lrs.iloc[i-1] + (1 - gamma) * (c.iloc[i] - c.iloc[i-1]) / c.iloc[i-1]
    feat['laguerre_rsi'] = (lrs - lrs.rolling(8).min()) / (lrs.rolling(8).max() - lrs.rolling(8).min()).replace(0, np.nan)
    feat['laguerre_rsi'] = feat['laguerre_rsi'].fillna(0.5)
    
    # === KALMAN DISTANCE ===
    # Simple kalman-like filter
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
    
    # === KST ===
    def roc_sum(periods):
        return sum(c.pct_change(p) for p in periods)
    rocma1 = c.pct_change(10).rolling(10).mean()
    rocma2 = c.pct_change(15).rolling(10).mean()
    rocma3 = c.pct_change(20).rolling(10).mean()
    rocma4 = c.pct_change(30).rolling(10).mean()
    feat['kst'] = (rocma1 * 1 + rocma2 * 2 + rocma3 * 3 + rocma4 * 4) / 10
    feat['kst_signal'] = feat['kst'].rolling(9).mean()
    
    # === TIME FEATURES ===
    feat['hour_of_day'] = df['timestamp'].dt.hour
    feat['day_of_week'] = df['timestamp'].dt.dayofweek
    
    return feat


def create_targets(df: pd.DataFrame, lookahead: int = 4) -> pd.Series:
    """Create target: 1 = buy (price goes up), 0 = hold, -1 = sell (price goes down)."""
    future_return = c = df['close'].pct_change(lookahead).shift(-lookahead)
    
    # Simple: 1 if positive return, -1 if negative
    target = pd.Series(index=df.index, dtype=int)
    target[future_return > 0.001] = 1   # Buy signal
    target[future_return < -0.001] = -1  # Sell signal
    target[(future_return >= -0.001) & (future_return <= 0.001)] = 0  # Hold
    
    return target


def run_backtest(coin: str, model) -> dict:
    """Run backtest for a single coin."""
    print(f"  {coin}...", end=" ")
    
    # Load and compute features
    df = load_coin_data(coin)
    features = compute_features(df)
    
    # Create target
    c = df['close']
    future_return = c.pct_change(LOOKAHEAD).shift(-LOOKAHEAD)
    target = np.select(
        [future_return > 0.001, future_return < -0.001],
        [1, -1],
        default=0
    )
    
    # Clean data
    valid_idx = ~(features.isna().any(axis=1) | np.isnan(target))
    X = features[valid_idx].values
    y = target[valid_idx]
    
    if len(X) < 1000:
        print(f"skip (only {len(X)} samples)")
        return None
    
    # Split: train on first half, test on second half
    split = int(len(X) * TRAIN_RATIO)
    X_train, X_test = X[:split], X[split:]
    y_train, y_test = y[:split], y[split:]
    
    # Scale features
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)
    
    # Train model
    model.fit(X_train_scaled, y_train)
    
    # Predict
    y_pred = model.predict(X_test_scaled)
    
    # Calculate metrics
    buy_signals = (y_pred == 1)
    if buy_signals.sum() > 0:
        buy_correct = (y_test[buy_signals] == 1).sum()
        buy_wr = buy_correct / buy_signals.sum() * 100
    else:
        buy_wr = 0
    
    # P&L simulation (simple)
    test_prices = df['close'].values[valid_idx][split:]
    initial_balance = 10000
    balance = initial_balance
    position = 0
    entry_price = 0
    
    for i in range(len(y_pred)):
        if y_pred[i] == 1 and position == 0:  # Buy signal
            position = balance / test_prices[i]
            entry_price = test_prices[i]
            balance = 0
        elif y_pred[i] == -1 and position > 0:  # Sell signal
            balance = position * test_prices[i]
            pnl = (test_prices[i] - entry_price) / entry_price
            position = 0
    
    # Close any open position
    if position > 0:
        balance = position * test_prices[-1]
    
    pnl_pct = (balance / initial_balance - 1) * 100
    
    # Results
    result = {
        'coin': coin,
        'train_size': len(X_train),
        'test_size': len(X_test),
        'buy_signals': int(buy_signals.sum()),
        'buy_win_rate': round(buy_wr, 1),
        'final_balance': round(balance, 2),
        'pnl_pct': round(pnl_pct, 1),
    }
    
    print(f"{buy_signals.sum()} signals, WR={buy_wr:.1f}%, PnL={pnl_pct:+.1f}%")
    
    return result


def main():
    print("=" * 60)
    print("RUN16: Neural Network Mean Reversion Enhancer")
    print("=" * 60)
    print()
    
    # Initialize model (simple MLP)
    model = MLPClassifier(
        hidden_layer_sizes=(64, 32),
        activation='relu',
        solver='adam',
        max_iter=500,
        early_stopping=True,
        validation_fraction=0.1,
        random_state=42,
        verbose=False
    )
    
    # Run backtest for each coin
    results = []
    for coin in COINS:
        try:
            result = run_backtest(coin, model)
            if result:
                results.append(result)
        except Exception as e:
            print(f"  {coin}: ERROR - {e}")
    
    # Summary
    print()
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    total_signals = sum(r['buy_signals'] for r in results)
    avg_wr = np.mean([r['buy_win_rate'] for r in results])
    total_pnl = sum(r['pnl_pct'] for r in results) / len(results)
    
    print(f"Coins tested: {len(results)}")
    print(f"Total buy signals: {total_signals}")
    print(f"Average win rate: {avg_wr:.1f}%")
    print(f"Average P&L per coin: {total_pnl:+.1f}%")
    
    # Per-coin table
    print()
    print(f"{'COIN':<8} {'SIGNALS':>10} {'WIN RATE':>12} {'P&L %':>10}")
    print("-" * 42)
    for r in sorted(results, key=lambda x: x['pnl_pct'], reverse=True):
        print(f"{r['coin']:<8} {r['buy_signals']:>10} {r['buy_win_rate']:>11.1f}% {r['pnl_pct']:>+9.1f}%")
    
    # Save results
    output = {
        'experiment': 'RUN16',
        'description': 'Neural Network Mean Reversion',
        'model': 'MLPClassifier(64, 32)',
        'features': FEATURES,
        'lookahead': LOOKAHEAD,
        'results': results,
        'summary': {
            'total_signals': total_signals,
            'avg_win_rate': avg_wr,
            'avg_pnl': total_pnl
        }
    }
    
    os.makedirs('/home/scamarena/ProjectCoin/archive/RUN16', exist_ok=True)
    with open('/home/scamarena/ProjectCoin/archive/RUN16/run16_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print()
    print("Results saved to /home/scamarena/ProjectCoin/archive/RUN16/run16_results.json")
    
    return output


if __name__ == '__main__':
    main()
