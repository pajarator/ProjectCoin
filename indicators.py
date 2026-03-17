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
    # When both gain and loss are 0 (flat price), return 50 (neutral)
    rs = gain / loss.replace(0, np.nan)
    rsi = 100 - (100 / (1 + rs))
    return rsi.fillna(50)

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

def WMA(data: pd.Series, period: int) -> pd.Series:
    """Weighted Moving Average"""
    weights = np.arange(1, period + 1, dtype=float)
    return data.rolling(window=period).apply(lambda x: np.dot(x, weights) / weights.sum(), raw=True)

def OBV(close: pd.Series, volume: pd.Series) -> pd.Series:
    """On-Balance Volume"""
    direction = np.sign(close.diff())
    direction.iloc[0] = 0
    return (direction * volume).cumsum()

def CMF(high: pd.Series, low: pd.Series, close: pd.Series, volume: pd.Series, period: int = 20) -> pd.Series:
    """Chaikin Money Flow"""
    hl_range = high - low
    # Doji bars (h==l): treat as neutral MFM=0 to avoid NaN contamination in rolling sum
    mfm = (((close - low) - (high - close)) / hl_range.replace(0, np.nan)).fillna(0)
    mfv = mfm * volume
    return mfv.rolling(window=period).sum() / volume.rolling(window=period).sum()

def WILLIAMS_R(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 14) -> pd.Series:
    """Williams %R"""
    highest_high = high.rolling(window=period).max()
    lowest_low = low.rolling(window=period).min()
    return -100 * (highest_high - close) / (highest_high - lowest_low)

def CCI(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 20) -> pd.Series:
    """Commodity Channel Index"""
    tp = (high + low + close) / 3
    sma_tp = tp.rolling(window=period).mean()
    mad = tp.rolling(window=period).apply(lambda x: np.abs(x - x.mean()).mean(), raw=True)
    return (tp - sma_tp) / (0.015 * mad)

def KELTNER(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 20, atr_mult: float = 1.5):
    """Keltner Channels. Returns: upper, middle, lower"""
    middle = EMA(close, period)
    atr = ATR(high, low, close, period)
    upper = middle + atr_mult * atr
    lower = middle - atr_mult * atr
    return upper, middle, lower

def DONCHIAN(high: pd.Series, low: pd.Series, period: int = 20):
    """Donchian Channel. Returns: upper, lower, middle"""
    upper = high.rolling(window=period).max()
    lower = low.rolling(window=period).min()
    middle = (upper + lower) / 2
    return upper, lower, middle

def HULL_MA(data: pd.Series, period: int = 9) -> pd.Series:
    """Hull Moving Average"""
    half_period = int(period / 2)
    sqrt_period = int(np.sqrt(period))
    wma_half = WMA(data, half_period)
    wma_full = WMA(data, period)
    diff = 2 * wma_half - wma_full
    return WMA(diff, sqrt_period)

def KAMA(data: pd.Series, period: int = 10, fast: int = 2, slow: int = 30) -> pd.Series:
    """Kaufman Adaptive Moving Average"""
    fast_sc = 2.0 / (fast + 1.0)
    slow_sc = 2.0 / (slow + 1.0)

    direction = abs(data - data.shift(period))
    volatility = data.diff().abs().rolling(window=period).sum()
    er = direction / volatility.replace(0, np.nan)
    sc = (er * (fast_sc - slow_sc) + slow_sc) ** 2

    kama = pd.Series(np.nan, index=data.index)
    # Initialize with first valid value after warmup
    first_valid = period
    if first_valid < len(data):
        kama.iloc[first_valid] = data.iloc[first_valid]
        for i in range(first_valid + 1, len(data)):
            if np.isnan(kama.iloc[i - 1]) or np.isnan(sc.iloc[i]):
                kama.iloc[i] = kama.iloc[i - 1]
            else:
                kama.iloc[i] = kama.iloc[i - 1] + sc.iloc[i] * (data.iloc[i] - kama.iloc[i - 1])
    return kama

