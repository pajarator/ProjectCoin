#!/usr/bin/env python3
"""
RUN4.6 - Correlation-Aware Strategy Optimization
Key insight: most coins are 0.7-0.9 correlated. When everything dumps together,
it's a market-wide move — NOT a mean reversion opportunity. Only enter when a
coin dips while the broader market is stable or rising.

Market Breadth Filter:
- Calculate z-score for ALL coins at each candle
- Count how many coins are below z < -1 (bearish breadth)
- Only allow entry when bearish breadth is LOW (coin-specific dip)
- When breadth is HIGH, the dip is market-wide — skip it

Also re-tests: RUN4.1 params, RUN4.2 strategies, genetic params,
with and without the breadth filter, to find the best combo.
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

OPTIMAL_STRAT = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10

# Parameter sets to test
PARAM_SETS = {
    'RUN4.1 (current)': {
        'stop_loss': 0.005, 'min_hold': 2,
        'z_threshold': -1.5, 'bb_margin': 1.02,
        'vol_mult': 1.2, 'adr_pct': 0.25, 'exit_z': 0.5,
    },
    'Genetic (RUN4.4)': {
        'stop_loss': 0.002, 'min_hold': 3,
        'z_threshold': -2.5, 'bb_margin': 1.0,
        'vol_mult': 2.0, 'adr_pct': 0.15, 'exit_z': 0.77,
    },
    'Balanced': {
        'stop_loss': 0.004, 'min_hold': 2,
        'z_threshold': -1.8, 'bb_margin': 1.01,
        'vol_mult': 1.5, 'adr_pct': 0.20, 'exit_z': 0.6,
    },
    'Conservative': {
        'stop_loss': 0.005, 'min_hold': 3,
        'z_threshold': -2.0, 'bb_margin': 1.01,
        'vol_mult': 1.3, 'adr_pct': 0.20, 'exit_z': 0.5,
    },
}

# Breadth thresholds to test: max % of coins bearish to allow entry
BREADTH_THRESHOLDS = [1.0, 0.50, 0.40, 0.30, 0.25, 0.20]


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


def build_market_breadth(all_data):
    """
    Build a time-aligned series of market breadth:
    For each timestamp, what fraction of coins have z < -1?
    """
    z_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        z_frames[coin] = df_ind['z']

    z_df = pd.DataFrame(z_frames)
    z_df = z_df.dropna(how='all')

    # Fraction of coins with z < -1 at each timestamp
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    # Also track average z across all coins
    avg_z = z_df.mean(axis=1)

    return breadth, avg_z


def entry_signal(row, strategy, params):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if strategy == 'mean_reversion':
        return row['z'] < params['z_threshold']
    elif strategy == 'vwap_reversion':
        return (row['z'] < params['z_threshold'] and
                row['c'] < row['sma20'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'bb_bounce':
        return (row['c'] <= row['bb_lo'] * params['bb_margin'] and
                row['v'] > row['vol_ma'] * (params['vol_mult'] + 0.1))
    elif strategy == 'adr_reversal':
        return row['c'] <= row['adr_lo'] + (row['adr_hi'] - row['adr_lo']) * params['adr_pct']
    elif strategy == 'dual_rsi':
        return row['z'] < params['z_threshold'] + 0.5
    return False


def run_backtest(df, strategy, params, breadth=None, breadth_max=1.0):
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

            if price_pnl <= -params['stop_loss']:
                loss = balance * RISK * params['stop_loss'] * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -params['stop_loss'] * LEVERAGE * 100, 'type': 'loss'})
                position = None
                cooldown = 3
                candles_held = 0
                continue

            if price_pnl > 0 and candles_held >= params['min_hold']:
                if row['c'] > row['sma20']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] > params['exit_z']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue

        if cooldown > 0:
            cooldown -= 1

        if not position and cooldown == 0:
            if entry_signal(row, strategy, params):
                # BREADTH FILTER: only enter if market isn't broadly bearish
                if breadth is not None and idx in breadth.index:
                    market_bearish_pct = breadth.loc[idx]
                    if market_bearish_pct > breadth_max:
                        continue  # Skip — too many coins dumping

                position = True
                entry_price = price

    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win' if price_pnl > 0 else 'loss'})

    if not trades:
        return None

    wins = [t for t in trades if t['pnl_pct'] > 0]
    losses = [t for t in trades if t['pnl_pct'] <= 0]
    total_win = sum(t['pnl_pct'] for t in wins) if wins else 0
    total_loss = sum(t['pnl_pct'] for t in losses) if losses else 0
    pf = abs(total_win / total_loss) if total_loss != 0 else 0

    return {
        'pf': pf,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'trades': len(trades),
        'wins': len(wins),
        'wr': len(wins) / len(trades) * 100,
        'avg_win': np.mean([t['pnl_pct'] for t in wins]) if wins else 0,
        'avg_loss': np.mean([t['pnl_pct'] for t in losses]) if losses else 0,
    }


def main():
    print("=" * 90)
    print("RUN4.6 - CORRELATION-AWARE STRATEGY OPTIMIZATION")
    print("=" * 90)
    print("Testing market breadth filter: skip entries when too many coins dump together")
    print(f"Breadth thresholds: {BREADTH_THRESHOLDS}")
    print(f"Parameter sets: {list(PARAM_SETS.keys())}")
    print("=" * 90)

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build market breadth
    print("Building market breadth index...")
    breadth, avg_z = build_market_breadth(all_data)
    print(f"  Avg breadth (% coins with z<-1): {breadth.mean():.1%}")
    print(f"  Max breadth: {breadth.max():.1%}")
    print(f"  Breadth > 50%: {(breadth > 0.5).mean():.1%} of time")
    print(f"  Breadth > 30%: {(breadth > 0.3).mean():.1%} of time")

    # === TEST ALL COMBINATIONS ===
    results = []

    for param_name, params in PARAM_SETS.items():
        for bmax in BREADTH_THRESHOLDS:
            combo_pfs = []
            combo_wrs = []
            combo_trades = []
            combo_pnls = []
            coin_details = {}

            for coin, df in all_data.items():
                strategy = OPTIMAL_STRAT[coin]
                r = run_backtest(df, strategy, params, breadth, bmax)
                if r and r['trades'] > 0:
                    combo_pfs.append(r['pf'])
                    combo_wrs.append(r['wr'])
                    combo_trades.append(r['trades'])
                    combo_pnls.append(r['pnl'])
                    coin_details[coin] = r

            if not combo_pfs:
                continue

            avg_pf = np.mean(combo_pfs)
            avg_wr = np.mean(combo_wrs)
            total_trades = sum(combo_trades)
            avg_pnl = np.mean(combo_pnls)

            bmax_label = f"{bmax*100:.0f}%" if bmax < 1.0 else "OFF"

            results.append({
                'params': param_name,
                'breadth_max': bmax,
                'breadth_label': bmax_label,
                'avg_pf': avg_pf,
                'avg_wr': avg_wr,
                'total_trades': total_trades,
                'avg_pnl': avg_pnl,
                'coin_details': coin_details,
            })

    # === PRINT COMPARISON TABLE ===
    print(f"\n{'='*90}")
    print("RESULTS: ALL PARAMETER + BREADTH COMBINATIONS")
    print(f"{'='*90}")
    print(f"\n{'Params':<22} {'Breadth':<10} {'Avg PF':<10} {'Avg WR':<10} {'Trades':<10} {'Avg P&L':<10}")
    print("-" * 80)

    # Sort by a composite score: PF * sqrt(WR) — rewards both
    results.sort(key=lambda x: x['avg_pf'] * (x['avg_wr'] / 100) ** 0.5, reverse=True)

    for r in results:
        marker = ""
        if r['avg_wr'] >= 60:
            marker = " ***"
        elif r['avg_wr'] >= 55:
            marker = " **"
        elif r['avg_wr'] >= 50:
            marker = " *"
        print(f"{r['params']:<22} {r['breadth_label']:<10} {r['avg_pf']:<10.2f} {r['avg_wr']:<10.1f}% {r['total_trades']:<10} {r['avg_pnl']:+.1f}%{marker}")

    # === BEST CONFIGS BY CATEGORY ===
    print(f"\n{'='*90}")
    print("BEST CONFIGS BY PRIORITY")
    print(f"{'='*90}")

    # Best overall (composite score)
    best = results[0]
    print(f"\n  BEST OVERALL (PF * sqrt(WR)):")
    print(f"    {best['params']} | Breadth={best['breadth_label']} | PF={best['avg_pf']:.2f} WR={best['avg_wr']:.1f}% P&L={best['avg_pnl']:+.1f}%")

    # Best win rate (with PF > 1.3)
    wr_filtered = [r for r in results if r['avg_pf'] > 1.3]
    if wr_filtered:
        best_wr = max(wr_filtered, key=lambda x: x['avg_wr'])
        print(f"\n  HIGHEST WIN RATE (PF>1.3):")
        print(f"    {best_wr['params']} | Breadth={best_wr['breadth_label']} | PF={best_wr['avg_pf']:.2f} WR={best_wr['avg_wr']:.1f}% P&L={best_wr['avg_pnl']:+.1f}%")

    # Best PF (with WR > 50%)
    pf_filtered = [r for r in results if r['avg_wr'] > 50]
    if pf_filtered:
        best_pf = max(pf_filtered, key=lambda x: x['avg_pf'])
        print(f"\n  HIGHEST PF (WR>50%):")
        print(f"    {best_pf['params']} | Breadth={best_pf['breadth_label']} | PF={best_pf['avg_pf']:.2f} WR={best_pf['avg_wr']:.1f}% P&L={best_pf['avg_pnl']:+.1f}%")

    # Best balanced (WR >= 55% AND PF > 1.5)
    balanced = [r for r in results if r['avg_wr'] >= 55 and r['avg_pf'] > 1.5]
    if balanced:
        best_bal = max(balanced, key=lambda x: x['avg_pf'] * x['avg_wr'])
        print(f"\n  BEST BALANCED (WR>=55% AND PF>1.5):")
        print(f"    {best_bal['params']} | Breadth={best_bal['breadth_label']} | PF={best_bal['avg_pf']:.2f} WR={best_bal['avg_wr']:.1f}% P&L={best_bal['avg_pnl']:+.1f}%")

    # === DETAILED BREAKDOWN OF BEST CONFIG ===
    # Use highest WR with PF>1.3 as the "recommended" since user hates low WR
    rec = best_wr if wr_filtered else best
    print(f"\n{'='*90}")
    print(f"RECOMMENDED CONFIG DETAILS: {rec['params']} + Breadth={rec['breadth_label']}")
    print(f"{'='*90}")

    print(f"\n{'Coin':<8} {'Strategy':<16} {'PF':<8} {'WR':<8} {'Trades':<8} {'P&L':<10} {'AvgWin':<10} {'AvgLoss'}")
    print("-" * 80)
    for coin in COINS:
        if coin in rec['coin_details']:
            d = rec['coin_details'][coin]
            print(f"{coin:<8} {OPTIMAL_STRAT[coin]:<16} {d['pf']:<8.2f} {d['wr']:<8.1f}% {d['trades']:<8} {d['pnl']:+<10.1f}% {d['avg_win']:<+10.2f}% {d['avg_loss']:+.2f}%")

    # === BREADTH FILTER IMPACT ===
    print(f"\n{'='*90}")
    print("BREADTH FILTER IMPACT (using RUN4.1 params)")
    print(f"{'='*90}")

    r41_results = [r for r in results if r['params'] == 'RUN4.1 (current)']
    r41_results.sort(key=lambda x: x['breadth_max'], reverse=True)

    if r41_results:
        baseline = r41_results[0]  # No filter
        print(f"\n{'Breadth':<10} {'Avg PF':<10} {'Avg WR':<10} {'Trades':<10} {'PF chg':<10} {'WR chg':<10} {'Trades chg'}")
        print("-" * 75)
        for r in r41_results:
            pf_chg = r['avg_pf'] - baseline['avg_pf']
            wr_chg = r['avg_wr'] - baseline['avg_wr']
            tr_chg = r['total_trades'] - baseline['total_trades']
            print(f"{r['breadth_label']:<10} {r['avg_pf']:<10.2f} {r['avg_wr']:<10.1f}% {r['total_trades']:<10} {pf_chg:+<10.2f} {wr_chg:+<10.1f}% {tr_chg:+}")

    # Save results
    save_data = {
        'breadth_stats': {
            'avg': float(breadth.mean()),
            'max': float(breadth.max()),
            'pct_above_50': float((breadth > 0.5).mean()),
            'pct_above_30': float((breadth > 0.3).mean()),
        },
        'results': [{
            'params': r['params'],
            'breadth_max': r['breadth_max'],
            'avg_pf': r['avg_pf'],
            'avg_wr': r['avg_wr'],
            'total_trades': r['total_trades'],
            'avg_pnl': r['avg_pnl'],
        } for r in results],
        'recommended': {
            'params': rec['params'],
            'breadth_max': rec['breadth_max'],
            'avg_pf': rec['avg_pf'],
            'avg_wr': rec['avg_wr'],
            'coin_details': rec['coin_details'],
        },
    }
    # Save param values for recommended
    for pname, pvals in PARAM_SETS.items():
        if pname == rec['params']:
            save_data['recommended']['param_values'] = pvals
            break

    with open('/home/scamarena/ProjectCoin/correlated_filter_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to correlated_filter_results.json")


if __name__ == "__main__":
    main()
