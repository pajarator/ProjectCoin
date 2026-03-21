#!/usr/bin/env python3
"""
RUN4.2 - Per-Coin Strategy Optimization
Find the best strategy for each coin individually
"""
import pandas as pd
import numpy as np
import json
import os
from collections import defaultdict

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

# All strategies to test
STRATEGIES = ['mean_reversion', 'vwap_reversion', 'bb_bounce', 'adr_reversal', 'dual_rsi']

# All coins
COINS = [
    {'symbol': 'ETH/USDT', 'tf': '15m', 'name': 'ETH'},
    {'symbol': 'NEAR/USDT', 'tf': '15m', 'name': 'NEAR'},
    {'symbol': 'BTC/USDT', 'tf': '15m', 'name': 'BTC'},
    {'symbol': 'AVAX/USDT', 'tf': '15m', 'name': 'AVAX'},
    {'symbol': 'SOL/USDT', 'tf': '15m', 'name': 'SOL'},
    {'symbol': 'LTC/USDT', 'tf': '15m', 'name': 'LTC'},
    {'symbol': 'ATOM/USDT', 'tf': '15m', 'name': 'ATOM'},
    {'symbol': 'XLM/USDT', 'tf': '15m', 'name': 'XLM'},
    {'symbol': 'DOGE/USDT', 'tf': '15m', 'name': 'DOGE'},
    {'symbol': 'DOT/USDT', 'tf': '15m', 'name': 'DOT'},
    {'symbol': 'MATIC/USDT', 'tf': '15m', 'name': 'MATIC'},
    {'symbol': 'LINK/USDT', 'tf': '15m', 'name': 'LINK'},
    {'symbol': 'ADA/USDT', 'tf': '15m', 'name': 'ADA'},
    {'symbol': 'BNB/USDT', 'tf': '15m', 'name': 'BNB'},
    {'symbol': 'TRX/USDT', 'tf': '15m', 'name': 'TRX'},
    {'symbol': 'XRP/USDT', 'tf': '15m', 'name': 'XRP'},
    {'symbol': 'UNI/USDT', 'tf': '15m', 'name': 'UNI'},
    {'symbol': 'SHIB/USDT', 'tf': '15m', 'name': 'SHIB'},
    {'symbol': 'DASH/USDT', 'tf': '15m', 'name': 'DASH'},
    {'symbol': 'ALGO/USDT', 'tf': '15m', 'name': 'ALGO'},
]

# Optimal params from RUN4.1
STOP_LOSS = 0.005
MIN_HOLD_CANDLES = 2
RISK = 0.10
LEVERAGE = 5
FEE = 0.001
INITIAL_CAPITAL = 100

def load_cache(symbol, tf, months=5):
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

def run_backtest(df, strategy):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None
    
    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    trades = []
    cooldown = 0
    candles_held = 0
    
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
            
            if price_pnl > 0 and candles_held >= MIN_HOLD_CANDLES:
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
            if entry_signal(row, strategy):
                position = True
                entry_price = price
    
    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'reason': 'EOD'})
    
    if not trades:
        return None
    
    wins = [t for t in trades if t['pnl_pct'] > 0]
    losses = [t for t in trades if t['pnl_pct'] <= 0]
    
    return {
        'strategy': strategy,
        'initial': INITIAL_CAPITAL,
        'final': balance,
        'pnl_pct': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'trades': len(trades),
        'win_rate': len(wins) / len(trades) * 100 if trades else 0,
        'avg_win': np.mean([t['pnl_pct'] for t in wins]) if wins else 0,
        'avg_loss': np.mean([t['pnl_pct'] for t in losses]) if losses else 0,
        'profit_factor': abs(sum(t['pnl_pct'] for t in wins) / sum(t['pnl_pct'] for t in losses)) if losses else 0,
    }

