"""
Feature engineering pipeline for ML-based strategy discovery.
Builds ~65-80 features from OHLCV data for use in ML models.
"""
import pandas as pd
import numpy as np
from indicators import (
    SMA, EMA, WMA, RSI, MACD, BOLLINGER_BANDS, ATR, STOCHASTIC, VWAP,
    MOMENTUM, ROC, ADX, OBV, CMF, WILLIAMS_R, CCI, KELTNER, DONCHIAN,
    HULL_MA, TRIX, AROON, VORTEX, AWESOME_OSCILLATOR, LAGUERRE_RSI,
    KALMAN_FILTER, KST
)

# 19 coins available in data_cache
COINS = [
    'ADA', 'ALGO', 'ATOM', 'AVAX', 'BNB', 'BTC', 'DASH', 'DOGE', 'DOT',
    'ETH', 'LINK', 'LTC', 'NEAR', 'SHIB', 'SOL', 'TRX', 'UNI', 'XLM', 'XRP'
]


def load_cached_data(coin: str, timeframe: str = '15m', duration: str = '1year') -> pd.DataFrame:
    """Load OHLCV data from data_cache."""
    path = f'data_cache/{coin}_USDT_{timeframe}_{duration}.csv'
    df = pd.read_csv(path)
    df.columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume']
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    df.set_index('timestamp', inplace=True)
    return df


def build_feature_matrix(df: pd.DataFrame, include_targets: bool = True) -> pd.DataFrame:
    """
    Build ~65-80 feature matrix from OHLCV DataFrame.

    Args:
        df: DataFrame with columns open, high, low, close, volume and datetime index
        include_targets: if True, add target columns (direction, pct change)

    Returns:
        DataFrame with all features. First ~200 rows may have NaN from warmup.
    """
    feat = pd.DataFrame(index=df.index)
    o, h, l, c, v = df['open'], df['high'], df['low'], df['close'], df['volume']

    # === PRICE FEATURES (15) ===
    feat['returns_1'] = c.pct_change(1)
    feat['returns_5'] = c.pct_change(5)
    feat['returns_15'] = c.pct_change(15)
    feat['log_returns'] = np.log(c / c.shift(1))
    feat['high_low_range'] = (h - l) / c
    sma20 = SMA(c, 20)
    sma50 = SMA(c, 50)
    feat['close_to_sma20'] = (c - sma20) / sma20
    feat['close_to_sma50'] = (c - sma50) / sma50
    high_20 = h.rolling(20).max()
    low_20 = l.rolling(20).min()
    feat['price_position'] = (c - low_20) / (high_20 - low_20)
    body = abs(c - o)
    candle_range = h - l
    safe_range = candle_range.replace(0, np.nan)
    feat['upper_shadow'] = ((h - pd.concat([c, o], axis=1).max(axis=1)) / safe_range).fillna(0)
    feat['lower_shadow'] = ((pd.concat([c, o], axis=1).min(axis=1) - l) / safe_range).fillna(0)
    feat['body_ratio'] = (body / safe_range).fillna(1)  # doji: treat as all-body
    feat['gap'] = (o - c.shift(1)) / c.shift(1)
    atr14 = ATR(h, l, c, 14)
    feat['atr_pct'] = atr14 / c
    feat['candle_direction'] = np.sign(c - o)

    # === MOVING AVERAGE FEATURES (8) ===
    ema9 = EMA(c, 9)
    ema21 = EMA(c, 21)
    feat['ema9_21_cross'] = (ema9 - ema21) / c
    feat['sma20_slope'] = sma20.pct_change(5)
    feat['sma50_slope'] = sma50.pct_change(5)
    feat['ma_convergence'] = (ema9 - sma50) / c
    hull = HULL_MA(c, 9)
    feat['hull_vs_close'] = (hull - c) / c
    feat['ema9_vs_close'] = (ema9 - c) / c
    feat['ema21_vs_close'] = (ema21 - c) / c
    feat['sma20_vs_sma50'] = (sma20 - sma50) / c

    # === MOMENTUM FEATURES (12) ===
    feat['rsi_14'] = RSI(c, 14)
    feat['rsi_7'] = RSI(c, 7)
    feat['rsi_slope'] = feat['rsi_14'].diff(3)
    stoch_k, stoch_d = STOCHASTIC(h, l, c)
    feat['stoch_k'] = stoch_k
    feat['stoch_d'] = stoch_d
    _, _, macd_hist = MACD(c)
    feat['macd_hist_norm'] = macd_hist / c
    feat['cci_20'] = CCI(h, l, c)
    feat['williams_r'] = WILLIAMS_R(h, l, c)
    feat['momentum_10'] = MOMENTUM(c, 10) / c.shift(10)
    feat['roc_12'] = ROC(c, 12)
    feat['awesome_osc'] = AWESOME_OSCILLATOR(h, l) / c
    feat['trix_15'] = TRIX(c)

    # === VOLATILITY FEATURES (8) ===
    bb_upper, bb_middle, bb_lower = BOLLINGER_BANDS(c)
    feat['bb_width'] = (bb_upper - bb_lower) / bb_middle
    feat['bb_position'] = (c - bb_lower) / (bb_upper - bb_lower)
    feat['atr_14'] = atr14
    atr50 = ATR(h, l, c, 50)
    feat['atr_ratio'] = atr14 / atr50
    kelt_upper, kelt_middle, kelt_lower = KELTNER(h, l, c)
    feat['keltner_position'] = (c - kelt_lower) / (kelt_upper - kelt_lower)
    feat['volatility_20'] = c.pct_change().rolling(20).std()
    vol_5 = c.pct_change().rolling(5).std()
    vol_20 = feat['volatility_20']
    feat['volatility_ratio'] = vol_5 / vol_20
    feat['squeeze'] = ((bb_upper < kelt_upper) & (bb_lower > kelt_lower)).astype(int)

    # === VOLUME FEATURES (7) ===
    vol_sma = v.rolling(20).mean()
    feat['volume_ratio'] = v / vol_sma
    obv = OBV(c, v)
    feat['obv_slope'] = obv.diff(5) / obv.abs().rolling(20).mean().replace(0, np.nan)
    feat['cmf_20'] = CMF(h, l, c, v)
    feat['volume_trend'] = v.rolling(5).mean() / vol_sma
    vwap = VWAP(h, l, c, v)
    feat['vwap_distance'] = (c - vwap) / c
    feat['volume_momentum'] = v.diff(5) / vol_sma
    feat['high_volume_bars_5'] = (v > 2 * vol_sma).rolling(5).sum()

    # === TREND FEATURES (5) ===
    feat['adx_14'] = ADX(h, l, c)
    aroon_up, aroon_down = AROON(h, l)
    feat['aroon_up'] = aroon_up
    feat['aroon_down'] = aroon_down
    vi_plus, vi_minus = VORTEX(h, l, c)
    feat['vortex_diff'] = vi_plus - vi_minus
    feat['trend_strength'] = abs(feat['close_to_sma50']) * feat['adx_14'] / 100

    # === TIME FEATURES (6) ===
    feat['hour_of_day'] = df.index.hour
    feat['day_of_week'] = df.index.dayofweek
    feat['is_london'] = ((df.index.hour >= 8) & (df.index.hour <= 16)).astype(int)
    feat['is_ny'] = ((df.index.hour >= 13) & (df.index.hour <= 21)).astype(int)
    feat['is_asia'] = ((df.index.hour >= 0) & (df.index.hour <= 8)).astype(int)
    # Cyclical encoding
    feat['hour_sin'] = np.sin(2 * np.pi * df.index.hour / 24)
    feat['hour_cos'] = np.cos(2 * np.pi * df.index.hour / 24)

    # === ADVANCED INDICATORS (5) ===
    feat['laguerre_rsi'] = LAGUERRE_RSI(c)
    kalman_est, kalman_var = KALMAN_FILTER(c)
    feat['kalman_dist'] = (c - kalman_est) / c
    kst_val, kst_sig = KST(c)
    feat['kst'] = kst_val
    feat['kst_signal'] = kst_sig
    feat['kst_hist'] = kst_val - kst_sig

    # === TARGETS ===
    if include_targets:
        feat['target_1bar'] = np.sign(c.shift(-1) - c)  # direction next bar
        feat['target_5bar'] = np.sign(c.shift(-5) - c)  # direction 5 bars ahead
        feat['target_pct_1bar'] = c.pct_change().shift(-1)  # pct return next bar

    return feat


