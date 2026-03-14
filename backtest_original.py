#!/usr/bin/env python3
"""
Backtest - ORIGINAL strategy that was showing profits in live trading
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

STATE_FILE = '/home/scamarena/ProjectCoin/backtest_state.json'
DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
MONTHS = 5

# === ORIGINAL COINS & STRATEGIES ===
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

# === ORIGINAL PARAMETERS (FROM LIVE TRADING) ===
INITIAL_CAPITAL = 100
RISK = 0.10            # 10% per trade
LEVERAGE = 5           # 5x leverage
STOP_LOSS = 0.015      # 1.5% stop loss (tighter to reduce avg loss)
MIN_HOLD_CANDLES = 8   # Hold at least 8 candles before SMA exit
FEE = 0.001

def print_progress(current, total, start_time, message=""):
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
    state = {'coin_idx': coin_idx, 'results': results, 'start_time': start_time, 'timestamp': time.time()}
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f)

def load_state():
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE, 'r') as f:
            return json.load(f)
    return None

def clear_state():
    if os.path.exists(STATE_FILE):
        os.remove(STATE_FILE)

def fetch_data(symbol, tf, months=MONTHS):
    """Fetch data with caching - downloads once, reuses forever"""
    # Create cache directory
    os.makedirs(DATA_CACHE_DIR, exist_ok=True)
    
    # Cache file path (CSV format)
    safe_symbol = symbol.replace('/', '_')
    cache_file = f"{DATA_CACHE_DIR}/{safe_symbol}_{tf}_{months}months.csv"
    
    # Check cache
    if os.path.exists(cache_file):
        print(f"  📂 Loading from cache: {cache_file}")
        df = pd.read_csv(cache_file, index_col=0, parse_dates=True)
        print(f"  ✓ Loaded {len(df)} candles from cache")
        return df
    
    # Fetch from API
    print(f"  ⬇️  Fetching {symbol} ({months} months)...")
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
    
    # Save to cache
    df.to_csv(cache_file)
    print(f"  ✓ Saved {len(df)} candles to cache")
    
    return df

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

def exit_signal(row, entry_price):
    pnl = (row['c'] - entry_price) / entry_price
    if pnl <= -STOP_LOSS:
        return 'SL', pnl
    if pnl > 0 and row['c'] > row['sma20']:
        return 'SMA', pnl
    if pnl > 0 and row['z'] > 0.5:
        return 'Z0', pnl
    return None, pnl

def run_backtest(df, strategy_name, coin_name):
    """Run backtest with proper leverage handling"""
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None
    
    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    position_size = 0  # Size in coins
    trades = []
    cooldown = 0
    candles_held = 0
    
    for i, (idx, row) in enumerate(df.iterrows()):
        price = row['c']
        
        if position:
            candles_held += 1
            
            # Calculate PnL 
            price_pnl = (price - entry_price) / entry_price  # Raw price change (e.g., -0.02 = -2%)
            
            # Check SL first (leveraged)
            leveraged_pnl_pct = price_pnl * LEVERAGE  # e.g., -0.02 * 5 = -0.10 = -10% on margin
            
            if leveraged_pnl_pct <= -STOP_LOSS * LEVERAGE:
                # Stop loss hit
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'reason': 'SL'})
                position = None
                cooldown = 3
                candles_held = 0
                continue
            
            # Exit on signals (only if in profit and held min candles)
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
        
        # Entry
        if not position and cooldown == 0:
            if entry_signal(row, strategy_name):
                position = True
                entry_price = price
                position_size = balance * RISK * LEVERAGE / price  # Size in coins
    
    # Close any open position at end
    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'reason': 'EOD'})
    
    return {'coin': coin_name, 'strategy': strategy_name, 'initial': INITIAL_CAPITAL, 'final': balance, 'trades': len(trades), 'trade_list': trades}

def main():
    print("=" * 70)
    print(f"BACKTEST: ORIGINAL STRATEGY (LIVE TRADING) - {MONTHS} MONTHS")
    print("=" * 70)
    print(f"Risk: {RISK*100:.0f}% | {LEVERAGE}x LEV | SL: {STOP_LOSS*100:.1f}% | Min Hold: {MIN_HOLD_CANDLES} candles")
    print(f"Data cache: {DATA_CACHE_DIR}")
    print(f"Strategies: mean_reversion, vwap_reversion, bb_bounce, adr_reversal, dual_rsi")
    print("=" * 70)
    
    saved = load_state()
    if saved:
        print(f"\n📂 Resuming from coin {saved['coin_idx']}...")
        start_time = saved.get('start_time', time.time())
        all_results = saved.get('results', [])
        # Auto-clear for non-interactive runs
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
        print_progress(idx, total, start_time, f"{coin['name']}")
        df = fetch_data(coin['symbol'], coin['tf'])
        if df is None or len(df) < 100:
            all_results.append(None)
            save_state(idx + 1, all_results, start_time)
            continue
        result = run_backtest(df, coin['strategy'], coin['name'])
        if result:
            pnl_pct = (result['final'] - result['initial']) / result['initial'] * 100
            print(f"\n  {coin['name']}: ${result['initial']:.0f} → ${result['final']:.2f} ({pnl_pct:+.1f}%) | {result['trades']} trades")
            all_results.append(result)
        else:
            all_results.append(None)
        save_state(idx + 1, all_results, start_time)
    
    print("\n")
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
    print("✅ Backtest complete!")

if __name__ == "__main__":
    main()
