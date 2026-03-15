#!/usr/bin/env python3
"""
Backtest for OPTIMIZED multi_curses.py strategy - 1 month
With progress indicator, ETA, and state save/resume
"""
import ccxt
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from collections import defaultdict
import time
import json
import os
import sys

# === STATE FILE ===
STATE_FILE = '/home/scamarena/ProjectCoin/backtest_state.json'

# === COINS ===
COINS = [
    {'symbol': 'ETH/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'ETH'},
    {'symbol': 'NEAR/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'NEAR'},
    {'symbol': 'BTC/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'BTC'},
    {'symbol': 'AVAX/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'AVAX'},
    {'symbol': 'SOL/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'SOL'},
    {'symbol': 'LTC/USDT', 'tf': '15m', 'strategy': 'volatility_squeeze', 'name': 'LTC'},
    {'symbol': 'ATOM/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'ATOM'},
    {'symbol': 'XLM/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'XLM'},
    {'symbol': 'DOGE/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'DOGE'},
    {'symbol': 'DOT/USDT', 'tf': '15m', 'strategy': 'volatility_squeeze', 'name': 'DOT'},
    {'symbol': 'MATIC/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'MATIC'},
    {'symbol': 'LINK/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'LINK'},
    {'symbol': 'ADA/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'ADA'},
    {'symbol': 'BNB/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'BNB'},
    {'symbol': 'TRX/USDT', 'tf': '15m', 'strategy': 'volatility_squeeze', 'name': 'TRX'},
    {'symbol': 'XRP/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'XRP'},
    {'symbol': 'UNI/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'UNI'},
    {'symbol': 'SHIB/USDT', 'tf': '15m', 'strategy': 'momentum_breakout', 'name': 'SHIB'},
    {'symbol': 'DASH/USDT', 'tf': '15m', 'strategy': 'volatility_squeeze', 'name': 'DASH'},
    {'symbol': 'ALGO/USDT', 'tf': '15m', 'strategy': 'trend_follow', 'name': 'ALGO'},
]

# === PARAMETERS ===
INITIAL_CAPITAL = 100
RISK = 0.10
LEVERAGE = 5
STOP_LOSS = 0.008
TAKE_PROFIT = 0.025
MIN_HOLD_CANDLES = 2
FEE = 0.001
USE_TRAILING_STOP = True
TRAIL_START = 0.012
TRAIL_DISTANCE = 0.005
COOLDOWN = 1
MONTHS = 1

# === PROGRESS ===
def print_progress(current, total, start_time, message=""):
    """Print progress bar with ETA"""
    elapsed = time.time() - start_time
    rate = current / elapsed if elapsed > 0 else 0
    remaining = (total - current) / rate if rate > 0 else 0
    
    mins = int(remaining // 60)
    secs = int(remaining % 60)
    
    bar_width = 30
    filled = int(bar_width * current / total)
    bar = "█" * filled + "░" * (bar_width - filled)
    
    pct = 100 * current / total
    print(f"\r[{bar}] {pct:.1f}% ({current}/{total}) | ETA: {mins}m {secs}s | {message}", end="", flush=True)

def save_state(coin_idx, results, start_time):
    """Save backtest state"""
    state = {
        'coin_idx': coin_idx,
        'results': results,
        'start_time': start_time,
        'timestamp': time.time()
    }
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f)

def load_state():
    """Load backtest state if exists"""
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE, 'r') as f:
            return json.load(f)
    return None

def clear_state():
    """Clear saved state"""
    if os.path.exists(STATE_FILE):
        os.remove(STATE_FILE)

# === FETCH DATA ===
def fetch_data(symbol, tf, months=MONTHS):
    ex = ccxt.binance({'enableRateLimit': True})
    since = int((datetime.now() - timedelta(days=30*months)).timestamp() * 1000)
    all_candles = []
    
    for retry in range(3):
        try:
            while True:
                candles = ex.fetch_ohlcv(symbol, tf, since=since, limit=1000)
                if not candles:
                    break
                all_candles.extend(candles)
                since = candles[-1][0] + 1
                if len(candles) < 1000:
                    break
                time.sleep(0.3)
            break
        except Exception as e:
            print(f"  Retry {retry+1}/3: {e}")
            time.sleep(1)
    
    if not all_candles:
        return None
    
    df = pd.DataFrame(all_candles, columns=['t','o','h','l','c','v'])
    df['t'] = pd.to_datetime(df['t'], unit='ms')
    df.set_index('t', inplace=True)
    df = df.drop_duplicates()
    return df

# === INDICATORS ===
def calculate_indicators(df):
    df = df.copy()
    df['sma20'] = df['c'].rolling(20).mean()
    df['sma9'] = df['c'].rolling(9).mean()
    df['std20'] = df['c'].rolling(20).std()
    df['z'] = (df['c'] - df['sma20']) / df['std20']
    df['bb_lo'] = df['sma20'] - 2 * df['std20']
    df['bb_hi'] = df['sma20'] + 2 * df['std20']
    df['vol_ma'] = df['v'].rolling(20).mean()
    
    delta = df['c'].diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df['rsi'] = 100 - (100 / (1 + rs))
    
    ema12 = df['c'].ewm(span=12, adjust=False).mean()
    ema26 = df['c'].ewm(span=26, adjust=False).mean()
    df['macd'] = ema12 - ema26
    df['macd_signal'] = df['macd'].ewm(span=9, adjust=False).mean()
    df['macd_hist'] = df['macd'] - df['macd_signal']
    
    high_low = df['h'] - df['l']
    high_close = abs(df['h'] - df['c'].shift())
    low_close = abs(df['l'] - df['c'].shift())
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    df['atr'] = tr.rolling(14).mean()
    
    plus_dm = high_low.where((df['h'] - df['h'].shift()) > (df['l'].shift() - df['l']), 0)
    minus_dm = high_low.where((df['l'].shift() - df['l']) > (df['h'] - df['h'].shift()), 0)
    plus_di = 100 * (plus_dm.rolling(14).mean() / df['atr'])
    minus_di = 100 * (minus_dm.rolling(14).mean() / df['atr'])
    dx = 100 * abs(plus_di - minus_di) / (plus_di + minus_di)
    df['adx'] = dx.rolling(14).mean()
    
    return df

# === SIGNALS ===
def entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')) or pd.isna(row.get('rsi')):
        return False
    
    if strategy == 'momentum_breakout':
        vol_confirm = row['v'] > row['vol_ma'] * 1.8
        rsi_oversold = row['rsi'] < 25
        z_oversold = row['z'] < -2.0
        price_bounce = row['c'] > row['l'] + (row['h'] - row['l']) * 0.5
        return (rsi_oversold or z_oversold) and vol_confirm and price_bounce
    
    elif strategy == 'trend_follow':
        uptrend = row['sma9'] > row['sma20']
        pullback = row['c'] < row['sma20'] * 0.98
        vol_confirm = row['v'] > row['vol_ma'] * 1.3
        adx_strong = row['adx'] > 30
        return uptrend and pullback and vol_confirm and adx_strong
    
    elif strategy == 'volatility_squeeze':
        return row['c'] <= row['bb_lo'] * 1.01 and row['v'] > row['vol_ma'] * 1.5
    
    return False

def exit_signal(row, entry_price, high_price):
    pnl = (row['c'] - entry_price) / entry_price
    
    if pnl <= -STOP_LOSS:
        return 'SL', pnl
    if pnl >= TAKE_PROFIT:
        return 'TP', pnl
    
    if USE_TRAILING_STOP and pnl >= TRAIL_START:
        trail_price = high_price * (1 - TRAIL_DISTANCE)
        if row['c'] < trail_price:
            return 'TS', pnl
    
    if pnl > 0:
        if row['c'] > row['sma20']:
            return 'SMA', pnl
        if row['z'] > 0.5:
            return 'Z0', pnl
        if row['rsi'] > 70:
            return 'RSI', pnl
        if row['macd'] < row['macd_signal'] and row['macd_hist'] < 0:
            return 'MACD', pnl
    
    return None, pnl

# === BACKTEST ===
def run_backtest(df, strategy_name, coin_name):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None
    
    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    high_price = 0
    trades = []
    cooldown = 0
    
    for i, (idx, row) in enumerate(df.iterrows()):
        price = row['c']
        
        if position:
            if price > high_price:
                high_price = price
            
            exit_reason, pnl = exit_signal(row, entry_price, high_price)
            if exit_reason:
                proceeds = balance * RISK * (1 + pnl)
                fee = proceeds * FEE
                net_proceeds = proceeds - fee
                balance = balance - (balance * RISK) + net_proceeds
                trades.append({'pnl_pct': pnl * 100, 'reason': exit_reason})
                position = None
                cooldown = COOLDOWN
        
        if cooldown > 0:
            cooldown -= 1
        
        if not position and cooldown == 0:
            if entry_signal(row, strategy_name):
                position = True
                entry_price = price
                high_price = price
                balance -= balance * RISK * (1 + FEE)
    
    if position:
        pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        proceeds = balance * RISK * (1 + pnl)
        fee = proceeds * FEE
        net_proceeds = proceeds - fee
        balance = balance - (balance * RISK) + net_proceeds
        trades.append({'pnl_pct': pnl * 100, 'reason': 'EOD'})
    
    return {
        'coin': coin_name,
        'strategy': strategy_name,
        'initial': INITIAL_CAPITAL,
        'final': balance,
        'trades': len(trades),
        'trade_list': trades
    }

# === MAIN ===
def main():
    print("=" * 70)
    print("BACKTEST: OPTIMIZED STRATEGY - 1 MONTH")
    print("=" * 70)
    print(f"Risk: {RISK*100:.0f}% | {LEVERAGE}x LEV | SL: {STOP_LOSS*100:.1f}% | TP: {TAKE_PROFIT*100:.1f}%")
    print(f"State file: {STATE_FILE}")
    print("=" * 70)
    
    # Check for resume
    saved = load_state()
    if saved:
        print(f"\n📂 Resuming from saved state...")
        print(f"   Coin index: {saved['coin_idx']}")
        print(f"   Results so far: {len(saved.get('results', []))}")
        start_time = saved.get('start_time', time.time())
        all_results = saved.get('results', [])
        clear = input("   Clear state and start fresh? (y/n): ").strip().lower()
        if clear == 'y':
            clear_state()
            all_results = []
            start_time = time.time()
    else:
        all_results = []
        start_time = time.time()
    
    total = len(COINS)
    completed = len(all_results)
    
    for idx in range(completed, total):
        coin = COINS[idx]
        
        # Progress
        print_progress(idx, total, start_time, f"{coin['name']}")
        
        # Fetch data
        df = fetch_data(coin['symbol'], coin['tf'])
        if df is None or len(df) < 100:
            print(f"\n  ⚠️  Skipping {coin['name']} - no data")
            all_results.append(None)
            save_state(idx + 1, all_results, start_time)
            continue
        
        # Run backtest
        result = run_backtest(df, coin['strategy'], coin['name'])
        
        if result:
            pnl_pct = (result['final'] - result['initial']) / result['initial'] * 100
            print(f"\n  {coin['name']}: ${result['initial']:.0f} → ${result['final']:.2f} ({pnl_pct:+.1f}%) | {result['trades']} trades")
            all_results.append(result)
        else:
            all_results.append(None)
        
        # Save state after each coin
        save_state(idx + 1, all_results, start_time)
    
    print("\n")
    
    # Summary
    valid_results = [r for r in all_results if r is not None]
    
    if valid_results:
        print("=" * 70)
        print("OVERALL SUMMARY")
        print("=" * 70)
        
        total_initial = sum(r['initial'] for r in valid_results)
        total_final = sum(r['final'] for r in valid_results)
        total_trades = sum(r['trades'] for r in valid_results)
        overall_pnl = (total_final - total_initial) / total_initial * 100
        
        print(f"Total Coins: {len(valid_results)}")
        print(f"Total Trades: {total_trades}")
        print(f"Total Initial: ${total_initial:.0f}")
        print(f"Total Final: ${total_final:.2f}")
        print(f"Overall P&L: {overall_pnl:+.2f}%")
        
        all_trades = []
        for r in valid_results:
            all_trades.extend(r['trade_list'])
        
        if all_trades:
            wins = [t for t in all_trades if t['pnl_pct'] > 0]
            losses = [t for t in all_trades if t['pnl_pct'] <= 0]
            win_rate = len(wins) / len(all_trades) * 100
            avg_win = np.mean([t['pnl_pct'] for t in wins]) if wins else 0
            avg_loss = np.mean([t['pnl_pct'] for t in losses]) if losses else 0
            profit_factor = abs(sum(t['pnl_pct'] for t in wins) / sum(t['pnl_pct'] for t in losses)) if losses else 0
            
            print(f"\nWin Rate: {win_rate:.1f}%")
            print(f"Avg Win: {avg_win:+.2f}%")
            print(f"Avg Loss: {avg_loss:.2f}%")
            print(f"Profit Factor: {profit_factor:.2f}")
            
            reasons = defaultdict(int)
            for t in all_trades:
                reasons[t['reason']] += 1
            print(f"\nExit Reasons:")
            for reason, count in sorted(reasons.items(), key=lambda x: -x[1]):
                print(f"  {reason}: {count}")
    
    print("=" * 70)
    clear_state()
    print("✅ Backtest complete! State cleared.")

if __name__ == "__main__":
    main()
