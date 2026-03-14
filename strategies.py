"""
Trading strategies to backtest.
Each strategy returns entry and exit signals as boolean Series.
"""
import pandas as pd
import numpy as np
from indicators import *

# ============================================================
# STRATEGY 1: RSI Reversal
# Buy when RSI < oversold, sell when RSI > overbought
# ============================================================
def strategy_rsi_reversal(df: pd.DataFrame, 
                          rsi_oversold: int = 30,
                          rsi_overbought: int = 70,
                          rsi_period: int = 14) -> tuple:
    """RSI Reversal Strategy"""
    rsi = RSI(df['close'], rsi_period)
    
    entry = (rsi < rsi_oversold) & (rsi.shift(1) >= rsi_oversold)
    exit = (rsi > rsi_overbought) & (rsi.shift(1) <= rsi_overbought)
    
    return entry, exit

# ============================================================
# STRATEGY 2: MACD Crossover
# Buy when MACD crosses above signal, sell when crosses below
# ============================================================
def strategy_macd_crossover(df: pd.DataFrame) -> tuple:
    """MACD Crossover Strategy"""
    macd, signal, hist = MACD(df['close'])
    
    # Bullish crossover: MACD crosses above signal
    entry = (macd > signal) & (macd.shift(1) <= signal.shift(1))
    
    # Bearish crossover: MACD crosses below signal
    exit = (macd < signal) & (macd.shift(1) >= signal.shift(1))
    
    return entry, exit

# ============================================================
# STRATEGY 3: Bollinger Band Bounce
# Buy when price touches lower band, sell at middle/upper
# ============================================================
def strategy_bb_bounce(df: pd.DataFrame, 
                       bb_period: int = 20, 
                       bb_std: float = 2.0) -> tuple:
    """Bollinger Band Bounce Strategy"""
    upper, middle, lower = BOLLINGER_BANDS(df['close'], bb_period, bb_std)
    
    # Entry: price touches or breaks below lower band, then recovers
    entry = (df['close'] <= lower) & (df['close'].shift(1) > lower.shift(1))
    
    # Exit: price reaches middle band
    exit = df['close'] >= middle
    
    return entry, exit

# ============================================================
# STRATEGY 4: EMA Crossover
# Buy fast EMA crosses above slow EMA
# ============================================================
def strategy_ema_crossover(df: pd.DataFrame, 
                           fast_period: int = 9, 
                           slow_period: int = 21) -> tuple:
    """EMA Crossover Strategy"""
    ema_fast = EMA(df['close'], fast_period)
    ema_slow = EMA(df['close'], slow_period)
    
    entry = (ema_fast > ema_slow) & (ema_fast.shift(1) <= ema_slow.shift(1))
    exit = (ema_fast < ema_slow) & (ema_fast.shift(1) >= ema_slow.shift(1))
    
    return entry, exit

# ============================================================
# STRATEGY 5: Trend Following with ADX
# Only trade when ADX shows strong trend
# ============================================================
def strategy_adx_trend(df: pd.DataFrame, 
                       adx_threshold: int = 25,
                       adx_period: int = 14) -> tuple:
    """ADX Trend Following Strategy"""
    adx = ADX(df['high'], df['low'], df['close'], adx_period)
    ema_fast = EMA(df['close'], 9)
    ema_slow = EMA(df['close'], 21)
    
    # Strong trend + bullish crossover
    strong_trend = adx > adx_threshold
    
    entry = strong_trend & (ema_fast > ema_slow) & (ema_fast.shift(1) <= ema_slow.shift(1))
    exit = (ema_fast < ema_slow) | (adx < adx_threshold)
    
    return entry, exit

# ============================================================
# STRATEGY 6: Stochastic Reversal
# Buy when stochastic is oversold, sell when overbought
# ============================================================
def strategy_stochastic_reversal(df: pd.DataFrame,
                                  k_period: int = 14,
                                  oversold: int = 20,
                                  overbought: int = 80) -> tuple:
    """Stochastic Reversal Strategy"""
    k, d = STOCHASTIC(df['high'], df['low'], df['close'], k_period)
    
    entry = (k < oversold) & (k.shift(1) >= oversold) & (k > d)
    exit = (k > overbought) & (k.shift(1) <= overbought)
    
    return entry, exit

