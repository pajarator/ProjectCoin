"""
Technical indicators for trading strategies.
"""
import pandas as pd
import numpy as np

def SMA(data: pd.Series, period: int) -> pd.Series:
    """Simple Moving Average"""
    return data.rolling(window=period).mean()

def EMA(data: pd.Series, period: int) -> pd.Series:
    """Exponential Moving Average"""
    return data.ewm(span=period, adjust=False).mean()

def RSI(data: pd.Series, period: int = 14) -> pd.Series:
    """Relative Strength Index"""
    delta = data.diff()
    gain = (delta.where(delta > 0, 0)).rolling(window=period).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(window=period).mean()
    
    rs = gain / loss
    rsi = 100 - (100 / (1 + rs))
    return rsi

def MACD(data: pd.Series, fast: int = 12, slow: int = 26, signal: int = 9):
    """
    MACD - Moving Average Convergence Divergence
    Returns: macd, signal_line, histogram
    """
    ema_fast = EMA(data, fast)
    ema_slow = EMA(data, slow)
    
    macd = ema_fast - ema_slow
    signal_line = EMA(macd, signal)
    histogram = macd - signal_line
    
    return macd, signal_line, histogram

def BOLLINGER_BANDS(data: pd.Series, period: int = 20, std_dev: float = 2.0):
    """
    Bollinger Bands
    Returns: upper, middle, lower
    """
    middle = SMA(data, period)
    std = data.rolling(window=period).std()
    
    upper = middle + (std_dev * std)
    lower = middle - (std_dev * std)
    
    return upper, middle, lower

def ATR(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 14) -> pd.Series:
    """Average True Range - volatility indicator"""
    tr1 = high - low
    tr2 = abs(high - close.shift(1))
    tr3 = abs(low - close.shift(1))
    
    tr = pd.concat([tr1, tr2, tr3], axis=1).max(axis=1)
    atr = tr.rolling(window=period).mean()
    
    return atr

def STOCHASTIC(high: pd.Series, low: pd.Series, close: pd.Series, k_period: int = 14, d_period: int = 3):
    """
    Stochastic Oscillator
    Returns: %K, %D
    """
    lowest_low = low.rolling(window=k_period).min()
    highest_high = high.rolling(window=k_period).max()
    
    k = 100 * ((close - lowest_low) / (highest_high - lowest_low))
    d = k.rolling(window=d_period).mean()
    
    return k, d

def VWAP(high: pd.Series, low: pd.Series, close: pd.Series, volume: pd.Series) -> pd.Series:
    """Volume Weighted Average Price"""
    typical_price = (high + low + close) / 3
    vwap = (typical_price * volume).cumsum() / volume.cumsum()
    return vwap

def VOLUME_SMA(volume: pd.Series, period: int = 20) -> pd.Series:
    """Volume Simple Moving Average"""
    return SMA(volume, period)

def MOMENTUM(data: pd.Series, period: int = 10) -> pd.Series:
    """Momentum - rate of change"""
    return data.diff(period)

def ROC(data: pd.Series, period: int = 12) -> pd.Series:
    """Rate of Change"""
    return ((data - data.shift(period)) / data.shift(period)) * 100

def ADX(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 14) -> pd.Series:
    """
    Average Directional Index - trend strength
    """
    plus_dm = high.diff()
    minus_dm = -low.diff()
    
    plus_dm[plus_dm < 0] = 0
    minus_dm[minus_dm < 0] = 0
    
    tr = ATR(high, low, close, period)
    
    plus_di = 100 * (plus_dm.rolling(window=period).mean() / tr)
    minus_di = 100 * (minus_dm.rolling(window=period).mean() / tr)
    
    dx = 100 * abs(plus_di - minus_di) / (plus_di + minus_di)
    adx = dx.rolling(window=period).mean()
    
    return adx

def add_all_indicators(df: pd.DataFrame) -> pd.DataFrame:
    """
    Add all indicators to a dataframe for analysis.
    """
    result = df.copy()
    
    # Moving Averages
    for period in [9, 21, 50, 200]:
        result[f'SMA_{period}'] = SMA(result['close'], period)
        result[f'EMA_{period}'] = EMA(result['close'], period)
    
    # RSI
    result['RSI'] = RSI(result['close'])
    
    # MACD
    result['MACD'], result['MACD_signal'], result['MACD_hist'] = MACD(result['close'])
    
    # Bollinger Bands
    result['BB_upper'], result['BB_middle'], result['BB_lower'] = BOLLINGER_BANDS(result['close'])
    result['BB_width'] = (result['BB_upper'] - result['BB_lower']) / result['BB_middle']
    result['BB_position'] = (result['close'] - result['BB_lower']) / (result['BB_upper'] - result['BB_lower'])
    
    # Stochastic
    result['STOCH_K'], result['STOCH_D'] = STOCHASTIC(result['high'], result['low'], result['close'])
    
    # Volume
    result['Volume_SMA'] = VOLUME_SMA(result['volume'])
    result['Volume_ratio'] = result['volume'] / result['Volume_SMA']
    
    # ATR
    result['ATR'] = ATR(result['high'], result['low'], result['close'])
    
    # ADX
    result['ADX'] = ADX(result['high'], result['low'], result['close'])
    
    # Momentum & ROC
    result['MOMENTUM'] = MOMENTUM(result['close'])
    result['ROC'] = ROC(result['close'])
    
    # VWAP (if we have volume)
    if 'volume' in result.columns:
        result['VWAP'] = VWAP(result['high'], result['low'], result['close'], result['volume'])
    
    # Price position
    result['High_20'] = result['high'].rolling(20).max()
    result['Low_20'] = result['low'].rolling(20).min()
    result['Price_position'] = (result['close'] - result['Low_20']) / (result['High_20'] - result['Low_20'])
    
    return result
