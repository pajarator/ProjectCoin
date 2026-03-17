"""
Backtesting engine for crypto trading strategies.
"""
import pandas as pd
import numpy as np
from typing import Callable, Optional
from dataclasses import dataclass

@dataclass
class Trade:
    """Represents a single trade"""
    entry_time: pd.Timestamp
    entry_price: float
    direction: str  # 'long' or 'short'
    size: float = 1.0
    exit_time: Optional[pd.Timestamp] = None
    exit_price: Optional[float] = None
    pnl_pct: Optional[float] = None
    reason: Optional[str] = None

@dataclass
class BacktestResult:
    """Results of a backtest"""
    trades: list
    total_trades: int
    winning_trades: int
    losing_trades: int
    win_rate: float
    avg_win: float
    avg_loss: float
    profit_factor: float
    max_drawdown: float
    Sharpe_ratio: float
    calmar_ratio: float = 0.0
    recovery_factor: float = 0.0
    total_pnl_pct: float = 0.0
    equity_curve: list = None
    
    def summary(self) -> str:
        return f"""
╔══════════════════════════════════════════════════════════╗
║                    BACKTEST RESULTS                      ║
╠══════════════════════════════════════════════════════════╣
║  Total Trades:     {self.total_trades:>6}                              ║
║  Win Rate:         {self.win_rate:>6.1f}%                             ║
║  Winners:          {self.winning_trades:>6}                              ║
║  Losers:           {self.losing_trades:>6}                              ║
║  Avg Win:          {self.avg_win:>6.2f}%                             ║
║  Avg Loss:         {self.avg_loss:>6.2f}%                             ║
║  Profit Factor:    {self.profit_factor:>6.2f}                             ║
║  Max Drawdown:     {self.max_drawdown:>6.2f}%                             ║
║  Sharpe Ratio:     {self.Sharpe_ratio:>6.2f}                             ║
╚══════════════════════════════════════════════════════════╝
"""

