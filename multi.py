#!/usr/bin/env python3
"""
Multi-Coin Paper Trading - Top 20 Performers
"""
import ccxt
import pandas as pd
import time
import sys
import json
import os
from datetime import datetime
import argparse

STATE_FILE = '/home/scamarena/ProjectCoin/trading_state.json'

# Parse args
parser = argparse.ArgumentParser()
parser.add_argument('--reset', action='store_true', help='Reset all balances and trades')
args = parser.parse_args()

if args.reset:
    if os.path.exists(STATE_FILE):
        os.remove(STATE_FILE)
    print("State reset!")

# === TOP 20 PERFORMERS ===

# === TOP 20 PERFORMERS ===
COINS = [
    {'symbol': 'ETH/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'ETH'},
    {'symbol': 'NEAR/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'NEAR'},
    {'symbol': 'BTC/USDT', 'tf': '15m', 'strategy': 'bb_bounce', 'name': 'BTC'},
    {'symbol': 'AVAX/USDT', 'tf': '15m', 'strategy': 'bb_bounce', 'name': 'AVAX'},
    {'symbol': 'SOL/USDT', 'tf': '15m', 'strategy': 'vwap_reversion', 'name': 'SOL'},
    {'symbol': 'LTC/USDT', 'tf': '15m', 'strategy': 'adr_reversal', 'name': 'LTC'},
    {'symbol': 'ATOM/USDT', 'tf': '15m', 'strategy': 'dual_rsi', 'name': 'ATOM'},
    {'symbol': 'XLM/USDT', 'tf': '15m', 'strategy': 'vwap_reversion', 'name': 'XLM'},
    {'symbol': 'DOGE/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'DOGE'},
    {'symbol': 'DOT/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'DOT'},
    {'symbol': 'MATIC/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'MATIC'},
    {'symbol': 'LINK/USDT', 'tf': '15m', 'strategy': 'vwap_reversion', 'name': 'LINK'},
    {'symbol': 'ADA/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'ADA'},
    {'symbol': 'BNB/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'BNB'},
    {'symbol': 'TRX/USDT', 'tf': '15m', 'strategy': 'adr_reversal', 'name': 'TRX'},
    {'symbol': 'XRP/USDT', 'tf': '15m', 'strategy': 'vwap_reversion', 'name': 'XRP'},
    {'symbol': 'UNI/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'UNI'},
    {'symbol': 'SHIB/USDT', 'tf': '15m', 'strategy': 'mean_reversion', 'name': 'SHIB'},
    {'symbol': 'DASH/USDT', 'tf': '15m', 'strategy': 'adr_reversal', 'name': 'DASH'},
    {'symbol': 'ALGO/USDT', 'tf': '15m', 'strategy': 'vwap_reversion', 'name': 'ALGO'},
]

INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.02
TAKE_PROFIT = 0.015
FEE = 0.001
SLIP = 0.0005
LOG_FILE = '/home/scamarena/ProjectCoin/trading_log.txt'

# Simple logger
def log_trade(msg):
    with open(LOG_FILE, 'a') as f:
        f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {msg}\n")

# Colors
def GR(s): return f"\033[92m{s}\033[0m"
def RD(s): return f"\033[91m{s}\033[0m"
def YW(s): return f"\033[93m{s}\033[0m"
def CY(s): return f"\033[96m{s}\033[0m"
def MG(s): return f"\033[95m{s}\033[0m"
def BK(s): return f"\033[1m{s}\033[0m"

class Trader:
    def __init__(self, s, tf, strat, name):
        self.sym = s
        self.tf = tf
        self.strat = strat
        self.name = name
        self.ex = ccxt.binance({'enableRateLimit': True})
        self.bal = INITIAL_CAPITAL
        self.pos = None
        self.trades = []
        self.cooldown = 0
        
        # Load persisted state
        self.load_state()
    
    def save_state(self):
        # Load all states, update this trader, save back
        state = {}
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE, 'r') as f:
                    state = json.load(f)
            except:
                pass
        state[self.name] = {
            'bal': self.bal,
            'pos': self.pos,
            'trades': self.trades
        }
        with open(STATE_FILE, 'w') as f:
            json.dump(state, f)
    
    def load_state(self):
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE, 'r') as f:
                    state = json.load(f)
                if self.name in state:
                    self.bal = state[self.name].get('bal', INITIAL_CAPITAL)
                    self.pos = state[self.name].get('pos', None)
                    self.trades = state[self.name].get('trades', [])
            except:
                pass
    
    def ind(self):
        d = self.ex.fetch_ohlcv(self.sym, self.tf, 30)
        df = pd.DataFrame(d, columns=['t','o','h','l','c','v'])
        
        sma20 = df['c'].rolling(20).mean()
        std20 = df['c'].rolling(20).std()
        
        return {
            'p': df['c'].iloc[-1],
            'z': (df['c'].iloc[-1] - sma20.iloc[-1]) / std20.iloc[-1],
            'sma': sma20.iloc[-1],
            'bb_lo': (sma20 - 2*std20).iloc[-1],
            'vol': df['v'].iloc[-1],
            'vol_ma': df['v'].rolling(20).mean().iloc[-1],
            'adr_lo': df['l'].rolling(24).min().iloc[-1],
            'adr_hi': df['h'].rolling(24).max().iloc[-1],
        }
    
    def entry(self, i):
        if self.cooldown > 0:
            self.cooldown -= 1
            return False
        if self.strat == 'mean_reversion':
            return i['z'] < -1.5
        elif self.strat == 'vwap_reversion':
            return i['z'] < -1.5 and i['p'] < i['sma'] and i['vol'] > i['vol_ma']*1.2
        elif self.strat == 'bb_bounce':
            return i['p'] <= i['bb_lo']*1.02 and i['vol'] > i['vol_ma']*1.3
        elif self.strat == 'adr_reversal':
            return i['p'] <= i['adr_lo'] + (i['adr_hi'] - i['adr_lo']) * 0.25
        elif self.strat == 'dual_rsi':
            return i['z'] < -1.0
        return False
    
    def exit(self, i):
        if not self.pos: return None
        pnl = (i['p'] - self.pos['e']) / self.pos['e']

        if pnl >= TAKE_PROFIT:
            return 'TP', pnl
        if pnl <= -STOP_LOSS:
            return 'SL', pnl

        # Only exit on signals if we have gains
        if pnl > 0:
            if i['p'] > i['sma']:
                return 'SMA', pnl
            if i['z'] > 0.5:
                return 'Z0', pnl

        return None
    
    def buy(self, p):
        if self.pos: return
        trade_amt = self.bal * RISK
        sz = trade_amt / p
        fee = trade_amt * FEE
        self.bal = self.bal - trade_amt - fee
        self.pos = {'e': p, 's': sz}
        self.save_state()
        log_trade(f"BUY {self.name} @ ${p:.2f} | Size: {sz:.4f} | Cost: ${trade_amt:.2f} | Balance: ${self.bal:.2f}")
    
    def sell(self, p):
        if not self.pos: return

        proceeds = self.pos['s'] * p
        fee = proceeds * FEE
        net_proceeds = proceeds - fee
        cost = self.pos['s'] * self.pos['e']
        pnl = net_proceeds - cost
        pnl_pct = (pnl / cost) * 100

        self.bal += net_proceeds
        self.trades.append({'pnl': pnl})
        self.pos = None
        self.cooldown = 3
        self.save_state()
        log_trade(f"SELL {self.name} @ ${p:.2f} | PnL: ${pnl:.2f} ({pnl_pct:+.2f}%) | Balance: ${self.bal:.2f}")
        return pnl
    
    def update(self):
        i = self.ind()
        
        if not self.pos and self.entry(i):
            self.buy(i['p'])
            return 'BUY', i
        
        r = self.exit(i)
        if r:
            self.sell(i['p'])
            return 'SELL', r
        
        return 'OK', i

traders = [Trader(c['symbol'], c['tf'], c['strategy'], c['name']) for c in COINS]

def z_color(z):
    if z < -1.5: return GR(f"{z:+.2f}")
    if z < -1.0: return YW(f"{z:+.2f}")
    if z > 1.5: return RD(f"{z:+.2f}")
    return f"{z:+.2f}"

print(BK("\n" + "="*85))
print(" COINCLAW - TOP 20 PERFORMERS | " + str(len(COINS)) + " coins x $" + str(INITIAL_CAPITAL) + " = $" + str(len(COINS)*INITIAL_CAPITAL))
print("="*85 + "\033[0m")

try:
    while True:
        total = 0
        wins = 0
        trades = 0
        
        # Header
        print(f" # COIN     STRAT         PRICE      Z-SCORE   VOL   POS    P&L      BAL     W", flush=True)
        print("-" * 85, flush=True)
        
        for idx, t in enumerate(traders):
            # Always get fresh indicators for display
            try:
                i = t.ind()
            except:
                i = None
            
            if i is None:
                continue
            
            # Process trade signals
            res = t.update()
            
            if res[0] == 'BUY':
                price = i['p']
                print(f" {GR('>>> BUY')} {t.name} ${price:.2f} | Z: {i['z']:+.2f}")
            
            if res[0] == 'SELL':
                pnl_pct = res[1][1] * 100
                c = GR if pnl_pct > 0 else RD
                print(f" {c('<<< SELL')} {t.name} | {res[1][0]} | PnL: {c(f'{pnl_pct:+.2f}%')}")
            
            # Position & P&L
            if t.pos:
                pnl = (i['p'] - t.pos['e']) / t.pos['e'] * 100
                pos = GR("LONG ")
                pnl_str = GR(f"{pnl:+.1f}%") if pnl >= 0 else RD(f"{pnl:+.1f}%")
            else:
                pos = "CASH "
                pnl_str = "-"
            
            bal_c = GR if t.bal >= INITIAL_CAPITAL else RD
            
            vol_r = i['vol'] / i['vol_ma']
            vol_c = GR if vol_r > 1.2 else (YW if vol_r > 0.8 else RD)
            vol_str = vol_c(f"{vol_r:.1f}x")
            
            w = sum(1 for x in t.trades if x['pnl'] > 0)
            win_str = f"{w}/{len(t.trades)}" if t.trades else "-"
            
            wins += w
            trades += len(t.trades)
            
            # Row
            print(f"{idx+1:2} {MG(t.name):<8} {t.strat:<12} ${i['p']:>8.2f}  {z_color(i['z']):>10}  {vol_str:>4}   {pos:<5} {pnl_str:>7} {bal_c('$' + str(int(t.bal))):<8} {win_str}", flush=True)
            
            total += t.bal
        
        # Total
        print("-" * 85, flush=True)
        
        total_pnl = total - len(COINS)*INITIAL_CAPITAL
        pnl_c = GR if total_pnl >= 0 else RD
        wr = f"{wins}/{trades}" if trades > 0 else "-"
        
        print(f" TOTAL: ${int(total)} ({pnl_c(str(int(total_pnl)))}) | {trades} trades | W: {wr} | {datetime.now().strftime('%H:%M:%S')}\n", flush=True)
        
        time.sleep(5)

except KeyboardInterrupt:
    print(f"\n\n{BK('='*60)}")
    print("  SESSION ENDED")
    print("="*60 + "\033[0m\n")
    
    total = 0
    for t in traders:
        total += t.bal
        w = sum(1 for x in t.trades if x['pnl'] > 0)
        print(f" {MG(t.name)}: ${t.bal:.2f} | {len(t.trades)} trades | {w}/{len(t.trades)} wins")
    
    pnl = total - len(COINS)*INITIAL_CAPITAL
    c = GR if pnl >= 0 else RD
    print(f"\n TOTAL: ${total:.2f} ({c(f'{pnl:+.2f}')})\n")