def main():
    print("=" * 70)
    print("RUN4.2 - PER-COIN STRATEGY OPTIMIZATION")
    print("=" * 70)
    print(f"Testing 5 strategies on 20 coins = 100 combinations")
    print(f"Using optimal params: SL={STOP_LOSS*100}%, MIN_HOLD={MIN_HOLD_CANDLES}")
    print("=" * 70)
    
    all_results = {}
    
    for coin in COINS:
        print(f"\n{coin['name']}: ", end="")
        
        df = load_cache(coin['symbol'], coin['tf'])
        if df is None:
            print("No data")
            continue
        
        coin_results = []
        for strat in STRATEGIES:
            result = run_backtest(df, strat)
            if result:
                coin_results.append(result)
        
        if not coin_results:
            print("No valid results")
            continue
        
        # Find best by profit factor
        coin_results.sort(key=lambda x: x['profit_factor'], reverse=True)
        best = coin_results[0]
        
        all_results[coin['name']] = {
            'best_strategy': best['strategy'],
            'best_pf': best['profit_factor'],
            'best_pnl': best['pnl_pct'],
            'all_strategies': coin_results
        }
        
        print(f"Best: {best['strategy']} (PF={best['profit_factor']:.2f}, P&L={best['pnl_pct']:+.1f}%)")
        
        # Show all for this coin
        for r in coin_results:
            print(f"    {r['strategy']}: PF={r['profit_factor']:.2f}, P&L={r['pnl_pct']:+.1f}%, WR={r['win_rate']:.0f}%")
    
    # Summary
    print("\n" + "=" * 70)
    print("OPTIMAL STRATEGY PER COIN")
    print("=" * 70)
    
    # Current assignments
    current = {
        'ETH': 'mean_reversion', 'NEAR': 'mean_reversion', 'BTC': 'bb_bounce',
        'AVAX': 'bb_bounce', 'SOL': 'vwap_reversion', 'LTC': 'adr_reversal',
        'ATOM': 'dual_rsi', 'XLM': 'vwap_reversion', 'DOGE': 'mean_reversion',
        'DOT': 'mean_reversion', 'MATIC': 'mean_reversion', 'LINK': 'vwap_reversion',
        'ADA': 'mean_reversion', 'BNB': 'mean_reversion', 'TRX': 'adr_reversal',
        'XRP': 'vwap_reversion', 'UNI': 'mean_reversion', 'SHIB': 'mean_reversion',
        'DASH': 'adr_reversal', 'ALGO': 'vwap_reversion'
    }
    
    total_current_pf = 0
    total_optimal_pf = 0
    total_current_pnl = 0
    total_optimal_pnl = 0
    
    print(f"\n{'Coin':<8} {'Current':<16} {'Optimal':<16} {'Curr PF':<10} {'Opt PF':<10} {'Change'}")
    print("-" * 80)
    
    for name, data in all_results.items():
        curr_strat = current.get(name, 'N/A')
        curr_result = next((r for r in data['all_strategies'] if r['strategy'] == curr_strat), None)
        curr_pf = curr_result['profit_factor'] if curr_result else 0
        curr_pnl = curr_result['pnl_pct'] if curr_result else 0
        
        opt_strat = data['best_strategy']
        opt_pf = data['best_pf']
        opt_pnl = data['best_pnl']
        
        change = "+" if opt_pf > curr_pf else ""
        
        print(f"{name:<8} {curr_strat:<16} {opt_strat:<16} {curr_pf:<10.2f} {opt_pf:<10.2f} {change}{opt_pf-curr_pf:+.2f}")
        
        total_current_pf += curr_pf
        total_optimal_pf += opt_pf
        total_current_pnl += curr_pnl
        total_optimal_pnl += opt_pnl
    
    print("-" * 80)
    print(f"{'AVG':<8} {'':<16} {'':<16} {total_current_pf/20:<10.2f} {total_optimal_pf/20:<10.2f}")
    
    # Save results
    with open('/home/scamarena/ProjectCoin/per_coin_results.json', 'w') as f:
        json.dump(all_results, f, indent=2)
    
    print("\n" + "=" * 70)
    print("RECOMMENDED STRATEGY ASSIGNMENTS")
    print("=" * 70)
    
    optimal_assignments = {name: data['best_strategy'] for name, data in all_results.items()}
    print("\nCOINS = [")
    for name, strat in optimal_assignments.items():
        symbol = f"{name}/USDT"
        print(f"    {{'symbol': '{symbol}', 'tf': '15m', 'strategy': '{strat}', 'name': '{name}'}},")
    print("]")
    
    print(f"\nTotal expected P&L with optimal: ${total_optimal_pnl:.1f}%")
    print(f"Total expected P&L with current: ${total_current_pnl:.1f}%")
    print(f"Improvement: {total_optimal_pnl - total_current_pnl:+.1f}%")

if __name__ == "__main__":
    main()
