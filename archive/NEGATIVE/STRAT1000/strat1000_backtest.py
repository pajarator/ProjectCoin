#!/usr/bin/env python3
"""
STRAT1000 - Backtester v4
Simple P&L tracking - no compounding issues
"""
import pandas as pd
import numpy as np
from multiprocessing import Pool, cpu_count
import json, os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

def calc_indicators(df):
    df = df.copy()
    df['sma20'] = df['c'].rolling(20).mean()
    df['bb_mid'] = df['c'].rolling(20).mean()
    df['bb_std'] = df['c'].rolling(20).std()
    df['bb_upper'] = df['bb_mid'] + 2 * df['bb_std']
    df['bb_lower'] = df['bb_mid'] - 2 * df['bb_std']
    df['z'] = (df['c'] - df['sma20']) / df['bb_std']
    delta = df['c'].diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    df['rsi'] = 100 - (100 / (1 + gain / loss))
    low14 = df['l'].rolling(14).min()
    high14 = df['h'].rolling(14).max()
    df['stoch'] = 100 * (df['c'] - low14) / (high14 - low14)
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['vr'] = df['v'] / df['vol_ma']
    df['adr_lo'] = df['l'].rolling(24).min()
    df['adr_hi'] = df['h'].rolling(24).max()
    return df

# Strategies
SCALPING = ['gap_fill', 'pivot_rev', 'engulfing', 'pin_bar', 'volume_spike', 'stochastic']
SWING = ['trend_follow', 'adr_rev', 'breakout', 'double_bottom']
ALL = SCALPING + SWING + ['mean_rev', 'rsi_rev', 'macd_cross', 'bb_bounce', 'adx_break']

entries = {
    'mean_rev': lambda r,p: r['z'] < -p.get('zt',1.5) and r['vr']>p.get('vm',1.2),
    'rsi_rev': lambda r,p: r['rsi']<p.get('ro',30) and r['vr']>p.get('vm',1.2),
    'macd_cross': lambda r,p: r['macd']>r['macd_signal'] if 'macd' in r else False,
    'bb_bounce': lambda r,p: r['c']<=r['bb_lower']*1.02 and r['vr']>p.get('vm',1.3),
    'trend_follow': lambda r,p: r['c']>r['sma20'] and r['vr']>p.get('vm',1.2),
    'adr_rev': lambda r,p: r['c']<=r['adr_lo']+(r['adr_hi']-r['adr_lo'])*0.25,
    'stochastic': lambda r,p: r['stoch']<20,
    'volume_spike': lambda r,p: r['vr']>2.0,
    'adx_break': lambda r,p: r['vr']>p.get('vm',1.2) and r['c']>r['sma20']*1.01,
    'gap_fill': lambda r,p: (r['o']-r['c'].shift(1))/r['c'].shift(1) < -0.005,
    'pivot_rev': lambda r,p: r['c'] < r['l'].rolling(5).min()*1.005,
    'engulfing': lambda r,p: r['c']>r['o'] and r['c'].shift(1)<r['o'].shift(1) and r['c']>r['o'].shift(1) and r['o']<r['c'].shift(1),
    'pin_bar': lambda r,p: (min(r['c'],r['o'])-r['l'])>abs(r['c']-r['o'])*2,
    'double_bottom': lambda r,p: r['c']<r['l'].rolling(10).min()*1.02,
    'breakout': lambda r,p: r['c']>r['h'].rolling(20).max().shift(1) and r['vr']>1.5,
}