def get_feature_columns(include_time: bool = True) -> list:
    """Return list of feature column names (excluding targets)."""
    cols = [
        # Price
        'returns_1', 'returns_5', 'returns_15', 'log_returns', 'high_low_range',
        'close_to_sma20', 'close_to_sma50', 'price_position',
        'upper_shadow', 'lower_shadow', 'body_ratio', 'gap', 'atr_pct', 'candle_direction',
        # Moving average
        'ema9_21_cross', 'sma20_slope', 'sma50_slope', 'ma_convergence',
        'hull_vs_close', 'ema9_vs_close', 'ema21_vs_close', 'sma20_vs_sma50',
        # Momentum
        'rsi_14', 'rsi_7', 'rsi_slope', 'stoch_k', 'stoch_d',
        'macd_hist_norm', 'cci_20', 'williams_r', 'momentum_10', 'roc_12',
        'awesome_osc', 'trix_15',
        # Volatility
        'bb_width', 'bb_position', 'atr_14', 'atr_ratio', 'keltner_position',
        'volatility_20', 'volatility_ratio', 'squeeze',
        # Volume
        'volume_ratio', 'obv_slope', 'cmf_20', 'volume_trend',
        'vwap_distance', 'volume_momentum', 'high_volume_bars_5',
        # Trend
        'adx_14', 'aroon_up', 'aroon_down', 'vortex_diff', 'trend_strength',
        # Advanced
        'laguerre_rsi', 'kalman_dist', 'kst', 'kst_signal', 'kst_hist',
    ]
    if include_time:
        cols += ['hour_of_day', 'day_of_week', 'is_london', 'is_ny', 'is_asia',
                 'hour_sin', 'hour_cos']
    return cols


if __name__ == '__main__':
    # Quick test
    df = load_cached_data('BTC')
    features = build_feature_matrix(df)
    print(f'Shape: {features.shape}')
    print(f'Columns: {len(features.columns)}')
    print(f'Feature cols: {len(get_feature_columns())}')
    # Check NaN after warmup
    warmup = 200
    clean = features.iloc[warmup:-5]  # skip last 5 for targets
    nan_cols = clean.columns[clean.isna().any()]
    if len(nan_cols) > 0:
        print(f'Columns with NaN after warmup: {nan_cols.tolist()}')
    else:
        print('No NaN after warmup row 200 - OK')
