#!/usr/bin/env python3
"""
RUN4.1 - Parameter Grid Search
Systematically test parameter combinations to find optimal settings
"""
import ccxt
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from collections import defaultdict
import time
import json
import os
from itertools import product

STATE_FILE = '/home/scamarena/ProjectCoin/backtest_state.json'
DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

# === COINS (same as main) ===
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

# === PARAMETER GRID ===
PARAM_GRID = {
    'STOP_LOSS': [0.005, 0.01, 0.015, 0.02, 0.025],
    'MIN_HOLD_CANDLES': [2, 4, 6, 8, 12, 16],
    'RISK': [0.05, 0.10, 0.15, 0.20],
}
# Fixed params
LEVERAGE = 5
FEE = 0.001
INITIAL_CAPITAL = 100
MONTHS = 5

def load_cache(symbol, tf, months):
    safe_symbol = symbol.replace('/', '_')
    cache_file = f"{DATA_CACHE_DIR}/{safe_symbol}_{tf}_{months}months.csv"
    if os.path.exists(cache_file):
        return pd.read_csv(cache_file, index_col=0, parse_dates=True)
    return None

def calculate_indicators(df):
    df = df.copy()
    df['sma20'] = df['c'].rolling(20).mean()
    df['std20'] = df['c'].rolling(20).std()
    df['z'] = (df['c'] - df['sma20']) / df['std20']
    df['bb_lo'] = df['sma20'] - 2 * df['std20']
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['adr_lo'] = df['l'].rolling(24).min()
    df['adr_hi'] = df['h'].rolling(24).max()
    return df

def entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if strategy == 'mean_reversion':
        return row['z'] < -1.5
    elif strategy == 'vwap_reversion':
        return row['z'] < -1.5 and row['c'] < row['sma20'] and row['v'] > row['vol_ma'] * 1.2
    elif strategy == 'bb_bounce':
        return row['c'] <= row['bb_lo'] * 1.02 and row['v'] > row['vol_ma'] * 1.3
    elif strategy == 'adr_reversal':
        return row['c'] <= row['adr_lo'] + (row['adr_hi'] - row['adr_lo']) * 0.25
    elif strategy == 'dual_rsi':
        return row['z'] < -1.0
    return False

def run_backtest(df, strategy_name, params):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None
    
    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    position_size = 0
    trades = []
    cooldown = 0
    candles_held = 0
    
    STOP_LOSS = params['STOP_LOSS']
    MIN_HOLD = params['MIN_HOLD_CANDLES']
    RISK = params['RISK']
    
    for i, (idx, row) in enumerate(df.iterrows()):
        price = row['c']
        
        if position:
            candles_held += 1
            price_pnl = (price - entry_price) / entry_price
            
            if price_pnl * LEVERAGE <= -STOP_LOSS * LEVERAGE:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'reason': 'SL'})
                position = None
                cooldown = 3
                candles_held = 0
                continue
            
            if price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'reason': 'SMA'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] > 0.5:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'reason': 'Z0'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
        
        if cooldown > 0:
            cooldown -= 1
        
        if not position and cooldown == 0:
            if entry_signal(row, strategy_name):
                position = True
                entry_price = price
                position_size = balance * RISK * LEVERAGE / price
    
    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'reason': 'EOD'})
    
    return {
        'initial': INITIAL_CAPITAL,
        'final': balance,
        'trades': len(trades),
        'trade_list': trades
    }

def run_grid_search():
    # Generate all combinations
    keys = PARAM_GRID.keys()
    values = PARAM_GRID.values()
    combinations = [dict(zip(keys, v)) for v in product(*values)]
    
    print(f"Testing {len(combinations)} parameter combinations...")
    print(f"Grid: SL={PARAM_GRID['STOP_LOSS']}")
    print(f"      MIN_HOLD={PARAM_GRID['MIN_HOLD_CANDLES']}")
    print(f"      RISK={PARAM_GRID['RISK']}")
    print("=" * 70)
    
    results = []
    total = len(combinations)
    
    for idx, params in enumerate(combinations):
        # Progress
        elapsed = idx / total * 100
        print(f"\rProgress: {elapsed:.1f}% ({idx+1}/{total})", end="", flush=True)
        
        all_trades = []
        total_final = 0
        
        for coin in COINS:
            df = load_cache(coin['symbol'], coin['tf'], MONTHS)
            if df is None or len(df) < 50:
                continue
            
            result = run_backtest(df, coin['strategy'], params)
            if result:
                all_trades.extend(result['trade_list'])
                total_final += result['final']
        
        if not all_trades:
            continue
        
        total_initial = len(COINS) * INITIAL_CAPITAL
        overall_pnl = (total_final - total_initial) / total_initial * 100
        
        wins = [t for t in all_trades if t['pnl_pct'] > 0]
        losses = [t for t in all_trades if t['pnl_pct'] <= 0]
        win_rate = len(wins) / len(all_trades) * 100 if all_trades else 0
        avg_win = np.mean([t['pnl_pct'] for t in wins]) if wins else 0
        avg_loss = np.mean([t['pnl_pct'] for t in losses]) if losses else 0
        profit_factor = abs(sum(t['pnl_pct'] for t in wins) / sum(t['pnl_pct'] for t in losses)) if losses else 0
        
        results.append({
            'params': params,
            'pnl': overall_pnl,
            'win_rate': win_rate,
            'avg_win': avg_win,
            'avg_loss': avg_loss,
            'profit_factor': profit_factor,
            'total_trades': len(all_trades),
        })
    
    print("\n\n")
    
    # Sort by profit factor
    results.sort(key=lambda x: x['profit_factor'], reverse=True)
    
    return results

def main():
    print("=" * 70)
    print("RUN4.1 - PARAMETER GRID SEARCH")
    print("=" * 70)
    start_time = time.time()
    
    results = run_grid_search()
    
    elapsed = time.time() - start_time
    print(f"Completed in {elapsed:.1f} seconds")
    
    # Top 10
    print("\n" + "=" * 70)
    print("TOP 10 PARAMETER COMBINATIONS (by Profit Factor)")
    print("=" * 70)
    
    for i, r in enumerate(results[:10]):
        p = r['params']
        print(f"\n#{i+1} | PF: {r['profit_factor']:.2f} | P&L: {r['pnl']:+.1f}% | WR: {r['win_rate']:.1f}%")
        print(f"    SL: {p['STOP_LOSS']*100:.1f}% | MIN_HOLD: {p['MIN_HOLD_CANDLES']} | RISK: {p['RISK']*100:.0f}%")
        print(f"    Avg Win: {r['avg_win']:+.2f}% | Avg Loss: {r['avg_loss']:.2f}% | Trades: {r['total_trades']}")
    
    # Save results
    output = {
        'total_combinations': len(results),
        'top_10': results[:10],
        'all_results': results
    }
    
    with open('/home/scamarena/ProjectCoin/grid_search_results.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print("\n" + "=" * 70)
    print("Results saved to grid_search_results.json")
    print("=" * 70)

if __name__ == "__main__":
    main()
