#!/usr/bin/env python3
"""
OBSOLETE - Replaced by multi_curses.py
Multi-Coin Paper Trading - Multiple strategies per coin
$100 capital per coin on verified setups
"""
import sys
print("This script is obsolete. Use multi_curses.py instead.")
sys.exit(1)
import ccxt
import pandas as pd
import time
from datetime import datetime
import sys
from concurrent.futures import ThreadPoolExecutor
import threading

# === CONFIGURATION ===
# Verified working setups from backtests
COINS = [
    {'symbol': 'ETH/USDT', 'tf': '15m', 'strategy': 'vwap_reversion'},
    {'symbol': 'ETH/USDT', 'tf': '15m', 'strategy': 'mean_reversion'},
    {'symbol': 'BTC/USDT', 'tf': '15m', 'strategy': 'bb_bounce'},
    {'symbol': 'BTC/USDT', 'tf': '15m', 'strategy': 'mean_reversion'},
    {'symbol': 'BNB/USDT', 'tf': '15m', 'strategy': 'mean_reversion'},
    {'symbol': 'SOL/USDT', 'tf': '15m', 'strategy': 'vwap_reversion'},
]

INITIAL_CAPITAL_PER_COIN = 100
RISK_PER_TRADE = 0.10  # 10%
STOP_LOSS_PCT = 0.02   # 2%
TAKE_PROFIT_PCT = 0.015  # 1.5%
FEE_RATE = 0.001
SLIPPAGE = 0.0005

# ANSI Colors
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
CYAN = '\033[96m'
BOLD = '\033[1m'
RESET = '\033[0m'

class CoinTrader:
    def __init__(self, symbol, timeframe, strategy):
        self.symbol = symbol
        self.timeframe = timeframe
        self.strategy = strategy
        self.exchange = ccxt.binance({'enableRateLimit': True})
        self.balance = INITIAL_CAPITAL_PER_COIN
        self.position = None
        self.trades = []
        self.last_signal = None
        
    def get_data(self):
        ohlcv = self.exchange.fetch_ohlcv(self.symbol, self.timeframe, limit=30)
        df = pd.DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
        df.set_index('timestamp', inplace=True)
        return df
    
    def get_indicators(self, df):
        sma_20 = df['close'].rolling(20).mean()
        std_20 = df['close'].rolling(20).std()
        
        typical_price = (df['high'] + df['low'] + df['close']) / 3
        vwap = (typical_price * df['volume']).sum() / df['volume'].sum()
        
        # Bollinger Bands
        bb_upper = sma_20 + 2 * std_20
        bb_lower = sma_20 - 2 * std_20
        
        return {
            'close': df['close'].iloc[-1],
            'sma_20': sma_20.iloc[-1],
            'std_20': std_20.iloc[-1],
            'z_score': (df['close'].iloc[-1] - sma_20.iloc[-1]) / std_20.iloc[-1],
            'vwap': vwap,
            'volume': df['volume'].iloc[-1],
            'volume_ma': df['volume'].rolling(20).mean().iloc[-1],
            'bb_lower': bb_lower.iloc[-1],
            'bb_upper': bb_upper.iloc[-1],
        }
    
    def check_entry(self, ind):
        if self.strategy == 'vwap_reversion':
            # Z < -1.5 + below SMA + volume spike
            return ind['z_score'] < -1.5 and ind['close'] < ind['sma_20'] and ind['volume'] > ind['volume_ma'] * 1.2
        
        elif self.strategy == 'mean_reversion':
            # Z < -1.5
            return ind['z_score'] < -1.5
        
        elif self.strategy == 'bb_bounce':
            # Price at or below lower BB + volume
            return ind['close'] <= ind['bb_lower'] * 1.02 and ind['volume'] > ind['volume_ma'] * 1.3
        
        return False
    
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
    
    def update(self):
        try:
            df = self.get_data()
            ind = self.get_indicators(df)
            
            # Check entry
            if self.position is None:
                signal = self.check_entry(ind)
                if signal:
                    msg = self.trade(ind['close'], 'buy')
                    return {'type': 'entry', 'msg': msg, 'ind': ind}
            
            # Check exit
            else:
                reason, pnl = self.check_exit(ind, self.position['entry'])
                if reason:
                    msg = self.trade(ind['close'], 'sell')
                    return {'type': 'exit', 'msg': msg, 'reason': reason, 'ind': ind}
            
            return {'type': 'status', 'ind': ind}
        except Exception as e:
            return {'type': 'error', 'msg': str(e)}

def print_header():
    print(f"\n{BOLD}{'='*80}")
    print(f"  COINCLAW MULTI-COIN PAPER TRADING")
    print(f"{'='*80}{RESET}")
    print(f"  Running {len(COINS)} coins x ${INITIAL_CAPITAL_PER_COIN} = ${len(COINS) * INITIAL_CAPITAL_PER_COIN} total")
    print(f"  Risk: {RISK_PER_TRADE*100:.0f}%/trade | SL: {STOP_LOSS_PCT*100:.0f}% | TP: {TAKE_PROFIT_PCT*100:.1f}%")
    print(f"{BOLD}{'='*80}{RESET}\n")

