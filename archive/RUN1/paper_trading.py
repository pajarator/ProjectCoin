#!/usr/bin/env python3
"""
OBSOLETE - Replaced by multi_curses.py
Paper Trading - VWAP Reversion Strategy
Live trading simulation with $100 capital on Binance
"""
import sys
print("This script is obsolete. Use multi_curses.py instead.")
sys.exit(1)
import ccxt
import pandas as pd
import time
from datetime import datetime
import sys

# === CONFIGURATION ===
SYMBOL = 'ETH/USDT'
TIMEFRAME = '15m'
INITIAL_CAPITAL = 100
RISK_PER_TRADE = 0.10  # 10%
STOP_LOSS_PCT = 0.02   # 2%
TAKE_PROFIT_PCT = 0.015  # 1.5%
FEE_RATE = 0.001
SLIPPAGE = 0.0005

# ANSI Colors
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BOLD = '\033[1m'
RESET = '\033[0m'

class PaperTrader:
    def __init__(self):
        self.exchange = ccxt.binance({'enableRateLimit': True})
        self.balance = INITIAL_CAPITAL
        self.position = None
        self.trades = []
        
    def get_data(self):
        ohlcv = self.exchange.fetch_ohlcv(SYMBOL, TIMEFRAME, limit=30)
        df = pd.DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
        df.set_index('timestamp', inplace=True)
        return df
    
    def get_indicators(self, df):
        sma_20 = df['close'].rolling(20).mean()
        std_20 = df['close'].rolling(20).std()
        
        typical_price = (df['high'] + df['low'] + df['close']) / 3
        vwap = (typical_price * df['volume']).sum() / df['volume'].sum()
        
        return {
            'close': df['close'].iloc[-1],
            'sma_20': sma_20.iloc[-1],
            'std_20': std_20.iloc[-1],
            'z_score': (df['close'].iloc[-1] - sma_20.iloc[-1]) / std_20.iloc[-1],
            'vwap': vwap,
            'volume': df['volume'].iloc[-1],
            'volume_ma': df['volume'].rolling(20).mean().iloc[-1]
        }
    
    def check_entry(self, ind):
        return ind['z_score'] < -1.5 and ind['close'] < ind['sma_20'] and ind['volume'] > ind['volume_ma'] * 1.2
    
    def check_exit(self, ind, entry_price):
        pnl_pct = (ind['close'] - entry_price) / entry_price
        
        if pnl_pct >= TAKE_PROFIT_PCT: return 'take_profit', pnl_pct
        if pnl_pct <= -STOP_LOSS_PCT: return 'stop_loss', pnl_pct
        if ind['close'] > ind['sma_20']: return 'sma_exit', pnl_pct
        if ind['z_score'] > 0: return 'mean_reverted', pnl_pct
        return None, pnl_pct
    
    def trade(self, price, direction):
        if direction == 'buy' and self.position is None:
            size = (self.balance * RISK_PER_TRADE) / (price * (1 + SLIPPAGE))
            self.position = {'entry': price * (1 + SLIPPAGE), 'size': size}
            self.balance -= size * price * FEE_RATE
            return f"BUY {size:.4f} @ ${price:.2f}"
        
        elif direction == 'sell' and self.position:
            exit_price = price * (1 - SLIPPAGE)
            pnl = self.position['size'] * (exit_price - self.position['entry'])
            self.balance += self.position['size'] * exit_price - (self.position['size'] * exit_price * FEE_RATE)
            
            trade = {'entry': self.position['entry'], 'exit': exit_price, 'pnl': pnl, 'pnl_pct': pnl / (self.position['size'] * self.position['entry']) * 100}
            self.trades.append(trade)
            self.position = None
            return f"SELL @ ${price:.2f} | PnL: ${pnl:.2f} ({trade['pnl_pct']:+.2f}%)"
        return None
    
    def print_status(self, ind):
        status = 'CASH'
        pnl = 0
        if self.position:
            status = 'LONG'
            pnl = (ind['close'] - self.position['entry']) / self.position['entry'] * 100
        
        pnl_color = GREEN if pnl >= 0 else RED
        
        sys.stdout.write(f"\r  [{datetime.now().strftime('%H:%M:%S')}] Price: ${ind['close']:.2f} | {BOLD}Balance: ${self.balance:.2f}{RESET} | Position: {status} | P&L: {pnl_color}{pnl:+.2f}%{RESET} | Z: {ind['z_score']:.2f}  ")
        sys.stdout.flush()
    
    def run(self):
        print(f"\n{BOLD}{'='*60}")
        print(f"  COINCLAW PAPER TRADING - VWAP REVERSION")
        print(f"{'='*60}{RESET}")
        print(f"  Symbol: {SYMBOL} | Timeframe: {TIMEFRAME}")
        print(f"  Capital: ${INITIAL_CAPITAL} | Risk: {RISK_PER_TRADE*100:.0f}%/trade")
        print(f"  Stop Loss: {STOP_LOSS_PCT*100:.0f}% | Take Profit: {TAKE_PROFIT_PCT*100:.1f}%")
        print(f"{BOLD}{'='*60}{RESET}\n")
        
        last_update = 0
        update_interval = 10
        
        try:
            while True:
                try:
                    df = self.get_data()
                    ind = self.get_indicators(df)
                    
                    # Check entry
                    if self.position is None:
                        signal = self.check_entry(ind)
                        if signal:
                            msg = self.trade(ind['close'], 'buy')
                            print(f"\n  [{datetime.now().strftime('%H:%M:%S')}] {GREEN}{msg}{RESET} | Z: {ind['z_score']:.2f} | Vol: {ind['volume']/ind['volume_ma']:.1f}x")
                    
                    # Check exit
                    else:
                        reason, pnl = self.check_exit(ind, self.position['entry'])
                        if reason:
                            msg = self.trade(ind['close'], 'sell')
                            color = GREEN if pnl > 0 else RED
                            print(f"\n  [{datetime.now().strftime('%H:%M:%S')}] {color}{msg}{RESET} | Reason: {reason}")
                    
                    # Status display
                    if time.time() - last_update >= update_interval:
                        self.print_status(ind)
                        last_update = time.time()
                    
                    time.sleep(1)
                    
                except Exception as e:
                    print(f"\nError: {e}")
                    time.sleep(10)
                    
        except KeyboardInterrupt:
            print(f"\n\n{BOLD}{'='*60}")
            print(f"  SESSION ENDED")
            print(f"{'='*60}{RESET}")
            print(f"  Final Balance: {BOLD}${self.balance:.2f}{RESET}")
            print(f"  Total Trades: {len(self.trades)}")
            if self.trades:
                wins = sum(1 for t in self.trades if t['pnl'] > 0)
                print(f"  Win Rate: {GREEN if wins/len(self.trades) >= 0.7 else YELLOW}{wins/len(self.trades)*100:.1f}%{RESET}")
                print(f"  Total P&L: {GREEN if self.balance >= INITIAL_CAPITAL else RED}${self.balance - INITIAL_CAPITAL:.2f}{RESET}")
            print(f"{BOLD}{'='*60}{RESET}\n")

if __name__ == '__main__':
    trader = PaperTrader()
    trader.run()