# ============================================================
# STRATEGY 7: Volume Spike Breakout
# Buy on volume spike + price breakout
# ============================================================
def strategy_volume_breakout(df: pd.DataFrame, 
                              volume_ma_period: int = 20,
                              volume_multiplier: float = 2.0) -> tuple:
    """Volume Spike Breakout Strategy"""
    volume_ma = VOLUME_SMA(df['volume'], volume_ma_period)
    high_20 = df['high'].rolling(20).max()
    
    # Volume spike
    volume_spike = df['volume'] > (volume_ma * volume_multiplier)
    
    # Breakout above 20-day high
    breakout = (df['close'] > high_20.shift(1)) & (df['close'].shift(1) <= high_20.shift(2))
    
    entry = volume_spike & breakout
    
    # Exit on opposite signal or after X candles
    exit = df['close'] < df['close'].shift(5)  # Hold for 5 candles
    
    return entry, exit

# ============================================================
# STRATEGY 8: RSI + MACD Confluence
# Entry only when both RSI and MACD agree
# ============================================================
def strategy_rsi_macd_confluence(df: pd.DataFrame) -> tuple:
    """RSI + MACD Confluence Strategy"""
    rsi = RSI(df['close'])
    macd, signal, hist = MACD(df['close'])
    
    # Bullish: RSI crossing up from oversold + MACD bullish crossover
    rsi_bullish = (rsi < 35) & (rsi.shift(1) >= 35)
    macd_bullish = (macd > signal) & (macd.shift(1) <= signal.shift(1))
    
    # Bearish: RSI crossing down from overbought + MACD bearish crossover
    rsi_bearish = (rsi > 65) & (rsi.shift(1) <= 65)
    macd_bearish = (macd < signal) & (macd.shift(1) >= signal.shift(1))
    
    entry = rsi_bullish | (macd_bullish & rsi < 45)
    exit = rsi_bearish | macd_bearish
    
    return entry, exit

# ============================================================
# STRATEGY 9: Support/Resistance Breakout
# Buy on support breakout with volume
# ============================================================
def strategy_sr_breakout(df: pd.DataFrame, 
                          lookback: int = 20,
                          volume_confirm: bool = True) -> tuple:
    """Support/Resistance Breakout Strategy"""
    volume_ma = VOLUME_SMA(df['volume'], 20)
    
    # Find local lows (support)
    rolling_min = df['low'].rolling(lookback).min()
    at_support = df['low'] <= rolling_min.shift(1) + (df['close'].std() * 0.5)
    
    # Breakout above recent high
    rolling_max = df['high'].rolling(lookback).max()
    breakout = df['close'] > rolling_max.shift(1)
    
    # Volume confirmation
    volume_ok = df['volume'] > volume_ma if volume_confirm else True
    
    entry = at_support & breakout & volume_ok
    
    # Exit when approaching resistance
    exit = df['close'] < rolling_max * 0.98
    
    return entry, exit

# ============================================================
# STRATEGY 10: Multiple Timeframe Confirmation
# Entry: 1h trend aligns with 4h
# ============================================================
def strategy_mtf_confluence(df: pd.DataFrame) -> tuple:
    """Multi-Timeframe Confluence Strategy (simulated on single TF)"""
    # Use longer period EMA as "higher timeframe" proxy
    ema_9 = EMA(df['close'], 9)
    ema_21 = EMA(df['close'], 21)
    ema_50 = EMA(df['close'], 50)
    
    # Trend: price above 50 EMA
    uptrend = df['close'] > ema_50
    
    # Entry: 9 EMA crosses above 21 EMA (in uptrend)
    entry = uptrend & (ema_9 > ema_21) & (ema_9.shift(1) <= ema_21.shift(1))
    
    # Exit: trend reversal
    exit = (ema_9 < ema_21) | (df['close'] < ema_50)
    
    return entry, exit

# ============================================================
# STRATEGY 11: Candlestick Patterns
# Buy on bullish patterns, sell on bearish
# ============================================================
def strategy_candlestick_patterns(df: pd.DataFrame) -> tuple:
    """Candlestick Pattern Strategy"""
    from backtester import scan_for_patterns
    
    patterns = scan_for_patterns(df)
    
    # Entry: bullish engulfing or hammer
    entry = patterns['ENGULF_BULL'] | patterns['HAMMER']
    
    # Exit: bearish engulfing or shooting star
    exit = patterns['ENGULF_BEAR'] | patterns['SHOOTING_STAR']
    
    return entry, exit

