#!/usr/bin/env python3
"""
Paper Trading TUI - VWAP Reversion Strategy
Live trading simulation with $100 capital on Binance
"""
import ccxt
import time
import os
import sys
from datetime import datetime
from collections import deque
import signal

# === CONFIGURATION ===
SYMBOL = 'ETH/USDT'
TIMEFRAME = '15m'
INITIAL_CAPITAL = 100
RISK_PER_TRADE = 0.10  # 10% of capital per trade
STOP_LOSS_PCT = 0.02   # 2% stop loss
TAKE_PROFIT_PCT = 0.015  # 1.5% take profit
FEE_RATE = 0.001  # Binance spot fee (0.1%)
SLIPPAGE = 0.0005  # 0.05% slippage

# Colors
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
BOLD = '\033[1m'
RESET = '\033[0m'

class PaperTrader:
    def __init__(self):
        self.exchange = ccxt.binance({'enableRateLimit': True})
        self.balance = INITIAL_CAPITAL
        self.position = None  # {'entry': price, 'size': amount}
        self.trades = []
        self.trade_log = deque(maxlen=50)
        self.running = True
        
        # Indicators
        self.price_history = deque(maxlen=100)
        self.vwap_history = deque(maxlen=100)
        
        signal.signal(signal.SIGINT, self.signal_handler)
    
    def signal_handler(self, sig, frame):
        self.running = False
    
    def calculate_vwap(self, df):
        """Calculate VWAP"""
        typical_price = (df['high'] + df['low'] + df['close']) / 3
        vwap = (typical_price * df['volume']).sum() / df['volume'].sum()
        return vwap
    
    def calculate_indicators(self, df):
        """Calculate indicators for VWAP Reversion"""
        # SMA 20
        sma_20 = df['close'].rolling(20).mean()
        
        # Standard deviation
        std_20 = df['close'].rolling(20).std()
        
        # Z-score
        z_score = (df['close'].iloc[-1] - sma_20.iloc[-1]) / std_20.iloc[-1]
        
        # VWAP (using recent data)
        typical_price = (df['high'] + df['low'] + df['close']) / 3
        vwap = (typical_price * df['volume']).sum() / df['volume'].sum()
        
        # Volume SMA
        volume_ma = df['volume'].rolling(20).mean()
        
        return {
            'close': df['close'].iloc[-1],
            'sma_20': sma_20.iloc[-1],
            'std_20': std_20.iloc[-1],
            'z_score': z_score,
            'vwap': vwap,
            'volume_ma': volume_ma.iloc[-1],
            'volume': df['volume'].iloc[-1]
        }
    
    def check_entry_signal(self, df):
        """VWAP Reversion Entry Signal"""
        if len(df) < 25:
            return False
        
        ind = self.calculate_indicators(df)
        
        # Z-score < -1.5 (oversold)
        z_oversold = ind['z_score'] < -1.5
        
        # Price below SMA 20 (mean reversion setup)
        below_sma = ind['close'] < ind['sma_20']
        
        # Volume confirmation
        volume_ok = ind['volume'] > ind['volume_ma'] * 1.2
        
        return z_oversold & below_sma & volume_ok
    
    def check_exit_signal(self, df, position_price):
        """Exit signals"""
        ind = self.calculate_indicators(df)
        current_price = ind['close']
        
        # Calculate P&L
        pnl_pct = (current_price - position_price) / position_price
        
        # Take profit
        if pnl_pct >= TAKE_PROFIT_PCT:
            return 'take_profit', pnl_pct
        
        # Stop loss
        if pnl_pct <= -STOP_LOSS_PCT:
            return 'stop_loss', pnl_pct
        
        # Price back above SMA 20
        if current_price > ind['sma_20']:
            return 'sma_exit', pnl_pct
        
        # Z-score > 0 (reverted to mean)
        if ind['z_score'] > 0:
            return 'mean_reverted', pnl_pct
        
        return None, pnl_pct
    
    def execute_entry(self, price):
        """Execute paper trade entry"""
        # Calculate position size
        risk_amount = self.balance * RISK_PER_TRADE
        position_size = risk_amount / STOP_LOSS_PCT  # Size based on stop loss
        
        # Account for fees
        fee = position_size * FEE_RATE
        actual_size = (self.balance * RISK_PER_TRADE) / (price * (1 + SLIPPAGE))
        
        self.position = {
            'entry': price * (1 + SLIPPAGE),  # Buy with slippage
            'size': actual_size,
            'entry_time': datetime.now()
        }
        
        self.balance -= fee
        return actual_size
    
    def execute_exit(self, price, reason, pnl_pct):
        """Execute paper trade exit"""
        if not self.position:
            return
        
        exit_price = price * (1 - SLIPPAGE)  # Sell with slippage
        fee = self.position['size'] * FEE_RATE
        
        # Calculate P&L
        pnl = self.position['size'] * (exit_price - self.position['entry'])
        self.balance += self.position['size'] * exit_price - fee
        
        trade = {
            'entry': self.position['entry'],
            'exit': exit_price,
            'size': self.position['size'],
            'pnl': pnl,
            'pnl_pct': pnl_pct * 100,
            'reason': reason,
            'time': datetime.now() - self.position['entry_time']
        }
        
        self.trades.append(trade)
        self.trade_log.append(trade)
        self.position = None
        
        return trade
    
    def get_status(self):
        """Get current trading status"""
        if self.position:
            current_price = self.price_history[-1] if self.price_history else 0
            pnl_pct = (current_price - self.position['entry']) / self.position['entry'] * 100 if current_price else 0
            return 'LONG', pnl_pct
        return 'CASH', 0
    
    def draw_screen(self, current_price, indicators):
        """Draw TUI screen"""
        # Clear screen
        os.system('clear' if os.name == 'posix' else 'cls')
        
        status, pnl = self.get_status()
        status_color = GREEN if status == 'CASH' or pnl > 0 else RED
        
        print(f"{BOLD}{'='*70}")
        print(f"  🦔 COINCLAW PAPER TRADING - VWAP REVERSION")
        print(f"{'='*70}{RESET}")
        print()
        print(f"  📊 MARKET | {SYMBOL} | {TIMEFRAME} | {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"  💰 CAPITAL | ${self.balance:.2f} | Initial: ${INITIAL_CAPITAL}")
        print(f"  📈 POSITION | {status_color}{status}{RESET} | P&L: {GREEN if pnl >= 0 else RED}{pnl:+.2f}%{RESET}")
        print()
        
        # Current price
        price_color = GREEN if pnl >= 0 else RED
        print(f"  ┌────────────────────────────────────────────────────────────────┐")
        print(f"  │  CURRENT PRICE: {price_color}${current_price:.2f}{RESET}                              │")
        print(f"  └────────────────────────────────────────────────────────────────┘")
        print()
        
        # Indicators
        print(f"  📉 INDICATORS:")
        print(f"     SMA(20):   ${ind['sma_20']:.2f}")
        print(f"     VWAP:      ${ind['vwap']:.2f}")
        print(f"     Z-Score:   {ind['z_score']:.2f} {'(oversold!)' if ind['z_score'] < -1.5 else ''}")
        print(f"     Volume:    {ind['volume']:.0f} (MA: {ind['volume_ma']:.0f})")
        print()
        
        # Last 5 trades
        print(f"  📜 RECENT TRADES:")
        print(f"  {'Entry':>10} | {'Exit':>10} | {'P&L %':>8} | {'Reason':>12}")
        print(f"  {'-'*10} | {'-'*10} | {'-'*8} | {'-'*12}")
        
        for trade in list(self.trade_log)[-5:]:
            pnl_color = GREEN if trade['pnl_pct'] > 0 else RED
            print(f"  ${trade['entry']:>8.2f} | ${trade['exit']:>8.2f} | {pnl_color}{trade['pnl_pct']:>+7.2f}%{RESET} | {trade['reason'][:12]:>12}")
        
        print()
        
        # Stats
        if self.trades:
            wins = [t for t in self.trades if t['pnl'] > 0]
            losses = [t for t in self.trades if t['pnl'] <= 0]
            win_rate = len(wins) / len(self.trades) * 100
            avg_win = sum(t['pnl_pct'] for t in wins) / len(wins) if wins else 0
            avg_loss = sum(t['pnl_pct'] for t in losses) / len(losses) if losses else 0
            
            print(f"  📊 STATS:")
            print(f"     Total Trades: {len(self.trades)} | Wins: {len(wins)} | Losses: {len(losses)}")
            print(f"     Win Rate: {GREEN if win_rate >= 70 else YELLOW}{win_rate:.1f}%{RESET}")
            print(f"     Avg Win: {GREEN}+{avg_win:.2f}%{RESET} | Avg Loss: {RED}{avg_loss:.2f}%{RESET}")
        
        print()
        print(f"  ⚙️  CONFIG | Risk: {RISK_PER_TRADE*100:.0f}% | SL: {STOP_LOSS_PCT*100:.0f}% | TP: {TAKE_PROFIT_PCT*100:.1f}% | Fee: {FEE_RATE*100:.1f}%")
        print(f"  {'='*70}")
        print(f"  Press Ctrl+C to stop")
        print()
    
    def run(self):
        """Main trading loop"""
        print(f"{BOLD}Starting Paper Trading...{RESET}")
        print(f"Symbol: {SYMBOL}")
        print(f"Strategy: VWAP Reversion")
        print(f"Capital: ${INITIAL_CAPITAL}")
        print(f"Risk per trade: {RISK_PER_TRADE*100:.0f}%")
        print()
        
        last_fetch = 0
        fetch_interval = 60  # Fetch every 60 seconds (15m candles take time to form)
        
        while self.running:
            try:
                current_time = time.time()
                
                # Fetch new data if needed
                if current_time - last_fetch >= fetch_interval:
                    # Get latest candles
                    ohlcv = self.exchange.fetch_ohlcv(SYMBOL, TIMEFRAME, limit=30)
                    df = __import__('pandas').DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
                    df['timestamp'] = __import__('pandas').to_datetime(df['timestamp'], unit='ms')
                    df.set_index('timestamp', inplace=True)
                    
                    current_price = df['close'].iloc[-1]
                    self.price_history.append(current_price)
                    
                    # Check for entry
                    if self.position is None:
                        if self.check_entry_signal(df):
                            size = self.execute_entry(current_price)
                            print(f"{GREEN}📈 BUY {size:.4f} ETH @ ${current_price:.2f}{RESET}")
                    
                    # Check for exit
                    else:
                        reason, pnl_pct = self.check_exit_signal(df, self.position['entry'])
                        if reason:
                            trade = self.execute_exit(current_price, reason, pnl_pct)
                            pnl_color = GREEN if trade['pnl'] > 0 else RED
                            print(f"{RED}📉 SELL @ ${current_price:.2f} | P&L: {pnl_color}${trade['pnl']:.2f} ({trade['pnl_pct']:+.2f}%){RESET} | Reason: {reason}")
                    
                    last_fetch = current_time
                
                # Calculate indicators for display
                ohlcv = self.exchange.fetch_ohlcv(SYMBOL, TIMEFRAME, limit=30)
                df = __import__('pandas').DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
                df['timestamp'] = __import__('pandas').to_datetime(df['timestamp'], unit='ms')
                df.set_index('timestamp', inplace=True)
                
                indicators = self.calculate_indicators(df)
                current_price = df['close'].iloc[-1]
                
                # Draw screen
                self.draw_screen(current_price, indicators)
                
                time.sleep(10)  # Update every 10 seconds
                
            except Exception as e:
                print(f"Error: {e}")
                time.sleep(10)
        
        # Summary on exit
        print(f"\n{BOLD}Trading Session Ended{RESET}")
        print(f"Final Balance: ${self.balance:.2f}")
        print(f"Total Trades: {len(self.trades)}")
        
        if self.trades:
            wins = [t for t in self.trades if t['pnl'] > 0]
            print(f"Win Rate: {len(wins)/len(self.trades)*100:.1f}%")
            print(f"Total P&L: ${self.balance - INITIAL_CAPITAL:.2f}")

if __name__ == '__main__':
    trader = PaperTrader()
    trader.run()