class Backtester:
    """
    Backtesting engine that simulates trades based on entry/exit signals.
    """
    
    def __init__(self, df: pd.DataFrame, initial_balance: float = 10000, 
                 fee: float = 0.001, slippage: float = 0.0005):
        """
        Args:
            df: DataFrame with OHLCV data and indicators
            initial_balance: Starting capital
            fee: Trading fee (0.001 = 0.1%)
            slippage: Slippage percentage
        """
        self.df = df
        self.initial_balance = initial_balance
        self.fee = fee
        self.slippage = slippage
        self.trades: list[Trade] = []
        
    def run(self, entry_signal: pd.Series, exit_signal: pd.Series,
            direction: str = 'long', stop_loss: Optional[float] = None,
            take_profit: Optional[float] = None,
            atr_stop_mult: Optional[float] = None,
            trailing_stop_pct: Optional[float] = None,
            trailing_activation_pct: Optional[float] = None) -> BacktestResult:
        """
        Run backtest with entry and exit signals.

        Args:
            entry_signal: Series of boolean - True when to enter
            exit_signal: Series of boolean - True when to exit
            direction: 'long' or 'short'
            stop_loss: Optional fixed stop loss percentage
            take_profit: Optional take profit percentage
            atr_stop_mult: Optional ATR-based stop loss multiplier.
                           Uses ATR column from df (must exist). Overrides stop_loss.
            trailing_stop_pct: Optional trailing stop percentage.
                               Trails behind highest price since entry.
            trailing_activation_pct: Profit threshold to activate trailing stop.
                                     If None, trailing starts immediately.

        Returns:
            BacktestResult with statistics
        """
        self.trades = []
        balance = self.initial_balance
        position = None
        entry_price = 0
        entry_idx = None
        trailing_peak = 0
        trailing_active = False

        # Pre-compute ATR if needed
        atr_values = None
        if atr_stop_mult is not None:
            from indicators import ATR as ATR_func
            if 'ATR' in self.df.columns:
                atr_values = self.df['ATR'].values
            else:
                atr_values = ATR_func(self.df['high'], self.df['low'], self.df['close'], 14).values

        # Align signals with dataframe
        entry_signal = entry_signal.reindex(self.df.index)
        exit_signal = exit_signal.reindex(self.df.index)

        for i, (idx, row) in enumerate(self.df.iterrows()):
            close = row['close']

            # Check stop loss / take profit / trailing
            if position is not None:
                pnl_pct = (close - entry_price) / entry_price * (1 if direction == 'long' else -1)

                # Determine effective stop loss
                effective_sl = stop_loss
                if atr_stop_mult is not None and atr_values is not None and not np.isnan(atr_values[i]):
                    effective_sl = (atr_values[i] * atr_stop_mult) / entry_price

                if effective_sl and pnl_pct <= -effective_sl:
                    exit_price = entry_price * (1 - effective_sl * (1 + self.slippage))
                    self._close_trade(idx, exit_price, 'stop_loss', direction)
                    position = None
                    trailing_active = False
                    continue

                if take_profit and pnl_pct >= take_profit:
                    exit_price = entry_price * (1 + take_profit * (1 - self.slippage))
                    self._close_trade(idx, exit_price, 'take_profit', direction)
                    position = None
                    trailing_active = False
                    continue

                # Trailing stop
                if trailing_stop_pct is not None:
                    if direction == 'long':
                        trailing_peak = max(trailing_peak, close)
                    else:
                        trailing_peak = min(trailing_peak, close) if trailing_peak > 0 else close

                    # Check activation
                    if trailing_activation_pct is not None:
                        if pnl_pct >= trailing_activation_pct:
                            trailing_active = True
                    else:
                        trailing_active = True

                    if trailing_active:
                        if direction == 'long':
                            trail_stop = trailing_peak * (1 - trailing_stop_pct)
                            if close <= trail_stop:
                                exit_price = close * (1 - self.slippage)
                                self._close_trade(idx, exit_price, 'trailing_stop', direction)
                                position = None
                                trailing_active = False
                                continue
                        else:
                            trail_stop = trailing_peak * (1 + trailing_stop_pct)
                            if close >= trail_stop:
                                exit_price = close * (1 + self.slippage)
                                self._close_trade(idx, exit_price, 'trailing_stop', direction)
                                position = None
                                trailing_active = False
                                continue

            # Entry signal
            if entry_signal.iloc[i] and position is None:
                entry_price = close * (1 + self.slippage if direction == 'long' else 1 - self.slippage)
                self.trades.append(Trade(
                    entry_time=idx,
                    entry_price=entry_price,
                    direction=direction
                ))
                position = True
                entry_idx = i
                trailing_peak = close
                trailing_active = False

            # Exit signal
            elif exit_signal.iloc[i] and position is not None:
                exit_price = close * (1 - self.slippage if direction == 'long' else 1 + self.slippage)
                self._close_trade(idx, exit_price, 'signal', direction)
                position = None
                trailing_active = False

            # Exit without position - ignore

        # Close any open position at the end
        if position is not None:
            exit_price = self.df.iloc[-1]['close']
            self._close_trade(self.df.index[-1], exit_price, 'end_of_data', direction)
        
        return self._calculate_stats()
    
    def _close_trade(self, exit_time: pd.Timestamp, exit_price: float, 
                     reason: str, direction: str):
        """Close a trade and record it."""
        trade = self.trades[-1]
        trade.exit_time = exit_time
        trade.exit_price = exit_price
        
        if direction == 'long':
            trade.pnl_pct = (exit_price - trade.entry_price) / trade.entry_price * 100
        else:
            trade.pnl_pct = (trade.entry_price - exit_price) / trade.entry_price * 100
        
        trade.reason = reason
        
    def _calculate_stats(self) -> BacktestResult:
        """Calculate statistics from trades."""
        if not self.trades:
            return BacktestResult(
                trades=[], total_trades=0, winning_trades=0, losing_trades=0,
                win_rate=0, avg_win=0, avg_loss=0, profit_factor=0,
                max_drawdown=0, Sharpe_ratio=0
            )
        
        pnls = [t.pnl_pct for t in self.trades if t.pnl_pct is not None]
        winning = [p for p in pnls if p > 0]
        losing = [p for p in pnls if p <= 0]
        
        total = len(pnls)
        wins = len(winning)
        losses = len(losing)
        
        win_rate = (wins / total * 100) if total > 0 else 0
        avg_win = np.mean(winning) if winning else 0
        avg_loss = np.mean(losing) if losing else 0
        
        # Profit factor
        total_wins = sum(winning) if winning else 0
        total_losses = abs(sum(losing)) if losing else 1
        profit_factor = total_wins / total_losses if total_losses > 0 else 0
        
        # Max drawdown
        equity = self.initial_balance
        equity_curve = [equity]
        max_equity = equity
        max_dd = 0
        
        for pnl in pnls:
            equity *= (1 + pnl/100)
            equity_curve.append(equity)
            max_equity = max(max_equity, equity)
            dd = (max_equity - equity) / max_equity * 100
            max_dd = max(max_dd, dd)
        
        # Sharpe ratio (simplified)
        returns = np.diff(equity_curve) / equity_curve[:-1]
        sharpe = np.mean(returns) / np.std(returns) * np.sqrt(252) if len(returns) > 1 and np.std(returns) > 0 else 0

        # Total P&L
        total_pnl_pct = (equity - self.initial_balance) / self.initial_balance * 100

        # Calmar ratio: annualized return / max drawdown
        ann_return = np.mean(returns) * 252 * 100 if len(returns) > 0 else 0
        calmar = ann_return / max_dd if max_dd > 0 else 0

        # Recovery factor: total P&L / max drawdown
        recovery = total_pnl_pct / max_dd if max_dd > 0 else 0

        return BacktestResult(
            trades=self.trades,
            total_trades=total,
            winning_trades=wins,
            losing_trades=losses,
            win_rate=win_rate,
            avg_win=avg_win,
            avg_loss=avg_loss,
            profit_factor=profit_factor,
            max_drawdown=max_dd,
            Sharpe_ratio=sharpe,
            calmar_ratio=calmar,
            recovery_factor=recovery,
            total_pnl_pct=total_pnl_pct,
            equity_curve=equity_curve,
        )