# ============================================================
# STRATEGY 12: Mean Reversion with Volatility
# Buy oversold in low volatility, sell when mean reverts
# ============================================================
def strategy_mean_reversion(df: pd.DataFrame) -> tuple:
    """Mean Reversion Strategy"""
    sma_20 = SMA(df['close'], 20)
    std_20 = df['close'].rolling(20).std()
    
    # Distance from mean in std units
    z_score = (df['close'] - sma_20) / std_20
    
    # Entry: price significantly below mean (oversold)
    entry = z_score < -1.5
    
    # Exit: price returns to mean or above
    exit = z_score > 0
    
    return entry, exit

# ============================================================
# STRATEGY 13: Momentum + Trend
# RSI momentum + trend confirmation
# ============================================================
def strategy_momentum_trend(df: pd.DataFrame) -> tuple:
    """Momentum + Trend Strategy"""
    rsi = RSI(df['close'])
    ema_20 = EMA(df['close'], 20)
    ema_50 = EMA(df['close'], 50)
    
    # Trend: 20 EMA above 50 EMA (uptrend)
    uptrend = ema_20 > ema_50
    
    # Momentum: RSI recovering from oversold
    rsi_recovering = (rsi > 30) & (rsi.shift(1) < 30) & (rsi.shift(2) < rsi.shift(1))
    
    entry = uptrend & rsi_recovering
    
    # Exit: RSI overbought or trend reversal
    exit = (rsi > 65) | (ema_20 < ema_50)
    
    return entry, exit

# ============================================================
# STRATEGY 14: Williams %R Reversal
# Buy when %R hits extreme oversold
# ============================================================
def strategy_williams_r(df: pd.DataFrame, 
                        oversold: int = -80,
                        overbought: int = -20) -> tuple:
    """Williams %R Reversal Strategy"""
    highest = df['high'].rolling(14).max()
    lowest = df['low'].rolling(14).min()
    
    will_r = -100 * ((highest - df['close']) / (highest - lowest))
    
    entry = (will_r < oversold) & (will_r.shift(1) >= oversold)
    exit = (will_r > overbought) & (will_r.shift(1) <= overbought)
    
    return entry, exit

# ============================================================
# STRATEGY 15: Composite Signal (Combined Best)
# Combines multiple indicators
# ============================================================
def strategy_composite(df: pd.DataFrame) -> tuple:
    """Composite Strategy - multiple confirmations"""
    # Get signals from multiple strategies
    rsi_entry, rsi_exit = strategy_rsi_reversal(df)
    macd_entry, macd_exit = strategy_macd_crossover(df)
    ema_entry, ema_exit = strategy_ema_crossover(df)
    stoch_entry, stoch_exit = strategy_stochastic_reversal(df)
    
    # Volume confirmation
    volume_ma = VOLUME_SMA(df['volume'], 20)
    volume_ok = df['volume'] > volume_ma * 1.2
    
    # Require at least 2 indicators to agree + volume
    entry_signals = (rsi_entry.astype(int) + macd_entry.astype(int) + 
                    ema_entry.astype(int) + stoch_entry.astype(int))
    entry = (entry_signals >= 2) & volume_ok
    
    # Exit on any bearish signal
    exit = rsi_exit | macd_exit | ema_exit | stoch_exit
    
    return entry, exit

# ============================================================
# REGISTRY: Map strategy names to functions
# ============================================================
STRATEGIES = {
    'rsi_reversal': strategy_rsi_reversal,
    'macd_crossover': strategy_macd_crossover,
    'bb_bounce': strategy_bb_bounce,
    'ema_crossover': strategy_ema_crossover,
    'adx_trend': strategy_adx_trend,
    'stochastic_reversal': strategy_stochastic_reversal,
    'volume_breakout': strategy_volume_breakout,
    'rsi_macd_confluence': strategy_rsi_macd_confluence,
    'sr_breakout': strategy_sr_breakout,
    'mtf_confluence': strategy_mtf_confluence,
    'candlestick_patterns': strategy_candlestick_patterns,
    'mean_reversion': strategy_mean_reversion,
    'momentum_trend': strategy_momentum_trend,
    'williams_r': strategy_williams_r,
    'composite': strategy_composite,
}
