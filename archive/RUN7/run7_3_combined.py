#!/usr/bin/env python3
"""
RUN7.3 - Combined Backtest: Current SL vs Optimized SL

Compares:
  v7_current  — sl=0.5%, no trailing (current hardcoded behavior)
  v8_optimized — best unified SL params from run7_1/run7_2

Per-coin breakdown and portfolio-level P&L, WR, PF, MaxDD.
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run7_3_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run7_3_results.json'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

OPTIMAL_LONG_STRAT = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

OPTIMAL_SHORT_STRAT = {
    'DASH': 'short_mean_rev', 'UNI': 'short_adr_rev', 'NEAR': 'short_adr_rev',
    'ADA': 'short_bb_bounce', 'LTC': 'short_mean_rev', 'SHIB': 'short_vwap_rev',
    'LINK': 'short_bb_bounce', 'ETH': 'short_adr_rev', 'DOT': 'short_vwap_rev',
    'XRP': 'short_bb_bounce', 'ATOM': 'short_adr_rev', 'SOL': 'short_adr_rev',
    'DOGE': 'short_bb_bounce', 'XLM': 'short_mean_rev', 'AVAX': 'short_bb_bounce',
    'ALGO': 'short_adr_rev', 'BNB': 'short_vwap_rev', 'BTC': 'short_adr_rev',
}

OPTIMAL_ISO_SHORT_STRAT = {}

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
MIN_HOLD = 2

BREADTH_LONG_MAX = 0.20
BREADTH_SHORT_MIN = 0.50
ISO_SHORT_BREADTH_MAX = 0.20

ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, saving checkpoint...")


signal.signal(signal.SIGINT, _sigint_handler)


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
    df['bb_hi'] = df['sma20'] + 2 * df['std20']
    df['bb_width'] = df['bb_hi'] - df['bb_lo']
    df['bb_width_avg'] = df['bb_width'].rolling(20).mean()
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['adr_lo'] = df['l'].rolling(24).min()
    df['adr_hi'] = df['h'].rolling(24).max()
    delta = df['c'].diff()
    gain = (delta.where(delta > 0, 0)).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df['rsi'] = 100 - (100 / (1 + rs))
    return df


def build_market_data(all_data):
    z_frames = {}
    rsi_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        z_frames[coin] = df_ind['z']
        rsi_frames[coin] = df_ind['rsi']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    rsi_df = pd.DataFrame(rsi_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    avg_z = z_df.mean(axis=1)
    avg_rsi = rsi_df.mean(axis=1)
    btc_z = None
    if 'BTC' in all_data:
        btc_df = calculate_indicators(all_data['BTC'])
        btc_z = btc_df['z']
    return breadth, avg_z, avg_rsi, btc_z


def long_entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if row['c'] > row['sma20'] or row['z'] > 0.5:
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


def short_entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if row['c'] < row['sma20'] or row['z'] < -0.5:
        return False
    if strategy == 'short_vwap_rev':
        return row['z'] > 1.5 and row['c'] > row['sma20'] and row['v'] > row['vol_ma'] * 1.2
    elif strategy == 'short_bb_bounce':
        return row['c'] >= row['bb_hi'] * 0.98 and row['v'] > row['vol_ma'] * 1.3
    elif strategy == 'short_mean_rev':
        return row['z'] > 1.5
    elif strategy == 'short_adr_rev':
        adr_range = row['adr_hi'] - row['adr_lo']
        return row['c'] >= row['adr_hi'] - adr_range * 0.25
    return False


def iso_short_entry_signal(row, strategy, params, market_ctx=None):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if row['c'] < row['sma20'] or row['z'] < -0.5:
        return False
    if strategy == 'iso_mean_rev':
        return row['z'] > params['z_threshold']
    elif strategy == 'iso_vwap_rev':
        return (row['z'] > params['z_threshold'] and
                row['c'] > row['sma20'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'iso_bb_bounce':
        return (row['c'] >= row['bb_hi'] * params['bb_margin'] and
                row['v'] > row['vol_ma'] * (params['vol_mult'] + 0.1))
    elif strategy == 'iso_adr_rev':
        adr_range = row['adr_hi'] - row['adr_lo']
        if adr_range <= 0:
            return False
        return (row['c'] >= row['adr_hi'] - adr_range * params['adr_pct'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'iso_relative_z':
        if market_ctx is None or pd.isna(market_ctx.get('avg_z', float('nan'))):
            return False
        return row['z'] > market_ctx['avg_z'] + params['z_spread']
    elif strategy == 'iso_rsi_extreme':
        if market_ctx is None or pd.isna(market_ctx.get('avg_rsi', float('nan'))):
            return False
        if pd.isna(row.get('rsi')):
            return False
        return row['rsi'] > params['rsi_threshold'] and market_ctx['avg_rsi'] < 55
    elif strategy == 'iso_divergence':
        if market_ctx is None or pd.isna(market_ctx.get('btc_z', float('nan'))):
            return False
        return row['z'] > params['z_threshold'] and market_ctx['btc_z'] < 0
    elif strategy == 'iso_vol_spike':
        return (row['z'] > 1.0 and
                row['v'] > row['vol_ma'] * params['vol_spike_mult'])
    elif strategy == 'iso_bb_squeeze':
        if pd.isna(row.get('bb_width_avg')) or row['bb_width_avg'] == 0:
            return False
        return (row['c'] >= row['bb_hi'] * 0.98 and
                row['bb_width'] < row['bb_width_avg'] * params['squeeze_factor'])
    return False


def run_backtest(df, long_strat, short_strat, iso_short_strat, breadth,
                 avg_z_series, avg_rsi_series, btc_z_series,
                 initial_sl=0.005, trail_mode='none', trail_activation=0.0, trail_distance=0.0):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None

    balance = INITIAL_CAPITAL
    peak_balance = INITIAL_CAPITAL
    max_drawdown = 0
    position = None
    entry_price = 0
    peak_price = 0
    trough_price = 0
    trades = []
    long_trades = []
    short_trades = []
    iso_short_trades = []
    cooldown = 0
    candles_held = 0
    entry_type = None
    trail_exits = 0
    reason_counts = {}

    for idx, row in df.iterrows():
        price = row['c']
        b = breadth.loc[idx] if idx in breadth.index else 0
        if b <= BREADTH_LONG_MAX:
            market_mode = 'long'
        elif b >= BREADTH_SHORT_MIN:
            market_mode = 'short'
        else:
            market_mode = 'iso_short'

        # EXIT
        if position == 'long':
            candles_held += 1
            if row['h'] > peak_price:
                peak_price = row['h']
            price_pnl = (price - entry_price) / entry_price
            exited = False
            exit_reason = None

            # 1. Initial SL
            if price_pnl <= -initial_sl:
                balance -= balance * RISK * initial_sl * LEVERAGE
                trade = {'pnl_pct': -initial_sl * LEVERAGE * 100, 'dir': 'long', 'type': 'loss', 'reason': 'SL'}
                trades.append(trade)
                long_trades.append(trade)
                exited = True
                exit_reason = 'SL'

            # 2. Trail/breakeven
            if not exited and trail_mode != 'none':
                peak_pct = (peak_price - entry_price) / entry_price
                if peak_pct >= trail_activation:
                    if trail_mode == 'breakeven' and price <= entry_price:
                        balance += balance * RISK * price_pnl * LEVERAGE
                        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'BE'}
                        trades.append(trade)
                        long_trades.append(trade)
                        exited = True
                        exit_reason = 'BE'
                        trail_exits += 1
                    elif trail_mode == 'trail' and price <= peak_price * (1 - trail_distance):
                        balance += balance * RISK * price_pnl * LEVERAGE
                        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'TRAIL'}
                        trades.append(trade)
                        long_trades.append(trade)
                        exited = True
                        exit_reason = 'TRAIL'
                        trail_exits += 1

            # 3. Signal exits
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20'] or row['z'] > 0.5:
                    balance += balance * RISK * price_pnl * LEVERAGE
                    reason = 'SMA' if row['c'] > row['sma20'] else 'Z0'
                    trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long', 'type': 'win', 'reason': reason}
                    trades.append(trade)
                    long_trades.append(trade)
                    exited = True
                    exit_reason = reason

            if exited:
                reason_counts[exit_reason] = reason_counts.get(exit_reason, 0) + 1
                position = None
                entry_type = None
                cooldown = 3
                candles_held = 0

        elif position == 'short':
            candles_held += 1
            if row['l'] < trough_price:
                trough_price = row['l']
            price_pnl = (entry_price - price) / entry_price
            exited = False
            exit_reason = None

            # 1. Initial SL
            if price_pnl <= -initial_sl:
                balance -= balance * RISK * initial_sl * LEVERAGE
                trade = {'pnl_pct': -initial_sl * LEVERAGE * 100, 'dir': 'short', 'type': 'loss',
                         'reason': 'SL', 'sub': entry_type}
                trades.append(trade)
                short_trades.append(trade)
                if entry_type == 'iso_short':
                    iso_short_trades.append(trade)
                exited = True
                exit_reason = 'SL'

            # 2. Trail/breakeven
            if not exited and trail_mode != 'none':
                trough_pct = (entry_price - trough_price) / entry_price
                if trough_pct >= trail_activation:
                    if trail_mode == 'breakeven' and price >= entry_price:
                        balance += balance * RISK * price_pnl * LEVERAGE
                        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'BE', 'sub': entry_type}
                        trades.append(trade)
                        short_trades.append(trade)
                        if entry_type == 'iso_short':
                            iso_short_trades.append(trade)
                        exited = True
                        exit_reason = 'BE'
                        trail_exits += 1
                    elif trail_mode == 'trail' and price >= trough_price * (1 + trail_distance):
                        balance += balance * RISK * price_pnl * LEVERAGE
                        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'TRAIL', 'sub': entry_type}
                        trades.append(trade)
                        short_trades.append(trade)
                        if entry_type == 'iso_short':
                            iso_short_trades.append(trade)
                        exited = True
                        exit_reason = 'TRAIL'
                        trail_exits += 1

            # 3. Signal exits
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                    balance += balance * RISK * price_pnl * LEVERAGE
                    reason = 'SMA' if price < row['sma20'] else 'Z0'
                    trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                             'type': 'win', 'reason': reason, 'sub': entry_type}
                    trades.append(trade)
                    short_trades.append(trade)
                    if entry_type == 'iso_short':
                        iso_short_trades.append(trade)
                    exited = True
                    exit_reason = reason

            if exited:
                reason_counts[exit_reason] = reason_counts.get(exit_reason, 0) + 1
                position = None
                entry_type = None
                cooldown = 3
                candles_held = 0

        if cooldown > 0:
            cooldown -= 1

        # ENTRY
        if position is None and cooldown == 0:
            market_ctx = {}
            if avg_z_series is not None and idx in avg_z_series.index:
                market_ctx['avg_z'] = avg_z_series.loc[idx]
            if avg_rsi_series is not None and idx in avg_rsi_series.index:
                market_ctx['avg_rsi'] = avg_rsi_series.loc[idx]
            if btc_z_series is not None and idx in btc_z_series.index:
                market_ctx['btc_z'] = btc_z_series.loc[idx]

            if market_mode == 'long':
                if long_entry_signal(row, long_strat):
                    position = 'long'
                    entry_price = price
                    peak_price = row['h']
                    entry_type = 'long'
                elif iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    position = 'short'
                    entry_price = price
                    trough_price = row['l']
                    entry_type = 'iso_short'
            elif market_mode == 'iso_short':
                if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    if b <= ISO_SHORT_BREADTH_MAX or ISO_SHORT_BREADTH_MAX >= 0.50:
                        position = 'short'
                        entry_price = price
                        trough_price = row['l']
                        entry_type = 'iso_short'
            elif market_mode == 'short':
                if short_entry_signal(row, short_strat):
                    position = 'short'
                    entry_price = price
                    trough_price = row['l']
                    entry_type = 'market_short'

        if balance > peak_balance:
            peak_balance = balance
        dd = (peak_balance - balance) / peak_balance * 100
        if dd > max_drawdown:
            max_drawdown = dd

    # Close open
    if position == 'long':
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        balance += balance * RISK * price_pnl * LEVERAGE
        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END'}
        trades.append(trade)
        long_trades.append(trade)
    elif position == 'short':
        price_pnl = (entry_price - df.iloc[-1]['c']) / entry_price
        balance += balance * RISK * price_pnl * LEVERAGE
        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END', 'sub': entry_type}
        trades.append(trade)
        short_trades.append(trade)
        if entry_type == 'iso_short':
            iso_short_trades.append(trade)

    if not trades:
        return None

    def calc_stats(tlist):
        if not tlist:
            return {'pf': 0, 'wr': 0, 'trades': 0, 'wins': 0}
        wins = [t for t in tlist if t['pnl_pct'] > 0]
        losses = [t for t in tlist if t['pnl_pct'] <= 0]
        tw = sum(t['pnl_pct'] for t in wins) if wins else 0
        tl = sum(t['pnl_pct'] for t in losses) if losses else 0
        return {
            'pf': abs(tw / tl) if tl != 0 else 0,
            'wr': len(wins) / len(tlist) * 100,
            'trades': len(tlist),
            'wins': len(wins),
        }

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'all': calc_stats(trades),
        'long': calc_stats(long_trades),
        'short': calc_stats(short_trades),
        'iso_short': calc_stats(iso_short_trades),
        'trail_exits': trail_exits,
        'reason_counts': reason_counts,
    }


def main():
    global ISO_SHORT_BREADTH_MAX

    print("=" * 90)
    print("RUN7.3 - COMBINED BACKTEST: CURRENT SL vs OPTIMIZED SL")
    print("=" * 90)

    # Load ISO short strategies
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat
            print(f"Loaded ISO short strategies for {len(OPTIMAL_ISO_SHORT_STRAT)} coins")
        if 'best_iso_params' in r61:
            bmaxes = [d.get('breadth_max', 0.20) for d in r61['best_iso_params'].values()]
            if bmaxes:
                from collections import Counter
                ISO_SHORT_BREADTH_MAX = Counter(bmaxes).most_common(1)[0][0]

    # Load optimized SL params from run7_1/run7_2
    r71_file = '/home/scamarena/ProjectCoin/run7_1_results.json'
    r72_file = '/home/scamarena/ProjectCoin/run7_2_results.json'

    optimized_params = None

    # Prefer run7_2 recommendation
    if os.path.exists(r72_file):
        with open(r72_file, 'r') as f:
            r72 = json.load(f)
        if 'universal_params' in r72 and r72['universal_params']:
            optimized_params = r72['universal_params']
            print(f"RUN7.2 recommendation: {r72.get('recommendation', '?')}")

    if os.path.exists(r71_file):
        with open(r71_file, 'r') as f:
            r71 = json.load(f)
        if optimized_params is None and 'best_overall' in r71 and r71['best_overall']:
            optimized_params = r71['best_overall']

    if optimized_params:
        print(f"Optimized SL: {optimized_params['trail_mode']} "
              f"sl={optimized_params['initial_sl']} act={optimized_params['trail_activation']} "
              f"dist={optimized_params['trail_distance']}")
    else:
        print("WARNING: No optimized params found. Using default: sl=0.007, trail, act=0.002, dist=0.002")
        optimized_params = {'trail_mode': 'trail', 'initial_sl': 0.007,
                            'trail_activation': 0.002, 'trail_distance': 0.002}

    # Load data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    breadth, avg_z, avg_rsi, btc_z = build_market_data(all_data)

    # === RUN BOTH MODES ===
    results_current = {}
    results_optimized = {}

    print("\nRunning backtests...")
    for i, coin in enumerate(COINS):
        if _shutdown:
            break
        if coin not in all_data:
            continue

        long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
        short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')
        iso_short_strat = OPTIMAL_ISO_SHORT_STRAT.get(coin)

        # Current: sl=0.5%, no trailing
        r = run_backtest(all_data[coin], long_strat, short_strat, iso_short_strat,
                         breadth, avg_z, avg_rsi, btc_z,
                         initial_sl=0.005, trail_mode='none')
        if r:
            results_current[coin] = r

        # Optimized
        r = run_backtest(all_data[coin], long_strat, short_strat, iso_short_strat,
                         breadth, avg_z, avg_rsi, btc_z,
                         initial_sl=optimized_params['initial_sl'],
                         trail_mode=optimized_params['trail_mode'],
                         trail_activation=optimized_params['trail_activation'],
                         trail_distance=optimized_params['trail_distance'])
        if r:
            results_optimized[coin] = r

        print(f"  [{i+1}/{len(COINS)}] {coin} done")

    # === COMPARISON TABLE ===
    print(f"\n{'='*90}")
    print("v7 (CURRENT SL) vs v8 (OPTIMIZED SL)")
    print(f"{'='*90}")

    def portfolio_stats(results):
        if not results:
            return {}
        wrs = [r['all']['wr'] for r in results.values() if r['all']['trades'] > 0]
        pfs = [r['all']['pf'] for r in results.values() if r['all']['trades'] > 0]
        pnls = [r['pnl'] for r in results.values()]
        dds = [r['max_dd'] for r in results.values()]
        total_trades = sum(r['all']['trades'] for r in results.values())
        return {
            'avg_wr': np.mean(wrs) if wrs else 0,
            'avg_pf': np.mean(pfs) if pfs else 0,
            'avg_pnl': np.mean(pnls) if pnls else 0,
            'avg_dd': np.mean(dds) if dds else 0,
            'total_trades': total_trades,
            'total_pnl': sum(r['pnl'] for r in results.values()),
        }

    s_cur = portfolio_stats(results_current)
    s_opt = portfolio_stats(results_optimized)

    print(f"\n{'Mode':<25} {'Avg WR':<10} {'Avg PF':<10} {'Trades':<10} {'Avg MaxDD':<12} {'Avg P&L':<12} {'Total P&L'}")
    print("-" * 95)
    if s_cur:
        print(f"{'v7 (sl=0.5%, none)':<25} {s_cur['avg_wr']:<10.1f}% {s_cur['avg_pf']:<10.2f} {s_cur['total_trades']:<10} "
              f"{s_cur['avg_dd']:<12.1f}% {s_cur['avg_pnl']:<12.1f}% {s_cur['total_pnl']:+.1f}%")
    if s_opt:
        mode_str = f"v8 ({optimized_params['trail_mode']})"
        print(f"{mode_str:<25} {s_opt['avg_wr']:<10.1f}% {s_opt['avg_pf']:<10.2f} {s_opt['total_trades']:<10} "
              f"{s_opt['avg_dd']:<12.1f}% {s_opt['avg_pnl']:<12.1f}% {s_opt['total_pnl']:+.1f}%")

    if s_cur and s_opt:
        print(f"\n  Delta:")
        print(f"    WR:     {s_opt['avg_wr']-s_cur['avg_wr']:+.1f}%")
        print(f"    PF:     {s_opt['avg_pf']-s_cur['avg_pf']:+.2f}")
        print(f"    P&L:    {s_opt['avg_pnl']-s_cur['avg_pnl']:+.1f}%")
        print(f"    MaxDD:  {s_opt['avg_dd']-s_cur['avg_dd']:+.1f}%")

    # === PER-COIN BREAKDOWN ===
    print(f"\n{'='*90}")
    print("PER-COIN BREAKDOWN (v7 current → v8 optimized)")
    print(f"{'='*90}")

    print(f"\n{'Coin':<8} {'v7 WR':<10} {'v8 WR':<10} {'v7 PF':<10} {'v8 PF':<10} "
          f"{'v7 P&L':<10} {'v8 P&L':<10} {'Trail#':<8} {'Better?'}")
    print("-" * 90)

    coins_better = 0
    coins_worse = 0
    for coin in COINS:
        if coin not in results_current or coin not in results_optimized:
            continue
        rc = results_current[coin]
        ro = results_optimized[coin]
        better = "YES" if ro['pnl'] > rc['pnl'] else "NO"
        if ro['pnl'] > rc['pnl']:
            coins_better += 1
        else:
            coins_worse += 1
        print(f"{coin:<8} {rc['all']['wr']:<10.1f} {ro['all']['wr']:<10.1f} "
              f"{rc['all']['pf']:<10.2f} {ro['all']['pf']:<10.2f} "
              f"{rc['pnl']:<10.1f} {ro['pnl']:<10.1f} {ro['trail_exits']:<8} {better}")

    print(f"\n  Better: {coins_better}  Worse: {coins_worse}")

    # === EXIT REASON DISTRIBUTION ===
    if results_optimized:
        print(f"\n{'='*90}")
        print("EXIT REASON DISTRIBUTION (v8 optimized)")
        print(f"{'='*90}")
        total_reasons = {}
        for r in results_optimized.values():
            for reason, count in r.get('reason_counts', {}).items():
                total_reasons[reason] = total_reasons.get(reason, 0) + count
        total = sum(total_reasons.values())
        for reason, count in sorted(total_reasons.items(), key=lambda x: -x[1]):
            print(f"  {reason:<8} {count:>5}  ({count/total*100:.1f}%)")

    # Save
    save_data = {
        'optimized_params': optimized_params,
        'portfolio_current': s_cur,
        'portfolio_optimized': s_opt,
        'coins_better': coins_better,
        'coins_worse': coins_worse,
        'per_coin_current': {c: {'pnl': r['pnl'], 'max_dd': r['max_dd'], 'all': r['all'],
                                  'long': r['long'], 'short': r['short']}
                             for c, r in results_current.items()},
        'per_coin_optimized': {c: {'pnl': r['pnl'], 'max_dd': r['max_dd'], 'all': r['all'],
                                    'long': r['long'], 'short': r['short'],
                                    'trail_exits': r['trail_exits'],
                                    'reason_counts': r.get('reason_counts', {})}
                               for c, r in results_optimized.items()},
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(save_data, f, indent=2)
    print(f"\nResults saved to {RESULTS_FILE}")


if __name__ == "__main__":
    main()
