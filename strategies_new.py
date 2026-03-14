"""
New strategies from Reddit/trading communities
"""
import pandas as pd
import numpy as np
from indicators import *

# ============================================================
# STRATEGY: RSI DIVERGENCE
# Buy when price makes lower low but RSI makes higher low
# ============================================================
def strategy_rsi_divergence(df: pd.DataFrame) -> tuple:
    """RSI Bullish Divergence - price lower low, RSI higher low"""
    rsi = RSI(df['close'])
    
    # Find local lows in price
    price_low = df['low'].rolling(5).min()
    rsi_low = rsi.rolling(5).min()
    
    # Price making lower low
    price_ll = df['low'] < df['low'].shift(5)
    # RSI making higher low
    rsi_hl = rsi > rsi.shift(5)
    
    # Both conditions + RSI oversold
    entry = price_ll & rsi_hl & (rsi < 40)
    
    # Exit when RSI reaches overbought or price crosses SMA
    sma_20 = SMA(df['close'], 20)
    exit = (rsi > 60) | (df['close'] > sma_20)
    
    return entry, exit

# ============================================================
# STRATEGY: SUPERTREND
# ============================================================
def strategy_supertrend(df: pd.DataFrame, period: int = 10, multiplier: float = 3.0) -> tuple:
    """Supertrend indicator strategy"""
    hl = (df['high'] + df['low']) / 2
    tr = ATR(df['high'], df['low'], df['close'], 14)
    
    upper = hl + multiplier * tr
    lower = hl - multiplier * tr
    
    # Calculate supertrend
    in_trend = pd.Series(True, index=df.index)
    direction = pd.Series(1, index=df.index)
    
    for i in range(1, len(df)):
        if df['close'].iloc[i] > upper.iloc[i-1]:
            in_trend.iloc[i] = True
            direction.iloc[i] = 1
        elif df['close'].iloc[i] < lower.iloc[i-1]:
            in_trend.iloc[i] = False
            direction.iloc[i] = -1
        else:
            in_trend.iloc[i] = in_trend.iloc[i-1]
            direction.iloc[i] = direction.iloc[i-1]
        
        if in_trend.iloc[i]:
            lower.iloc[i] = max(lower.iloc[i], lower.iloc[i-1])
        else:
            upper.iloc[i] = min(upper.iloc[i], upper.iloc[i-1])
    
    # Entry: trend changes to up
    entry = (direction == 1) & (direction.shift(1) == -1)
    
    # Exit: trend changes to down
    exit = (direction == -1) & (direction.shift(1) == 1)
    
    return entry, exit

# ============================================================
# STRATEGY: ORDER BLOCK (ICT Concept)
# Buy at previous support that held
# ============================================================
def strategy_order_block(df: pd.DataFrame) -> tuple:
    """Order Block - buy at previous support"""
    # Find bullish candle followed by at least 2 down candles
    # Then buy at low of first bullish candle
    
    # Bullish candle
    bullish = df['close'] > df['open']
    
    # After it, 2+ bearish candles
    bearish = df['close'] < df['open']
    
    # Entry: price returns to what was support (order block)
    entry = bullish.shift(3) & bearish.shift(2) & bearish.shift(1) & (df['close'] < df['close'].shift(3))
    
    # Exit at next high or after 3 candles
    exit = (df['close'] > df['high'].shift(3)) | (df['close'].shift(3) < df['close'].shift(2))
    
    return entry, exit

# ============================================================
# STRATEGY: FVG (FAIR VALUE GAP)
# Buy when price fills the gap
# ============================================================
def strategy_fvg(df: pd.DataFrame) -> tuple:
    """Fair Value Gap - buy on retracement to fill gap"""
    # FVG: middle candle has gap above and below
    # Gap up: high[i-1] < low[i+1]
    # Gap down: low[i-1] > high[i+1]
    
    fvg_bullish = (df['high'].shift(2) < df['low'].shift(-1)) & (df['close'].shift(1) > df['open'].shift(1))
    fvg_bearish = (df['low'].shift(2) > df['high'].shift(-1)) & (df['close'].shift(1) < df['open'].shift(1))
    
    # Entry: price retraces into FVG
    entry = fvg_bullish & (df['close'] < df['close'].shift(1)) & (df['close'] > df['low'].shift(2))
    
    # Exit when gap is filled
    exit = df['close'] < df['low'].shift(2)
    
    return entry, exit

