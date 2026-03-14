#!/usr/bin/env python3
"""
RUN5.3 - Combined Long+Short Backtest
Full directional system: long when breadth low, short when breadth high, idle in between.

Directional mode:
  breadth <= 0.20  → LONG mode (check long entries)
  0.20 < breadth < 0.50  → IDLE (no new entries)
  breadth >= 0.50  → SHORT mode (check short entries)
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

# Short strategies from RUN5.2 results (best per coin)
OPTIMAL_SHORT_STRAT = {
    'DASH': 'short_mean_rev', 'UNI': 'short_adr_rev', 'NEAR': 'short_adr_rev',
    'ADA': 'short_bb_bounce', 'LTC': 'short_mean_rev', 'SHIB': 'short_vwap_rev',
    'LINK': 'short_bb_bounce', 'ETH': 'short_adr_rev', 'DOT': 'short_vwap_rev',
    'XRP': 'short_bb_bounce', 'ATOM': 'short_adr_rev', 'SOL': 'short_adr_rev',
    'DOGE': 'short_bb_bounce', 'XLM': 'short_mean_rev', 'AVAX': 'short_bb_bounce',
    'ALGO': 'short_adr_rev', 'BNB': 'short_vwap_rev', 'BTC': 'short_adr_rev',
}

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.005
MIN_HOLD = 2

BREADTH_LONG_MAX = 0.20   # Long entries only when breadth <= this
BREADTH_SHORT_MIN = 0.50  # Short entries only when breadth >= this


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
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['adr_lo'] = df['l'].rolling(24).min()
    df['adr_hi'] = df['h'].rolling(24).max()
    return df


def build_market_breadth(all_data):
    z_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        z_frames[coin] = df_ind['z']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    return breadth


def long_entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    # Guard: don't enter long if already above SMA20 or z > 0.5
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
    # Guard: don't enter short if already below SMA20 or z < -0.5
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


def run_combined_backtest(df, long_strat, short_strat, breadth, mode='combined'):
    """
    Run a directional backtest.
    mode: 'long_only', 'short_only', 'combined'
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
    cooldown = 0
    candles_held = 0
    equity_curve = []

    time_in_long_mode = 0
    time_in_idle_mode = 0
    time_in_short_mode = 0

    for idx, row in df.iterrows():
        price = row['c']

        # Determine market mode
        b = breadth.loc[idx] if idx in breadth.index else 0
        if b <= BREADTH_LONG_MAX:
            market_mode = 'long'
            time_in_long_mode += 1
        elif b >= BREADTH_SHORT_MIN:
            market_mode = 'short'
            time_in_short_mode += 1
        else:
            market_mode = 'idle'
            time_in_idle_mode += 1

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
                cooldown = 3
                candles_held = 0

        elif position == 'short':
            candles_held += 1
            price_pnl = (entry_price - price) / entry_price  # Short PnL

            exited = False
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trade = {'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'dir': 'short', 'type': 'loss'}
                trades.append(trade)
                short_trades.append(trade)
                exited = True
            elif price_pnl > 0 and candles_held >= MIN_HOLD:
                if price < row['sma20'] or row['z'] < -0.5:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trade = {'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short', 'type': 'win'}
                    trades.append(trade)
                    short_trades.append(trade)
                    exited = True

            if exited:
                position = None
                cooldown = 3
                candles_held = 0

        # === COOLDOWN ===
        if cooldown > 0:
            cooldown -= 1

        # === ENTRY LOGIC ===
        if position is None and cooldown == 0:
            if mode in ('long_only', 'combined') and market_mode == 'long':
                if long_entry_signal(row, long_strat):
                    position = 'long'
                    entry_price = price

            if position is None and mode in ('short_only', 'combined') and market_mode == 'short':
                if short_entry_signal(row, short_strat):
                    position = 'short'
                    entry_price = price

        # Track equity and drawdown
        equity_curve.append(balance)
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
                 'type': 'win' if price_pnl > 0 else 'loss'}
        trades.append(trade)
        short_trades.append(trade)

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

    total_candles = time_in_long_mode + time_in_idle_mode + time_in_short_mode

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'all': calc_stats(trades),
        'long': calc_stats(long_trades),
        'short': calc_stats(short_trades),
        'time_long_pct': time_in_long_mode / total_candles * 100 if total_candles > 0 else 0,
        'time_idle_pct': time_in_idle_mode / total_candles * 100 if total_candles > 0 else 0,
        'time_short_pct': time_in_short_mode / total_candles * 100 if total_candles > 0 else 0,
    }