def scan_for_patterns(df: pd.DataFrame) -> pd.DataFrame:
    """
    Scan for candlestick patterns and other signals.
    Returns DataFrame with pattern columns (boolean).
    """
    result = df.copy()
    
    # Get typical candle data
    body = abs(result['close'] - result['open'])
    upper_shadow = result['high'] - result[['close', 'open']].max(axis=1)
    lower_shadow = result[['close', 'open']].min(axis=1) - result['low']
    
    # Bullish Engulfing
    result['ENGULF_BULL'] = (
        (result['close'] > result['open']) &  # Current: bullish
        (result['close'].shift(1) < result['open'].shift(1)) &  # Previous: bearish
        (result['close'] > result['open'].shift(1)) &  # Engulfs previous open
        (result['open'] < result['close'].shift(1))  # Engulfs previous close
    )
    
    # Bearish Engulfing
    result['ENGULF_BEAR'] = (
        (result['close'] < result['open']) &  # Current: bearish
        (result['close'].shift(1) > result['open'].shift(1)) &  # Previous: bullish
        (result['close'] < result['open'].shift(1)) &  # Engulfs previous open
        (result['open'] > result['close'].shift(1))  # Engulfs previous close
    )
    
    # Hammer (bullish reversal)
    result['HAMMER'] = (
        (body < upper_shadow) &  # Small body
        (lower_shadow > 2 * body) &  # Long lower shadow
        (upper_shadow < body)  # Little to no upper shadow
    ) & (result['close'] > result['open'])  # Bullish
    
    # Shooting Star (bearish reversal)
    result['SHOOTING_STAR'] = (
        (body < lower_shadow) &  # Small body
        (upper_shadow > 2 * body) &  # Long upper shadow
        (lower_shadow < body)  # Little to no lower shadow
    ) & (result['close'] < result['open'])  # Bearish
    
    # Doji
    result['DOJI'] = body < (result['high'] - result['low']) * 0.1
    
    # Morning Star (3-candle)
    result['MORNING_STAR'] = (
        (result['close'].shift(2) < result['open'].shift(2)) &  # Bearish first
        (body.shift(1) < body.shift(2) * 0.3) &  # Small second candle
        (result['close'] > (result['open'].shift(2) + result['close'].shift(2)) / 2)  # Closes above mid
    ) & (result['close'] > result['open'])
    
    # Three White Soldiers (bullish)
    result['WHITE_SOLDIERS'] = (
        (result['close'] > result['open']) &
        (result['close'].shift(1) > result['open'].shift(1)) &
        (result['close'].shift(2) > result['open'].shift(2)) &
        (result['close'] > result['close'].shift(1)) &
        (result['close'].shift(1) > result['close'].shift(2))
    )
    
    return result