# ============================================================
# STRATEGY: MARKET STRUCTURE SHIFT (ICT MSS)
# ============================================================
def strategy_mss(df: pd.DataFrame) -> tuple:
    """Market Structure Shift - break of swing high"""
    # Find swing high
    swing_high = df['high'].rolling(5).max()
    
    # MSS: price breaks above recent swing high
    mss = df['close'] > swing_high.shift(1)
    
    entry = mss & (df['close'] > df['close'].shift(2))
    
    # Exit on break below recent low
    swing_low = df['low'].rolling(5).min()
    exit = df['close'] < swing_low
    
    return entry, exit

# ============================================================
# STRATEGY: MOVING AVERAGE RIBBON
# ============================================================
def strategy_ma_ribbon(df: pd.DataFrame) -> tuple:
    """Moving Average Ribbon - trade in direction of fan"""
    ema_9 = EMA(df['close'], 9)
    ema_21 = EMA(df['close'], 21)
    ema_50 = EMA(df['close'], 50)
    
    # All EMAs aligned (9 > 21 > 50)
    bullish_ribbon = (ema_9 > ema_21) & (ema_21 > ema_50)
    bearish_ribbon = (ema_9 < ema_21) & (ema_21 < ema_50)
    
    # Entry: ribbon turns bullish
    entry = bullish_ribbon & ~bullish_ribbon.shift(1)
    
    # Exit: ribbon turns bearish
    exit = bearish_ribbon & ~bearish_ribbon.shift(1)
    
    return entry, exit

# ============================================================
# STRATEGY: TREND LINE BREAKOUT
# ============================================================
def strategy_trendline(df: pd.DataFrame) -> tuple:
    """Simple trendline breakout using linear regression"""
    from scipy import stats
    
    # Calculate trendline slope
    y = df['close'].values
    x = np.arange(len(y))
    slope, _ = stats.linregress(x[-20:], y[-20:])
    
    # Entry: price breaks above upward trendline
    entry = (slope > 0) & (df['close'] > df['close'].shift(1)) & (df['volume'] > df['volume'].rolling(10).mean())
    
    # Exit: slope turns negative or price drops
    exit = (slope < 0) | (df['close'] < df['close'].shift(3))
    
    return entry, exit

# ============================================================
# STRATEGY: DUAL RSI
# Two RSI periods for confirmation
# ============================================================
def strategy_dual_rsi(df: pd.DataFrame) -> tuple:
    """Dual RSI - RSI 7 and RSI 14 must agree"""
    rsi_fast = RSI(df['close'], 7)
    rsi_slow = RSI(df['close'], 14)
    
    # Both oversold
    entry = (rsi_fast < 30) & (rsi_slow < 35)
    
    # Either overbought
    exit = (rsi_fast > 70) | (rsi_slow > 65)
    
    return entry, exit

# ============================================================
# STRATEGY: VOLUME PRICE TREND
# ============================================================
def strategy_vpt(df: pd.DataFrame) -> tuple:
    """Volume Price Trend"""
    vpt = ((df['close'] - df['close'].shift(1)) / df['close'].shift(1)) * df['volume']
    vpt = vpt.cumsum()
    vpt_ma = SMA(vpt, 10)
    
    # VPT crosses above its MA
    entry = (vpt > vpt_ma) & (vpt.shift(1) <= vpt_ma.shift(1))
    
    # VPT crosses below its MA
    exit = (vpt < vpt_ma) & (vpt.shift(1) >= vpt_ma.shift(1))
    
    return entry, exit

# ============================================================
# STRATEGY: PIVOT POINT BOUNCE
# ============================================================
def strategy_pivot(df: pd.DataFrame) -> tuple:
    """Pivot Point Bounce"""
    # Classic pivot points
    pp = (df['high'].shift(1) + df['low'].shift(1) + df['close'].shift(1)) / 3
    r1 = 2 * pp - df['low'].shift(1)
    s1 = 2 * pp - df['high'].shift(1)
    
    # Entry: price bounces off support
    entry = (df['close'] < s1) & (df['close'].shift(1) < s1) & (df['close'] > s1.shift(1))
    
    # Exit: at pivot or resistance
    exit = (df['close'] > pp) | (df['close'] > r1)
    
    return entry, exit

NEW_STRATEGIES = {
    'rsi_divergence': strategy_rsi_divergence,
    'supertrend': strategy_supertrend,
    'order_block': strategy_order_block,
    'fvg': strategy_fvg,
    'mss': strategy_mss,
    'ma_ribbon': strategy_ma_ribbon,
    'trendline': strategy_trendline,
    'dual_rsi': strategy_dual_rsi,
    'vpt': strategy_vpt,
    'pivot': strategy_pivot,
}