def TRIX(data: pd.Series, period: int = 15) -> pd.Series:
    """Triple Smoothed EMA Rate of Change"""
    ema1 = EMA(data, period)
    ema2 = EMA(ema1, period)
    ema3 = EMA(ema2, period)
    return ema3.pct_change() * 100

def AROON(high: pd.Series, low: pd.Series, period: int = 25):
    """Aroon Indicator. Returns: aroon_up, aroon_down"""
    aroon_up = high.rolling(window=period + 1).apply(lambda x: x.argmax() / period * 100, raw=True)
    aroon_down = low.rolling(window=period + 1).apply(lambda x: x.argmin() / period * 100, raw=True)
    return aroon_up, aroon_down

def VORTEX(high: pd.Series, low: pd.Series, close: pd.Series, period: int = 14):
    """Vortex Indicator. Returns: vi_plus, vi_minus"""
    vm_plus = abs(high - low.shift(1))
    vm_minus = abs(low - high.shift(1))
    tr1 = high - low
    tr2 = abs(high - close.shift(1))
    tr3 = abs(low - close.shift(1))
    tr = pd.concat([tr1, tr2, tr3], axis=1).max(axis=1)
    vi_plus = vm_plus.rolling(window=period).sum() / tr.rolling(window=period).sum()
    vi_minus = vm_minus.rolling(window=period).sum() / tr.rolling(window=period).sum()
    return vi_plus, vi_minus

def AWESOME_OSCILLATOR(high: pd.Series, low: pd.Series) -> pd.Series:
    """Awesome Oscillator"""
    median_price = (high + low) / 2
    return SMA(median_price, 5) - SMA(median_price, 34)

def LAGUERRE_RSI(close: pd.Series, gamma: float = 0.8) -> pd.Series:
    """Laguerre RSI - ported from coinclaw Rust implementation"""
    n = len(close)
    values = close.values
    result = np.full(n, np.nan)
    l0 = l1 = l2 = l3 = 0.0

    for i in range(n):
        prev_l0 = l0
        prev_l1 = l1
        prev_l2 = l2

        l0 = (1.0 - gamma) * values[i] + gamma * l0
        l1 = -gamma * l0 + prev_l0 + gamma * l1
        l2 = -gamma * l1 + prev_l1 + gamma * l2
        l3 = -gamma * l2 + prev_l2 + gamma * l3

        if i >= 3:
            cu = cd = 0.0
            d0 = l0 - l1
            d1 = l1 - l2
            d2 = l2 - l3
            if d0 > 0: cu += d0
            else: cd -= d0
            if d1 > 0: cu += d1
            else: cd -= d1
            if d2 > 0: cu += d2
            else: cd -= d2
            result[i] = (cu / (cu + cd) * 100.0) if (cu + cd) > 0 else 50.0

    return pd.Series(result, index=close.index)

def KALMAN_FILTER(close: pd.Series, process_var: float = 1e-5, measure_var: float = 0.01):
    """Kalman Filter - ported from coinclaw Rust implementation. Returns: estimate, variance"""
    n = len(close)
    values = close.values
    est = np.full(n, np.nan)
    var = np.full(n, np.nan)

    x_est = values[0]
    p_est = 1.0
    est[0] = x_est
    var[0] = p_est

    for i in range(1, n):
        x_pred = x_est
        p_pred = p_est + process_var
        k = p_pred / (p_pred + measure_var)
        x_est = x_pred + k * (values[i] - x_pred)
        p_est = (1.0 - k) * p_pred
        est[i] = x_est
        var[i] = p_est

    return pd.Series(est, index=close.index), pd.Series(var, index=close.index)

def KST(close: pd.Series):
    """Know Sure Thing oscillator - ported from coinclaw Rust. Returns: kst, signal"""
    roc10 = ROC(close, 10)
    roc15 = ROC(close, 15)
    roc20 = ROC(close, 20)
    roc30 = ROC(close, 30)

    kst = (SMA(roc10, 10) * 1 + SMA(roc15, 10) * 2 +
           SMA(roc20, 10) * 3 + SMA(roc30, 15) * 4)
    signal = SMA(kst, 9)
    return kst, signal

