"""
Enhanced strategies - combining the best performing elements.
"""
import pandas as pd
import numpy as np
from indicators import *

# ============================================================
# ENHANCED STRATEGY 1: Mean Reversion Pro
# Mean reversion with trend filter + volume confirmation
# ============================================================
def strategy_mean_reversion_pro(df: pd.DataFrame) -> tuple:
    """Enhanced Mean Reversion with trend filter and volume"""
    sma_20 = SMA(df['close'], 20)
    sma_50 = SMA(df['close'], 50)
    std_20 = df['close'].rolling(20).std()
    volume_ma = VOLUME_SMA(df['volume'], 20)
    
    # Z-score for mean reversion
    z_score = (df['close'] - sma_20) / std_20
    
    # Trend filter: above 50 SMA for long
    uptrend = df['close'] > sma_50
    
    # Volume confirmation
    volume_confirm = df['volume'] > volume_ma * 1.3
    
    # Entry: significantly oversold + uptrend + volume
    entry = (z_score < -1.5) & uptrend & volume_confirm
    
    # Exit: back to mean or trend reversal
    exit = (z_score > 0) | (df['close'] < sma_50)
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 2: Williams %R Pro
# %R with EMA filter + RSI confirmation
# ============================================================
def strategy_williams_r_pro(df: pd.DataFrame) -> tuple:
    """Williams %R with trend filter"""
    highest = df['high'].rolling(14).max()
    lowest = df['low'].rolling(14).min()
    
    will_r = -100 * ((highest - df['close']) / (highest - lowest))
    
    # EMA filter - only trade in direction of trend
    ema_9 = EMA(df['close'], 9)
    ema_21 = EMA(df['close'], 21)
    uptrend = ema_9 > ema_21
    
    # RSI confirmation
    rsi = RSI(df['close'])
    rsi_recovering = (rsi > 30) & (rsi.shift(1) < 30)
    
    # Entry: oversold + uptrend + rsi recovering
    entry = (will_r < -80) & uptrend & rsi_recovering
    
    # Exit: overbought or trend reversal
    exit = (will_r > -20) | (ema_9 < ema_21)
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 3: RSI Reversal Pro
# RSI with multiple confirmations
# ============================================================
def strategy_rsi_reversal_pro(df: pd.DataFrame) -> tuple:
    """RSI Reversal with EMA trend filter"""
    rsi = RSI(df['close'])
    ema_9 = EMA(df['close'], 9)
    ema_21 = EMA(df['close'], 21)
    ema_50 = EMA(df['close'], 50)
    volume_ma = VOLUME_SMA(df['volume'], 20)
    
    # Trend: above 50 EMA
    uptrend = df['close'] > ema_50
    
    # EMA crossover (9 above 21)
    ema_bullish = ema_9 > ema_21
    
    # RSI reversal from oversold
    rsi_bullish = (rsi < 35) & (rsi.shift(1) >= 35)
    
    # Volume
    volume_ok = df['volume'] > volume_ma
    
    # Entry: oversold + uptrend + bullish ema + volume
    entry = (rsi < 35) & uptrend & ema_bullish & (rsi.shift(1) < rsi) & volume_ok
    
    # Exit: RSI reaches neutral/overbought
    exit = (rsi > 55) | (ema_9 < ema_21)
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 4: BB Bounce Pro
# Bollinger Bands with trend confirmation
# ============================================================
def strategy_bb_bounce_pro(df: pd.DataFrame) -> tuple:
    """Bollinger Band Bounce with trend filter"""
    upper, middle, lower = BOLLINGER_BANDS(df['close'], 20, 2.0)
    ema_21 = EMA(df['close'], 21)
    ema_50 = EMA(df['close'], 50)
    volume_ma = VOLUME_SMA(df['volume'], 20)
    
    # Trend: price above 50 EMA
    uptrend = df['close'] > ema_50
    
    # Price at or below lower band
    at_lower_band = df['close'] <= lower
    
    # Bouncing off lower band (previous bar was below)
    bounce = (df['close'] > lower) & (df['close'].shift(1) <= lower.shift(1))
    
    # Volume spike on bounce
    volume_spike = df['volume'] > volume_ma * 1.5
    
    # Entry: bounce off lower band in uptrend with volume
    entry = bounce & uptrend & volume_spike
    
    # Exit: at middle band or trend reversal
    exit = (df['close'] >= middle) | (df['close'] < ema_50)
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 5: Triple Confluence
# RSI + MACD + Stochastic all agree
# ============================================================
def strategy_triple_confluence(df: pd.DataFrame) -> tuple:
    """Three indicators must agree"""
    rsi = RSI(df['close'])
    macd, signal, hist = MACD(df['close'])
    k, d = STOCHASTIC(df['high'], df['low'], df['close'])
    ema_21 = EMA(df['close'], 21)
    ema_50 = EMA(df['close'], 50)
    
    # All in oversold territory
    rsi_oversold = rsi < 35
    macd_bullish = (macd > signal) & (macd.shift(1) <= signal.shift(1))
    stoch_oversold = (k < 25) & (k > d)
    
    # Trend filter
    uptrend = ema_21 > ema_50
    
    # Entry: all three indicators bullish + uptrend
    entry = rsi_oversold & macd_bullish & stoch_oversold & uptrend
    
    # Exit: any indicator turns bearish
    rsi_exit = rsi > 60
    macd_exit = (macd < signal) & (macd.shift(1) >= signal.shift(1))
    trend_exit = ema_21 < ema_50
    
    exit = rsi_exit | macd_exit | trend_exit
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 6: Volume-Weighted Mean Reversion
# Mean reversion with VWAP confirmation
# ============================================================
def strategy_vwap_reversion(df: pd.DataFrame) -> tuple:
    """VWAP-based mean reversion"""
    vwap = VWAP(df['high'], df['low'], df['close'], df['volume'])
    sma_20 = SMA(df['close'], 20)
    std_20 = df['close'].rolling(20).std()
    
    # Distance from VWAP in std units
    vwap_distance = (df['close'] - vwap) / std_20
    
    # Price significantly below VWAP and 20 SMA
    oversold = (vwap_distance < -1.5) & (df['close'] < sma_20)
    
    # Recovery: price moving back toward VWAP
    recovering = (vwap_distance > vwap_distance.shift(1)) & (vwap_distance.shift(1) < vwap_distance.shift(2))
    
    entry = oversold & recovering
    
    # Exit when back to VWAP or above 20 SMA
    exit = (vwap_distance > 0) | (df['close'] > sma_20)
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 7: Momentum Trap Reversal
# Catch the traps - buy when momentum stalls
# ============================================================
def strategy_momentum_trap(df: pd.DataFrame) -> tuple:
    """Buy when momentum pushes price into support"""
    high_20 = df['high'].rolling(20).max()
    low_20 = df['low'].rolling(20).min()
    
    # At 20-day low (support)
    at_support = df['close'] <= low_20 * 1.02
    
    # Momentum slowing (recent drop smaller than previous)
    momentum_slowing = (df['close'] - df['close'].shift(1)) > (df['close'].shift(1) - df['close'].shift(2))
    
    # RSI not as oversold (divergence - price at low but RSI higher)
    rsi = RSI(df['close'])
    rsi_divergence = rsi > rsi.shift(1)
    
    # Entry: at support + momentum slowing + rsi divergence
    entry = at_support & momentum_slowing & rsi_divergence
    
    # Exit: at 20-day high or after 5 bars
    exit = (df['close'] >= high_20 * 0.95) | (df['close'].shift(5) < df['close'].shift(4))
    
    return entry, exit

