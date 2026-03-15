#!/usr/bin/env python3
"""
RUN6.3 - Combined 3-Mode Backtest (Long + ISO Short + Market-Dump Short)
Full directional system with 3 modes:

  breadth <= 20%        -> LONG mode (long entries + ISO_SHORT entries)
  20% < breadth < 50%   -> ISO_SHORT only (was IDLE in v6)
  breadth >= 50%        -> SHORT mode (market-dump shorts, existing RUN5)

4-way comparison: long_only, long+market_short (v6), long+iso_short, combined_all (v7)
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

# Long strategies from RUN4.2
OPTIMAL_LONG_STRAT = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

# Market-dump short strategies from RUN5.2
OPTIMAL_SHORT_STRAT = {
    'DASH': 'short_mean_rev', 'UNI': 'short_adr_rev', 'NEAR': 'short_adr_rev',
    'ADA': 'short_bb_bounce', 'LTC': 'short_mean_rev', 'SHIB': 'short_vwap_rev',
    'LINK': 'short_bb_bounce', 'ETH': 'short_adr_rev', 'DOT': 'short_vwap_rev',
    'XRP': 'short_bb_bounce', 'ATOM': 'short_adr_rev', 'SOL': 'short_adr_rev',
    'DOGE': 'short_bb_bounce', 'XLM': 'short_mean_rev', 'AVAX': 'short_bb_bounce',
    'ALGO': 'short_adr_rev', 'BNB': 'short_vwap_rev', 'BTC': 'short_adr_rev',
}

# ISO short strategies — will be loaded from run6_1 results if available
OPTIMAL_ISO_SHORT_STRAT = {}

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.005
MIN_HOLD = 2

BREADTH_LONG_MAX = 0.20   # Long entries only when breadth <= this
BREADTH_SHORT_MIN = 0.50  # Market-dump short entries only when breadth >= this
ISO_SHORT_BREADTH_MAX = 0.20  # ISO shorts enter when breadth <= this (will be updated from run6_1)

# Default ISO short params
ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}


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
    """Market-dump short entry (breadth >= 50%)."""
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
    """ISO short entry: coin-specific overbought in calm market."""
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


def run_combined_backtest(df, long_strat, short_strat, iso_short_strat, breadth,
                          avg_z_series, avg_rsi_series, btc_z_series, mode='combined_all'):
    """
    Run a directional backtest.
    mode: 'long_only', 'long+market_short' (v6), 'long+iso_short', 'combined_all' (v7)
    """
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None

    balance = INITIAL_CAPITAL
    peak_balance = INITIAL_CAPITAL
    max_drawdown = 0
    position = None   # None, 'long', 'short'
    entry_price = 0
    trades = []
    long_trades = []
    short_trades = []
    iso_short_trades = []
    cooldown = 0
    candles_held = 0
    entry_type = None  # 'long', 'market_short', 'iso_short'

    time_in_long_mode = 0
    time_in_idle_mode = 0
    time_in_iso_short_mode = 0
    time_in_short_mode = 0

    for idx, row in df.iterrows():
        price = row['c']

        # Determine market mode (v7 3-mode)
        b = breadth.loc[idx] if idx in breadth.index else 0
        if b <= BREADTH_LONG_MAX:
            market_mode = 'long'       # Can do longs + ISO shorts
            time_in_long_mode += 1
        elif b >= BREADTH_SHORT_MIN:
            market_mode = 'short'      # Market-dump shorts only
            time_in_short_mode += 1
        else:
            market_mode = 'iso_short'  # ISO shorts only (was IDLE in v6)
            time_in_iso_short_mode += 1

        # === EXIT LOGIC (always active regardless of mode) ===
        if position == 'long':
            candles_held += 1
            price_pnl = (price - entry_price) / entry_price

            exited = False
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trade = {'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'dir': 'long', 'type': 'loss'}
                trades.append(trade)
                long_trades.append(trade)
                exited = True
            elif price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20'] or row['z'] > 0.5:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long', 'type': 'win'}
                    trades.append(trade)
                    long_trades.append(trade)
                    exited = True

            if exited:
                position = None
                entry_type = None
                cooldown = 3
                candles_held = 0

        elif position == 'short':
            candles_held += 1
            price_pnl = (entry_price - price) / entry_price

            exited = False
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trade = {'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'dir': 'short',
                         'type': 'loss', 'sub': entry_type}
                trades.append(trade)
                short_trades.append(trade)
                if entry_type == 'iso_short':
                    iso_short_trades.append(trade)
                exited = True
            elif price_pnl > 0 and candles_held >= MIN_HOLD:
                exit_cond = price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']
                if exit_cond:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                             'type': 'win', 'sub': entry_type}
                    trades.append(trade)
                    short_trades.append(trade)
                    if entry_type == 'iso_short':
                        iso_short_trades.append(trade)
                    exited = True

            if exited:
                position = None
                entry_type = None
                cooldown = 3
                candles_held = 0

        # === COOLDOWN ===
        if cooldown > 0:
            cooldown -= 1

        # === ENTRY LOGIC ===
        if position is None and cooldown == 0:
            # Build market context
            market_ctx = {}
            if avg_z_series is not None and idx in avg_z_series.index:
                market_ctx['avg_z'] = avg_z_series.loc[idx]
            if avg_rsi_series is not None and idx in avg_rsi_series.index:
                market_ctx['avg_rsi'] = avg_rsi_series.loc[idx]
            if btc_z_series is not None and idx in btc_z_series.index:
                market_ctx['btc_z'] = btc_z_series.loc[idx]

            if market_mode == 'long':
                # LONG mode: check long first, then ISO short
                if mode in ('long_only', 'long+market_short', 'long+iso_short', 'combined_all'):
                    if long_entry_signal(row, long_strat):
                        position = 'long'
                        entry_price = price
                        entry_type = 'long'

                if position is None and mode in ('long+iso_short', 'combined_all'):
                    if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                        position = 'short'
                        entry_price = price
                        entry_type = 'iso_short'

            elif market_mode == 'iso_short':
                # ISO_SHORT mode: only ISO shorts (was IDLE in v6)
                if mode in ('long+iso_short', 'combined_all'):
                    if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                        # Additional breadth check for ISO shorts in this zone
                        if b <= ISO_SHORT_BREADTH_MAX or ISO_SHORT_BREADTH_MAX >= 0.50:
                            position = 'short'
                            entry_price = price
                            entry_type = 'iso_short'

            elif market_mode == 'short':
                # SHORT mode: market-dump shorts only
                if mode in ('long+market_short', 'combined_all'):
                    if short_entry_signal(row, short_strat):
                        position = 'short'
                        entry_price = price
                        entry_type = 'market_short'

        # Track equity and drawdown
        if balance > peak_balance:
            peak_balance = balance
        dd = (peak_balance - balance) / peak_balance * 100
        if dd > max_drawdown:
            max_drawdown = dd

    # Close any open position at end
    if position == 'long':
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                 'type': 'win' if price_pnl > 0 else 'loss'}
        trades.append(trade)
        long_trades.append(trade)
    elif position == 'short':
        price_pnl = (entry_price - df.iloc[-1]['c']) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                 'type': 'win' if price_pnl > 0 else 'loss', 'sub': entry_type}
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

    total_candles = time_in_long_mode + time_in_idle_mode + time_in_iso_short_mode + time_in_short_mode

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'all': calc_stats(trades),
        'long': calc_stats(long_trades),
        'short': calc_stats(short_trades),
        'iso_short': calc_stats(iso_short_trades),
        'time_long_pct': time_in_long_mode / total_candles * 100 if total_candles > 0 else 0,
        'time_idle_pct': time_in_idle_mode / total_candles * 100 if total_candles > 0 else 0,
        'time_iso_short_pct': time_in_iso_short_mode / total_candles * 100 if total_candles > 0 else 0,
        'time_short_pct': time_in_short_mode / total_candles * 100 if total_candles > 0 else 0,
    }


def main():
    print("=" * 90)
    print("RUN6.3 - COMBINED 3-MODE BACKTEST (v7)")
    print("=" * 90)
    print(f"Long mode:      breadth <= {BREADTH_LONG_MAX*100:.0f}%  (longs + ISO shorts)")
    print(f"ISO_SHORT mode: {BREADTH_LONG_MAX*100:.0f}% < breadth < {BREADTH_SHORT_MIN*100:.0f}%  (ISO shorts only, was IDLE in v6)")
    print(f"Short mode:     breadth >= {BREADTH_SHORT_MIN*100:.0f}%  (market-dump shorts)")
    print("=" * 90)

    # Load RUN5.2 results for market-dump short strategies
    r52_file = '/home/scamarena/ProjectCoin/run5_2_results.json'
    if os.path.exists(r52_file):
        with open(r52_file, 'r') as f:
            r52 = json.load(f)
        if 'optimal_short_strat' in r52:
            for coin, strat in r52['optimal_short_strat'].items():
                OPTIMAL_SHORT_STRAT[coin] = strat
            print(f"Loaded RUN5.2 market-dump short strategies for {len(r52['optimal_short_strat'])} coins")

    # Load RUN6.1 results for ISO short strategies
    global ISO_SHORT_BREADTH_MAX, ISO_SHORT_PARAMS
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat
            print(f"Loaded RUN6.1 ISO short strategies for {len(r61['optimal_iso_short_strat'])} coins")
        # Update breadth max from most common
        if 'best_iso_params' in r61:
            bmaxes = [d.get('breadth_max', 0.20) for d in r61['best_iso_params'].values()]
            if bmaxes:
                from collections import Counter
                ISO_SHORT_BREADTH_MAX = Counter(bmaxes).most_common(1)[0][0]
                print(f"ISO_SHORT_BREADTH_MAX set to {ISO_SHORT_BREADTH_MAX*100:.0f}%")
    else:
        print("WARNING: run6_1_results.json not found, ISO shorts will be empty")

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build market data
    breadth, avg_z, avg_rsi, btc_z = build_market_data(all_data)
    print(f"Breadth: avg={breadth.mean():.1%}, <=20%: {(breadth <= 0.2).mean():.1%}, "
          f">=50%: {(breadth >= 0.5).mean():.1%}")

    # === RUN FOUR MODES ===
    modes = ['long_only', 'long+market_short', 'long+iso_short', 'combined_all']
    results = {m: {} for m in modes}

    for coin, df in all_data.items():
        long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
        short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')
        iso_short_strat = OPTIMAL_ISO_SHORT_STRAT.get(coin)

        for m in modes:
            r = run_combined_backtest(
                df, long_strat, short_strat, iso_short_strat, breadth,
                avg_z, avg_rsi, btc_z, mode=m
            )
            if r:
                results[m][coin] = r

    # === COMPARISON TABLE ===
    print(f"\n{'='*90}")
    print("4-WAY COMPARISON")
    print(f"{'='*90}")

    print(f"\n{'Mode':<22} {'Avg WR':<10} {'Avg PF':<10} {'Trades':<10} {'Avg MaxDD':<12} {'Avg P&L'}")
    print("-" * 75)

    summary = {}
    for m in modes:
        if not results[m]:
            continue
        coins_r = results[m]
        wrs = [r['all']['wr'] for r in coins_r.values() if r['all']['trades'] > 0]
        pfs = [r['all']['pf'] for r in coins_r.values() if r['all']['trades'] > 0]
        total_trades = sum(r['all']['trades'] for r in coins_r.values())
        dds = [r['max_dd'] for r in coins_r.values()]
        pnls = [r['pnl'] for r in coins_r.values()]

        avg_wr = np.mean(wrs) if wrs else 0
        avg_pf = np.mean(pfs) if pfs else 0
        avg_dd = np.mean(dds) if dds else 0
        avg_pnl = np.mean(pnls) if pnls else 0

        summary[m] = {'wr': avg_wr, 'pf': avg_pf, 'trades': total_trades,
                       'dd': avg_dd, 'pnl': avg_pnl}

        print(f"{m:<22} {avg_wr:<10.1f}% {avg_pf:<10.2f} {total_trades:<10} "
              f"{avg_dd:<12.1f}% {avg_pnl:+.1f}%")

    # === TIME ALLOCATION (combined_all mode) ===
    if results['combined_all']:
        avg_long_t = np.mean([r['time_long_pct'] for r in results['combined_all'].values()])
        avg_iso_t = np.mean([r['time_iso_short_pct'] for r in results['combined_all'].values()])
        avg_short_t = np.mean([r['time_short_pct'] for r in results['combined_all'].values()])
        avg_idle_t = np.mean([r['time_idle_pct'] for r in results['combined_all'].values()])
        print(f"\n  Time allocation (v7 combined_all):")
        print(f"    LONG={avg_long_t:.1f}% | ISO_SHORT={avg_iso_t:.1f}% | SHORT={avg_short_t:.1f}% | IDLE={avg_idle_t:.1f}%")

    # v6 time allocation for comparison
    if results['long+market_short']:
        v6_idle = np.mean([r['time_idle_pct'] + r['time_iso_short_pct']
                           for r in results['long+market_short'].values()])
        print(f"    v6 idle time: {v6_idle:.1f}%")

    # === PER-COIN DETAILS (combined_all) ===
    print(f"\n{'='*90}")
    print("PER-COIN DETAILS (combined_all / v7)")
    print(f"{'='*90}")

    print(f"\n{'Coin':<8} {'Long':<16} {'MktShort':<16} {'IsoShort':<18} {'WR':<8} {'PF':<8} "
          f"{'L#':<5} {'S#':<5} {'IS#':<5} {'MaxDD':<8} {'P&L'}")
    print("-" * 120)

    for coin in COINS:
        if coin not in results['combined_all']:
            continue
        r = results['combined_all'][coin]
        ls = OPTIMAL_LONG_STRAT.get(coin, '?')
        ss = OPTIMAL_SHORT_STRAT.get(coin, '?')
        iso = OPTIMAL_ISO_SHORT_STRAT.get(coin, 'none')
        print(f"{coin:<8} {ls:<16} {ss:<16} {iso:<18} {r['all']['wr']:<8.1f}% {r['all']['pf']:<8.2f} "
              f"{r['long']['trades']:<5} {r['short']['trades'] - r['iso_short']['trades']:<5} "
              f"{r['iso_short']['trades']:<5} {r['max_dd']:<8.1f}% {r['pnl']:+.1f}%")

    # === IMPROVEMENT ANALYSIS ===
    print(f"\n{'='*90}")
    print("IMPROVEMENT ANALYSIS")
    print(f"{'='*90}")

    comparisons = [
        ('long_only', 'long+market_short', 'v5 → v6 (add market-dump shorts)'),
        ('long+market_short', 'combined_all', 'v6 → v7 (add ISO shorts)'),
        ('long_only', 'combined_all', 'v5 → v7 (full improvement)'),
    ]

    for from_m, to_m, label in comparisons:
        if from_m in summary and to_m in summary:
            f = summary[from_m]
            t = summary[to_m]
            print(f"\n  {label}:")
            print(f"    WR:     {f['wr']:.1f}% → {t['wr']:.1f}%  ({t['wr']-f['wr']:+.1f}%)")
            print(f"    PF:     {f['pf']:.2f} → {t['pf']:.2f}  ({t['pf']-f['pf']:+.2f})")
            print(f"    Trades: {f['trades']} → {t['trades']}  ({t['trades']-f['trades']:+})")
            print(f"    MaxDD:  {f['dd']:.1f}% → {t['dd']:.1f}%  ({t['dd']-f['dd']:+.1f}%)")
            print(f"    P&L:    {f['pnl']:+.1f}% → {t['pnl']:+.1f}%  ({t['pnl']-f['pnl']:+.1f}%)")

    # Idle time comparison
    if results['long+market_short'] and results['combined_all']:
        v6_idle = np.mean([r['time_idle_pct'] + r['time_iso_short_pct']
                           for r in results['long+market_short'].values()])
        v7_idle = np.mean([r['time_idle_pct'] for r in results['combined_all'].values()])
        print(f"\n  Idle time: v6={v6_idle:.1f}% → v7={v7_idle:.1f}%  ({v7_idle-v6_idle:+.1f}%)")

    # === ISO SHORT CONTRIBUTION ===
    if results['combined_all']:
        iso_trades_total = sum(r['iso_short']['trades'] for r in results['combined_all'].values())
        iso_wins = sum(r['iso_short']['wins'] for r in results['combined_all'].values())
        iso_wr = iso_wins / iso_trades_total * 100 if iso_trades_total > 0 else 0
        print(f"\n  ISO short contribution: {iso_trades_total} trades, {iso_wr:.1f}% WR")

    # Save results
    save_data = {
        'summary': summary,
        'optimal_long_strat': OPTIMAL_LONG_STRAT,
        'optimal_short_strat': OPTIMAL_SHORT_STRAT,
        'optimal_iso_short_strat': OPTIMAL_ISO_SHORT_STRAT,
        'breadth_long_max': BREADTH_LONG_MAX,
        'breadth_short_min': BREADTH_SHORT_MIN,
        'iso_short_breadth_max': ISO_SHORT_BREADTH_MAX,
        'per_coin': {},
    }
    for coin in COINS:
        if coin in results['combined_all']:
            r = results['combined_all'][coin]
            save_data['per_coin'][coin] = {
                'pnl': r['pnl'], 'max_dd': r['max_dd'],
                'all': r['all'], 'long': r['long'],
                'short': r['short'], 'iso_short': r['iso_short'],
            }

    with open('/home/scamarena/ProjectCoin/run6_3_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run6_3_results.json")


if __name__ == "__main__":
    main()