def HEIKIN_ASHI(open: pd.Series, high: pd.Series, low: pd.Series, close: pd.Series):
    """Heikin-Ashi candles. Returns: ha_open, ha_high, ha_low, ha_close"""
    ha_close = (open + high + low + close) / 4
    ha_open = pd.Series(np.nan, index=open.index)
    ha_open.iloc[0] = (open.iloc[0] + close.iloc[0]) / 2
    for i in range(1, len(open)):
        ha_open.iloc[i] = (ha_open.iloc[i - 1] + ha_close.iloc[i - 1]) / 2
    ha_high = pd.concat([high, ha_open, ha_close], axis=1).max(axis=1)
    ha_low = pd.concat([low, ha_open, ha_close], axis=1).min(axis=1)
    return ha_open, ha_high, ha_low, ha_close

def ICHIMOKU(high: pd.Series, low: pd.Series, close: pd.Series,
             tenkan: int = 9, kijun: int = 26, senkou_b: int = 52):
    """Ichimoku Cloud. Returns: tenkan_sen, kijun_sen, senkou_a, senkou_b, chikou"""
    tenkan_sen = (high.rolling(window=tenkan).max() + low.rolling(window=tenkan).min()) / 2
    kijun_sen = (high.rolling(window=kijun).max() + low.rolling(window=kijun).min()) / 2
    senkou_a = ((tenkan_sen + kijun_sen) / 2).shift(kijun)
    senkou_b_line = ((high.rolling(window=senkou_b).max() + low.rolling(window=senkou_b).min()) / 2).shift(kijun)
    chikou = close.shift(-kijun)
    return tenkan_sen, kijun_sen, senkou_a, senkou_b_line, chikou


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


def add_all_indicators_extended(df: pd.DataFrame) -> pd.DataFrame:
    """Add all indicators (original + new) for ML feature matrices."""
    result = add_all_indicators(df)

    h, l, c, v = result['high'], result['low'], result['close'], result['volume']

    # New moving averages
    result['WMA_9'] = WMA(c, 9)
    result['HULL_MA'] = HULL_MA(c, 9)
    result['KAMA'] = KAMA(c, 10)

    # Volume indicators
    result['OBV'] = OBV(c, v)
    result['CMF'] = CMF(h, l, c, v)

    # Momentum
    result['WILLIAMS_R'] = WILLIAMS_R(h, l, c)
    result['CCI'] = CCI(h, l, c)
    result['TRIX'] = TRIX(c)
    result['AO'] = AWESOME_OSCILLATOR(h, l)

    # Channels
    result['KELTNER_upper'], result['KELTNER_middle'], result['KELTNER_lower'] = KELTNER(h, l, c)
    result['DONCHIAN_upper'], result['DONCHIAN_lower'], result['DONCHIAN_middle'] = DONCHIAN(h, l)

    # Trend
    result['AROON_up'], result['AROON_down'] = AROON(h, l)
    result['VORTEX_plus'], result['VORTEX_minus'] = VORTEX(h, l, c)

    # RUN13 complement indicators
    result['LAGUERRE_RSI'] = LAGUERRE_RSI(c)
    result['KALMAN_est'], result['KALMAN_var'] = KALMAN_FILTER(c)
    result['KST'], result['KST_signal'] = KST(c)

    # Heikin-Ashi
    result['HA_open'], result['HA_high'], result['HA_low'], result['HA_close'] = HEIKIN_ASHI(
        result['open'], h, l, c)

    # Ichimoku
    (result['ICHI_tenkan'], result['ICHI_kijun'], result['ICHI_senkou_a'],
     result['ICHI_senkou_b'], result['ICHI_chikou']) = ICHIMOKU(h, l, c)

    # Derived: squeeze (BB inside Keltner)
    result['SQUEEZE'] = (result['BB_upper'] < result['KELTNER_upper']) & (result['BB_lower'] > result['KELTNER_lower'])

    return result