def main():
    print("=" * 90)
    print("RUN5.3 - COMBINED LONG+SHORT BACKTEST")
    print("=" * 90)
    print(f"Long mode:  breadth <= {BREADTH_LONG_MAX*100:.0f}%")
    print(f"Idle mode:  {BREADTH_LONG_MAX*100:.0f}% < breadth < {BREADTH_SHORT_MIN*100:.0f}%")
    print(f"Short mode: breadth >= {BREADTH_SHORT_MIN*100:.0f}%")
    print("=" * 90)

    # Try to load RUN5.2 results for optimal short strategies
    r52_file = '/home/scamarena/ProjectCoin/run5_2_results.json'
    if os.path.exists(r52_file):
        with open(r52_file, 'r') as f:
            r52 = json.load(f)
        if 'optimal_short_strat' in r52:
            for coin, strat in r52['optimal_short_strat'].items():
                OPTIMAL_SHORT_STRAT[coin] = strat
            print(f"Loaded RUN5.2 optimal short strategies for {len(r52['optimal_short_strat'])} coins")

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build market breadth
    breadth = build_market_breadth(all_data)
    print(f"Breadth: avg={breadth.mean():.1%}, >=50%: {(breadth >= 0.5).mean():.1%}")

    # === RUN THREE MODES ===
    modes = ['long_only', 'short_only', 'combined']
    results = {m: {} for m in modes}

    for coin, df in all_data.items():
        long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
        short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')

        for m in modes:
            r = run_combined_backtest(df, long_strat, short_strat, breadth, mode=m)
            if r:
                results[m][coin] = r

    # === COMPARISON TABLE ===
    print(f"\n{'='*90}")
    print("COMPARISON: LONG-ONLY vs SHORT-ONLY vs COMBINED")
    print(f"{'='*90}")

    print(f"\n{'Mode':<14} {'Avg WR':<10} {'Avg PF':<10} {'Trades':<10} {'Avg MaxDD':<12} {'Avg P&L'}")
    print("-" * 65)

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

        print(f"{m:<14} {avg_wr:<10.1f}% {avg_pf:<10.2f} {total_trades:<10} "
              f"{avg_dd:<12.1f}% {avg_pnl:+.1f}%")

    # === TIME ALLOCATION (combined mode) ===
    if results['combined']:
        avg_long_t = np.mean([r['time_long_pct'] for r in results['combined'].values()])
        avg_idle_t = np.mean([r['time_idle_pct'] for r in results['combined'].values()])
        avg_short_t = np.mean([r['time_short_pct'] for r in results['combined'].values()])
        print(f"\n  Time allocation (combined): LONG={avg_long_t:.1f}% | IDLE={avg_idle_t:.1f}% | SHORT={avg_short_t:.1f}%")

    # === PER-COIN DETAILS (combined) ===
    print(f"\n{'='*90}")
    print("PER-COIN DETAILS (combined mode)")
    print(f"{'='*90}")

    print(f"\n{'Coin':<8} {'Long Strat':<16} {'Short Strat':<18} {'WR':<8} {'PF':<8} "
          f"{'L trades':<10} {'S trades':<10} {'MaxDD':<8} {'P&L'}")
    print("-" * 100)

    for coin in COINS:
        if coin not in results['combined']:
            continue
        r = results['combined'][coin]
        ls = OPTIMAL_LONG_STRAT.get(coin, '?')
        ss = OPTIMAL_SHORT_STRAT.get(coin, '?')
        print(f"{coin:<8} {ls:<16} {ss:<18} {r['all']['wr']:<8.1f}% {r['all']['pf']:<8.2f} "
              f"{r['long']['trades']:<10} {r['short']['trades']:<10} "
              f"{r['max_dd']:<8.1f}% {r['pnl']:+.1f}%")

    # === IMPROVEMENT ANALYSIS ===
    if 'long_only' in summary and 'combined' in summary:
        print(f"\n{'='*90}")
        print("IMPROVEMENT ANALYSIS (combined vs long-only)")
        print(f"{'='*90}")

        lo = summary['long_only']
        co = summary['combined']

        print(f"  WR:     {lo['wr']:.1f}% → {co['wr']:.1f}%  ({co['wr']-lo['wr']:+.1f}%)")
        print(f"  PF:     {lo['pf']:.2f} → {co['pf']:.2f}  ({co['pf']-lo['pf']:+.2f})")
        print(f"  Trades: {lo['trades']} → {co['trades']}  ({co['trades']-lo['trades']:+})")
        print(f"  MaxDD:  {lo['dd']:.1f}% → {co['dd']:.1f}%  ({co['dd']-lo['dd']:+.1f}%)")
        print(f"  P&L:    {lo['pnl']:+.1f}% → {co['pnl']:+.1f}%  ({co['pnl']-lo['pnl']:+.1f}%)")

        # Idle time comparison
        if results['long_only']:
            lo_idle = 100 - np.mean([r['time_long_pct'] for r in results['long_only'].values()])
            co_idle = np.mean([r['time_idle_pct'] for r in results['combined'].values()])
            print(f"\n  Idle time: {lo_idle:.1f}% → {co_idle:.1f}%  ({co_idle-lo_idle:+.1f}%)")

    # Save results
    save_data = {
        'summary': summary,
        'optimal_long_strat': OPTIMAL_LONG_STRAT,
        'optimal_short_strat': OPTIMAL_SHORT_STRAT,
        'breadth_long_max': BREADTH_LONG_MAX,
        'breadth_short_min': BREADTH_SHORT_MIN,
        'per_coin': {},
    }
    for coin in COINS:
        if coin in results['combined']:
            r = results['combined'][coin]
            save_data['per_coin'][coin] = {
                'pnl': r['pnl'], 'max_dd': r['max_dd'],
                'all': r['all'], 'long': r['long'], 'short': r['short'],
            }

    with open('/home/scamarena/ProjectCoin/run5_3_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run5_3_results.json")


if __name__ == "__main__":
    main()