# ============================================================
# ENHANCED STRATEGY 8: ADR Reversal
# Trade based on Average Daily Range
# ============================================================
def strategy_adr_reversal(df: pd.DataFrame) -> tuple:
    """Trade reversals based on ADR"""
    high_24 = df['high'].rolling(24).max()
    low_24 = df['low'].rolling(24).min()
    adr = high_24 - low_24
    
    # Price at 75% of ADR range (exhausted move)
    at_adr_extreme = df['close'] <= low_24 + (adr * 0.25)
    
    # Hammer/candlestick reversal signal
    body = abs(df['close'] - df['open'])
    lower_shadow = df[['close', 'open']].min(axis=1) - df['low']
    hammer = (lower_shadow > 2 * body) & (body < (df['high'] - df[['close', 'open']].max(axis=1)))
    
    rsi = RSI(df['close'])
    rsi_oversold = rsi < 40
    
    entry = at_adr_extreme & (hammer | rsi_oversold)
    
    # Exit at middle of range
    exit = df['close'] >= (high_24 + low_24) / 2
    
    return entry, exit

# Registry
ENHANCED_STRATEGIES = {
    'mean_reversion_pro': strategy_mean_reversion_pro,
    'williams_r_pro': strategy_williams_r_pro,
    'rsi_reversal_pro': strategy_rsi_reversal_pro,
    'bb_bounce_pro': strategy_bb_bounce_pro,
    'triple_confluence': strategy_triple_confluence,
    'vwap_reversion': strategy_vwap_reversion,
    'momentum_trap': strategy_momentum_trap,
    'adr_reversal': strategy_adr_reversal,
}
