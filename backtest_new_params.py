#!/usr/bin/env python3
"""
Backtest for multi_curses.py strategy - 5 months historical data
"""
import ccxt
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from collections import defaultdict
import time

# === COINS (same as multi_curses.py) ===
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

# === NEW PARAMETERS ===
INITIAL_CAPITAL = 100
RISK = 0.10            # 10% per trade
STOP_LOSS = 0.02       # 2% stop loss
MIN_HOLD_CANDLES = 8   # Must hold at least 8 candles (2h) before SMA exit
FEE = 0.001
LOOKBACK = 30          # candles for indicators

def fetch_data(symbol, tf, months=5):
    """Fetch historical data for a symbol"""
    ex = ccxt.binance({'enableRateLimit': True})
    
    # Calculate start time (months ago)
    since = int((datetime.now() - timedelta(days=30*months)).timestamp() * 1000)
    
    all_candles = []
    max_retries = 3
    
    for retry in range(max_retries):
        try:
            # Fetch in batches of 1000
            while True:
                candles = ex.fetch_ohlcv(symbol, tf, since=since, limit=1000)
                if not candles:
                    break
                all_candles.extend(candles)
                since = candles[-1][0] + 1  # Next batch starts after last
                if len(candles) < 1000:
                    break
                time.sleep(0.5)
            break
        except Exception as e:
            print(f"  Retry {retry+1}/{max_retries}: {e}")
            time.sleep(2)
    
    if not all_candles:
        return None
    
    df = pd.DataFrame(all_candles, columns=['t','o','h','l','c','v'])
    df['t'] = pd.to_datetime(df['t'], unit='ms')
    df.set_index('t', inplace=True)
    df = df.drop_duplicates()
    
    return df

def calculate_indicators(df):
    """Calculate indicators for strategy"""
    df = df.copy()
    
    # SMAs
    df['sma20'] = df['c'].rolling(20).mean()
    df['std20'] = df['c'].rolling(20).std()
    
    # Z-score
    df['z'] = (df['c'] - df['sma20']) / df['std20']
    
    # Bollinger Bands
    df['bb_lo'] = df['sma20'] - 2 * df['std20']
    
    # Volume
    df['vol_ma'] = df['v'].rolling(20).mean()
    
    # ADR (Average Daily Range)
    df['adr_lo'] = df['l'].rolling(24).min()  # 24 x 15m = 6 hours
    df['adr_hi'] = df['h'].rolling(24).max()
    
    return df

def entry_signal(row, strategy):
    """Check if entry signal fires"""
    if pd.isna(row['z']) or pd.isna(row['v']):
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