def print_traders(traders):
    # Header
    print(f"{'Coin':<12} {'Strategy':<16} {'Price':>10} {'Position':>10} {'P&L':>10} {'Balance':>10} {'Z-Score':>8}")
    print("-" * 80)
    
    total_balance = 0
    
    for t in traders:
        ind = t.update()['ind'] if t.update().get('ind') else {'close': 0, 'z_score': 0}
        
        coin = t.symbol.replace('/USDT', '')
        strat = t.strategy[:14]
        price = ind.get('close', 0)
        
        if t.position:
            pnl = (ind.get('close', 0) - t.position['entry']) / t.position['entry'] * 100
            pos = 'LONG'
            pnl_str = f"{GREEN if pnl >= 0 else RED}{pnl:+.1f}%{RESET}"
        else:
            pnl = 0
            pos = 'CASH'
            pnl_str = '-'
        
        bal_str = f"${t.balance:.2f}"
        z_str = f"{ind.get('z_score', 0):.2f}"
        
        color = GREEN if t.balance >= INITIAL_CAPITAL_PER_COIN else RED
        
        print(f"{CYAN}{coin:<12}{RESET} {strat:<16} ${price:>9.2f} {pos:>10} {pnl_str:>10} {color}{bal_str:<10}{RESET} {z_str:>8}")
        
        total_balance += t.balance
    
    print("-" * 80)
    total_initial = len(COINS) * INITIAL_CAPITAL_PER_COIN
    total_pnl = total_balance - total_initial
    print(f"{BOLD}TOTAL: ${total_balance:.2f} ({'+' if total_pnl >= 0 else ''}{total_pnl:.2f} / {total_pnl/total_initial*100:+.1f}%){RESET}")

def main():
    print_header()
    
    # Initialize traders
    traders = []
    for coin in COINS:
        t = CoinTrader(coin['symbol'], coin['tf'], coin['strategy'])
        traders.append(t)
        print(f"  Started: {coin['symbol']} ({coin['strategy']})")
    
    print(f"\n  Press Ctrl+C to stop\n")
    
    try:
        while True:
            # Update all traders
            results = []
            for t in traders:
                result = t.update()
                results.append(result)
                
                # Print trade notifications
                if result['type'] == 'entry':
                    color = GREEN
                    print(f"\n{color}  [{result['ind']['close']}] {t.symbol}: {result['msg']} | Z: {result['ind']['z_score']:.2f}{RESET}")
                elif result['type'] == 'exit':
                    color = GREEN if result['ind'].get('close', 0) > 0 else RED
                    print(f"\n{color}  [{result['ind']['close']}] {t.symbol}: {result['msg']} | Reason: {result['reason']}{RESET}")
            
            # Print status (single line)
            print(f"\r  [{datetime.now().strftime('%H:%M:%S')}] ", end='')
            total = sum(t.balance for t in traders)
            pnl = total - len(COINS) * INITIAL_CAPITAL_PER_COIN
            color = GREEN if pnl >= 0 else RED
            print(f"Total: {color}${total:.2f}{RESET} | ", end='')
            
            for t in traders:
                status = 'LONG' if t.position else 'CASH'
                print(f"{t.symbol.replace('/USDT','')}:{status} ", end='')
            print("", end='\r')
            
            time.sleep(5)
            
    except KeyboardInterrupt:
        print(f"\n\n{BOLD}{'='*80}")
        print(f"  SESSION ENDED")
        print(f"{'='*80}{RESET}")
        
        total_balance = 0
        total_trades = 0
        
        for t in traders:
            total_balance += t.balance
            total_trades += len(t.trades)
            
            if t.trades:
                wins = sum(1 for tr in t.trades if tr['pnl'] > 0)
                print(f"\n  {CYAN}{t.symbol}{RESET} ({t.strategy}):")
                print(f"    Balance: ${t.balance:.2f}")
                print(f"    Trades: {len(t.trades)} | Wins: {wins} | Win Rate: {GREEN if wins/len(t.trades) >= 0.7 else YELLOW}{wins/len(t.trades)*100:.0f}%{RESET}")
        
        total_initial = len(COINS) * INITIAL_CAPITAL_PER_COIN
        total_pnl = total_balance - total_initial
        
        print(f"\n{BOLD}{'='*80}")
        print(f"  TOTAL")
        print(f"{'='*80}{RESET}")
        print(f"  Initial: ${total_initial:.2f}")
        print(f"  Final:   ${total_balance:.2f}")
        print(f"  P&L:     {GREEN if total_pnl >= 0 else RED}${total_pnl:+.2f} ({total_pnl/total_initial*100:+.1f}%){RESET}")
        print(f"  Trades:  {total_trades}")
        print(f"{BOLD}{'='*80}{RESET}\n")

if __name__ == '__main__':
    main()
