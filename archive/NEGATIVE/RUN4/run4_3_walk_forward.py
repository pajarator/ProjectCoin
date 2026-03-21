#!/usr/bin/env python3
"""
RUN4.3 - Walk-Forward Analysis
Validate RUN4.2 strategy assignments aren't overfit.
Train on 2 months, test on 1 month, rolling 4 windows.
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

STRATEGIES = ['mean_reversion', 'vwap_reversion', 'bb_bounce', 'adr_reversal', 'dual_rsi']

# RUN4.2 optimal assignments
OPTIMAL = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

COINS = list(OPTIMAL.keys())

# RUN4.1 optimal params
STOP_LOSS = 0.005
MIN_HOLD_CANDLES = 2
RISK = 0.10
LEVERAGE = 5
INITIAL_CAPITAL = 100


def load_cache(name):
    cache_file = f"{DATA_CACHE_DIR}/{name}_USDT_15m_5months.csv"
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

    for idx, row in df.iterrows():
        price = row['c']

        if position:
            candles_held += 1
            price_pnl = (price - entry_price) / entry_price

            if price_pnl <= -STOP_LOSS:
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

    # Close any open position at end
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
        'profit_factor': abs(sum(t['pnl_pct'] for t in wins) / sum(t['pnl_pct'] for t in losses)) if losses and sum(t['pnl_pct'] for t in losses) != 0 else 0,
    }


def main():
    print("=" * 80)
    print("RUN4.3 - WALK-FORWARD ANALYSIS")
    print("=" * 80)
    print("Validating RUN4.2 strategy assignments on rolling out-of-sample windows")
    print("Train: 2 months | Test: 1 month | Step: 1 month | 4 windows")
    print("=" * 80)

    # Define windows (approximate month boundaries in the ~5 month dataset)
    # Data: Oct 15 2025 - Mar 10 2026
    windows = [
        {'name': 'W1', 'train_start': '2025-10-15', 'train_end': '2025-12-14', 'test_start': '2025-12-15', 'test_end': '2026-01-14'},
        {'name': 'W2', 'train_start': '2025-11-15', 'train_end': '2026-01-14', 'test_start': '2026-01-15', 'test_end': '2026-02-14'},
        {'name': 'W3', 'train_start': '2025-12-15', 'train_end': '2026-02-14', 'test_start': '2026-02-15', 'test_end': '2026-03-10'},
    ]

    all_results = {}

    for coin in COINS:
        df = load_cache(coin)
        if df is None:
            continue

        coin_windows = []
        for w in windows:
            train_df = df[(df.index >= w['train_start']) & (df.index < w['train_end'])]
            test_df = df[(df.index >= w['test_start']) & (df.index <= w['test_end'])]

            if len(train_df) < 100 or len(test_df) < 50:
                continue

            # Train: find best strategy on training data
            train_results = {}
            for strat in STRATEGIES:
                r = run_backtest(train_df, strat)
                if r:
                    train_results[strat] = r

            if not train_results:
                continue

            train_best = max(train_results, key=lambda s: train_results[s]['profit_factor'])
            train_best_pf = train_results[train_best]['profit_factor']

            # Test: run train-selected strategy on test data (out-of-sample)
            test_result = run_backtest(test_df, train_best)

            # Also test the RUN4.2 assigned strategy on test data
            assigned_strat = OPTIMAL[coin]
            assigned_result = run_backtest(test_df, assigned_strat)

            # Also test all strategies on test data to find actual best
            test_all = {}
            for strat in STRATEGIES:
                r = run_backtest(test_df, strat)
                if r:
                    test_all[strat] = r
            test_actual_best = max(test_all, key=lambda s: test_all[s]['profit_factor']) if test_all else None

            coin_windows.append({
                'window': w['name'],
                'train_best': train_best,
                'train_best_pf': train_best_pf,
                'test_train_pf': test_result['profit_factor'] if test_result else 0,
                'test_train_pnl': test_result['pnl_pct'] if test_result else 0,
                'assigned_strat': assigned_strat,
                'test_assigned_pf': assigned_result['profit_factor'] if assigned_result else 0,
                'test_assigned_pnl': assigned_result['pnl_pct'] if assigned_result else 0,
                'test_actual_best': test_actual_best,
                'test_actual_best_pf': test_all[test_actual_best]['profit_factor'] if test_actual_best and test_actual_best in test_all else 0,
            })

        if coin_windows:
            all_results[coin] = coin_windows

    # === PRINT RESULTS ===
    print(f"\n{'='*80}")
    print("WALK-FORWARD RESULTS BY COIN")
    print(f"{'='*80}")

    consistent_coins = []
    inconsistent_coins = []

    for coin, wins in all_results.items():
        assigned = OPTIMAL[coin]
        print(f"\n{coin} (assigned: {assigned})")
        print(f"  {'Win':<4} {'Train Best':<16} {'Train PF':<10} {'Test PF':<10} {'Assigned PF':<12} {'Actual Best':<16} {'Actual PF'}")
        print(f"  {'-'*90}")

        avg_test_assigned_pf = 0
        profitable_windows = 0
        for w in wins:
            marker = "*" if w['test_assigned_pf'] >= 1.0 else "!"
            print(f"  {w['window']:<4} {w['train_best']:<16} {w['train_best_pf']:<10.2f} {w['test_train_pf']:<10.2f} {w['test_assigned_pf']:<12.2f} {w['test_actual_best']:<16} {w['test_actual_best_pf']:.2f} {marker}")
            avg_test_assigned_pf += w['test_assigned_pf']
            if w['test_assigned_pf'] >= 1.0:
                profitable_windows += 1

        avg_test_assigned_pf /= len(wins)
        consistency = profitable_windows / len(wins) * 100

        if consistency >= 67:
            consistent_coins.append((coin, assigned, avg_test_assigned_pf, consistency))
        else:
            inconsistent_coins.append((coin, assigned, avg_test_assigned_pf, consistency))

    # === SUMMARY ===
    print(f"\n{'='*80}")
    print("CONSISTENCY SUMMARY")
    print(f"{'='*80}")

    print(f"\nCONSISTENT (profitable in >=67% of test windows):")
    print(f"  {'Coin':<8} {'Strategy':<16} {'Avg Test PF':<12} {'Consistency'}")
    print(f"  {'-'*50}")
    for coin, strat, pf, cons in sorted(consistent_coins, key=lambda x: -x[2]):
        print(f"  {coin:<8} {strat:<16} {pf:<12.2f} {cons:.0f}%")

    print(f"\nINCONSISTENT (profitable in <67% of test windows):")
    print(f"  {'Coin':<8} {'Strategy':<16} {'Avg Test PF':<12} {'Consistency'}")
    print(f"  {'-'*50}")
    for coin, strat, pf, cons in sorted(inconsistent_coins, key=lambda x: -x[2]):
        print(f"  {coin:<8} {strat:<16} {pf:<12.2f} {cons:.0f}%")

    # === STRATEGY STABILITY ===
    print(f"\n{'='*80}")
    print("STRATEGY STABILITY (does train-best match across windows?)")
    print(f"{'='*80}")

    for coin, wins in all_results.items():
        train_picks = [w['train_best'] for w in wins]
        unique = set(train_picks)
        stable = "STABLE" if len(unique) == 1 else f"UNSTABLE ({len(unique)} different)"
        print(f"  {coin:<8} {stable:<20} picks: {', '.join(train_picks)}")

    # === OVERFITTING CHECK ===
    print(f"\n{'='*80}")
    print("OVERFITTING CHECK (train PF vs test PF)")
    print(f"{'='*80}")

    train_pfs = []
    test_pfs = []
    for coin, wins in all_results.items():
        for w in wins:
            train_pfs.append(w['train_best_pf'])
            test_pfs.append(w['test_train_pf'])

    avg_train = np.mean(train_pfs) if train_pfs else 0
    avg_test = np.mean(test_pfs) if test_pfs else 0
    degradation = (1 - avg_test / avg_train) * 100 if avg_train > 0 else 0

    print(f"  Avg Train PF: {avg_train:.2f}")
    print(f"  Avg Test PF:  {avg_test:.2f}")
    print(f"  Degradation:  {degradation:.1f}%")
    if degradation < 20:
        print(f"  Verdict: LOW overfitting risk")
    elif degradation < 40:
        print(f"  Verdict: MODERATE overfitting risk")
    else:
        print(f"  Verdict: HIGH overfitting risk - strategies may not generalize")

    # Save results
    with open('/home/scamarena/ProjectCoin/walk_forward_results.json', 'w') as f:
        json.dump(all_results, f, indent=2)

    print(f"\nResults saved to walk_forward_results.json")


if __name__ == "__main__":
    main()