def run_backtest(df, strategy_name, coin_name):
    """Run backtest for a single coin"""
    df = calculate_indicators(df)
    
    # Need warmup period
    df = df.dropna()
    if len(df) < 50:
        return None
    
    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    high_price = 0
    trades = []
    cooldown = 0
    candles_held = 0
    
    for i, (idx, row) in enumerate(df.iterrows()):
        price = row['c']
        
        # Track how long we've held the position
        if position:
            candles_held += 1
        
        # Update high price for trailing stop
        if position and price > high_price:
            high_price = price
        
        if position:
            pnl_pct = (price - entry_price) / entry_price
            
            # Check SL
            if pnl_pct <= -STOP_LOSS:
                proceeds = balance * RISK * (1 - STOP_LOSS)
                fee = proceeds * FEE
                net_proceeds = proceeds - fee
                balance = balance - (balance * RISK) + net_proceeds
                trades.append({'pnl_pct': -STOP_LOSS * 100, 'reason': 'SL'})
                position = None
                cooldown = 3
                candles_held = 0
                continue
            
            # Exit on SMA/Z0 signals ONLY after minimum hold time (let winners run!)
            if pnl_pct > 0 and candles_held >= MIN_HOLD_CANDLES:
                if price > row['sma20']:
                    proceeds = balance * RISK * (1 + pnl_pct)
                    fee = proceeds * FEE
                    net_proceeds = proceeds - fee
                    balance = balance - (balance * RISK) + net_proceeds
                    trades.append({'pnl_pct': pnl_pct * 100, 'reason': 'SMA'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] > 0.5:
                    proceeds = balance * RISK * (1 + pnl_pct)
                    fee = proceeds * FEE
                    net_proceeds = proceeds - fee
                    balance = balance - (balance * RISK) + net_proceeds
                    trades.append({'pnl_pct': pnl_pct * 100, 'reason': 'Z0'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
        
        # Cooldown
        if cooldown > 0:
            cooldown -= 1
        
        # Entry signal
        if not position and cooldown == 0:
            if entry_signal(row, strategy_name):
                position = True
                entry_price = price
                high_price = price
                balance -= balance * RISK * (1 + FEE)  # Deduct cost + fee
    
    # Close any open position at end
    if position:
        pnl_pct = (df.iloc[-1]['c'] - entry_price) / entry_price
        proceeds = balance * RISK * (1 + pnl_pct)
        fee = proceeds * FEE
        net_proceeds = proceeds - fee
        balance = balance - (balance * RISK) + net_proceeds
        trades.append({'pnl_pct': pnl_pct * 100, 'reason': 'EOD'})
    
    return {
        'coin': coin_name,
        'strategy': strategy_name,
        'initial': INITIAL_CAPITAL,
        'final': balance,
        'trades': len(trades),
        'trade_list': trades
    }

def main():
    print("=" * 60)
    print("BACKTEST: 6 MONTHS | 20 COINS | NO TP + MIN HOLD")
    print("=" * 60)
    print(f"Risk: {RISK*100:.0f}% | SL: {STOP_LOSS*100:.0f}% | Min Hold: {MIN_HOLD_CANDLES} candles")
    print("=" * 60)
    
    all_results = []
    total_trades = 0
    total_pnl = 0
    
    for coin in COINS:
        print(f"\n📊 {coin['name']} ({coin['symbol']}) - {coin['strategy']}")
        
        # Fetch data - 6 months
        df = fetch_data(coin['symbol'], coin['tf'], months=6)
        if df is None or len(df) < 100:
            print(f"  ⚠️  Skipping - not enough data")
            continue
        
        print(f"  Data: {df.index[0].strftime('%Y-%m-%d')} to {df.index[-1].strftime('%Y-%m-%d')} ({len(df)} candles)")
        
        # Run backtest
        result = run_backtest(df, coin['strategy'], coin['name'])
        
        if result:
            pnl_pct = (result['final'] - result['initial']) / result['initial'] * 100
            print(f"  Result: ${result['initial']:.0f} → ${result['final']:.2f} ({pnl_pct:+.1f}%) | {result['trades']} trades")
            
            if result['trades'] > 0:
                wins = [t for t in result['trade_list'] if t['pnl_pct'] > 0]
                losses = [t for t in result['trade_list'] if t['pnl_pct'] <= 0]
                win_rate = len(wins) / result['trades'] * 100
                avg_win = np.mean([t['pnl_pct'] for t in wins]) if wins else 0
                avg_loss = np.mean([t['pnl_pct'] for t in losses]) if losses else 0
                print(f"  Win Rate: {win_rate:.1f}% | Avg Win: {avg_win:+.2f}% | Avg Loss: {avg_loss:.2f}%")
            
            all_results.append(result)
            total_trades += result['trades']
            total_pnl += result['final'] - result['initial']
    
    # Summary
    print("\n" + "=" * 60)
    print("OVERALL SUMMARY")
    print("=" * 60)
    
    if all_results:
        total_initial = sum(r['initial'] for r in all_results)
        total_final = sum(r['final'] for r in all_results)
        overall_pnl = (total_final - total_initial) / total_initial * 100
        
        print(f"Total Coins: {len(all_results)}")
        print(f"Total Trades: {total_trades}")
        print(f"Total Initial: ${total_initial:.0f}")
        print(f"Total Final: ${total_final:.2f}")
        print(f"Overall P&L: {overall_pnl:+.2f}%")
        
        # All trades analysis
        all_trades = []
        for r in all_results:
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
            
            # Exit reasons
            reasons = defaultdict(int)
            for t in all_trades:
                reasons[t['reason']] += 1
            print(f"\nExit Reasons:")
            for reason, count in sorted(reasons.items(), key=lambda x: -x[1]):
                print(f"  {reason}: {count}")
    
    print("=" * 60)

if __name__ == "__main__":
    main()