def run_test(args):
    coin, strat, params, tf = args
    f = f"{DATA_CACHE_DIR}/{coin}_USDT_{tf}_5months.csv"
    if not os.path.exists(f): return None
    try:
        df = pd.read_csv(f, index_col=0, parse_dates=True)
        # Add macd columns
        df['ema12'] = df['c'].ewm(span=12).mean()
        df['ema26'] = df['c'].ewm(span=26).mean()
        df['macd'] = df['ema12'] - df['ema26']
        df['macd_signal'] = df['macd'].ewm(span=9).mean()
        
        df = calc_indicators(df).dropna()
        if len(df) < 50: return None
        
        pnls = []  # Track individual trade PnLs as percentages
        
        for i in range(1, len(df)):
            row = df.iloc[i]
            prev = df.iloc[i-1]
            
            # Entry
            if i >= 20:
                try:
                    if strat in entries and entries[strat](row, params):
                        entry_p = row['c']
                        # Simulate position for 1-5 candles
                        for j in range(i+1, min(i+6, len(df))):
                            exit_row = df.iloc[j]
                            pnl_pct = (exit_row['c'] - entry_p) / entry_p * params['lev']
                            
                            # Check exit conditions
                            if pnl_pct <= -params['sl']:
                                pnls.append(-params['sl'] * 100)
                                break
                            elif pnl_pct >= params['sl'] * 2:
                                pnls.append(params['sl'] * 2 * 100)
                                break
                            elif j - i >= params['min_hold']:
                                if exit_row['c'] > exit_row['sma20']:
                                    pnls.append(pnl_pct * 100)
                                    break
                except: pass
        
        if not pnls: return None
        
        wins = [p for p in pnls if p > 0]
        losses = [p for p in pnls if p <= 0]
        
        return {
            'strat': strat, 'tf': tf, 'coin': coin,
            'pnl': sum(pnls),
            'n': len(pnls),
            'wr': len(wins)/len(pnls)*100 if pnls else 0,
            'avg_w': np.mean(wins) if wins else 0,
            'avg_l': np.mean(losses) if losses else 0,
            'pf': abs(sum(wins)/sum(losses)) if losses and sum(losses)!=0 else 0,
        }
    except: return None

def main():
    print("="*70)
    print("STRAT1000 BACKTESTER v4")
    print(f"CPUs: {cpu_count()}")
    print("="*70)
    
    coins = ['ETH','BTC','SOL','DASH','AVAX']
    p = {'sl':1.0, 'min_hold':2, 'lev':5, 'vm':1.2, 'zt':1.5, 'ro':30}
    
    configs = []
    for c in coins:
        for s in SCALPING: configs.append((c,s,p.copy(),'1m'))
        for s in SWING + ['mean_rev','rsi_rev','macd_cross','bb_bounce','adx_break']: 
            configs.append((c,s,p.copy(),'5m'))
    
    print(f"Testing {len(configs)} configs...")
    
    with Pool(cpu_count()) as pool:
        results = list(pool.map(run_test, configs))
    
    valid = [r for r in results if r]
    
    by = {}
    for r in valid:
        k = f"{r['strat']} ({r['tf']})"
        if k not in by: by[k] = []
        by[k].append(r)
    
    summary = []
    for k, rs in by.items():
        t = sum(r['n'] for r in rs)
        w = sum(r['n']*r['wr'] for r in rs)/t if t else 0
        pnl = sum(r['pnl']*r['n'] for r in rs)/t if t else 0
        aw = sum(r['avg_w']*r['n'] for r in rs)/t if t else 0
        al = sum(r['avg_l']*r['n'] for r in rs)/t if t else 0
        pf = abs(aw/al) if al else 0
        summary.append({'strat':k, 'pnl':pnl, 'wr':w, 'aw':aw, 'al':al, 'pf':pf, 'n':t})
    
    summary.sort(key=lambda x: x['pf'], reverse=True)
    
    print("\n" + "="*70)
    print("TOP STRATEGIES")
    print("="*70)
    print(f"{'Strategy':<25} {'P&L%':>10} {'WR%':>8} {'AvgW%':>8} {'AvgL%':>8} {'PF':>8} {'N':>8}")
    print("-"*75)
    for r in summary[:15]:
        print(f"{r['strat']:<25} {r['pnl']:>+10.1f} {r['wr']:>8.1f} {r['aw']:>+8.2f} {r['al']:>+8.2f} {r['pf']:>8.2f} {r['n']:>8}")
    
    with open('/home/scamarena/ProjectCoin/strat1000_results.json','w') as f:
        json.dump(summary, f, indent=2)
    print("\nSaved!")

if __name__ == "__main__": main()
