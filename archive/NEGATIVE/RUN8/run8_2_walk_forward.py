#!/usr/bin/env python3
"""
RUN8.2 - Walk-Forward Validation of Take Profit

3 windows (train 2mo, test 1mo):
  W1: Oct 15-Dec 14 -> Dec 15-Jan 14
  W2: Nov 15-Jan 14 -> Jan 15-Feb 14
  W3: Dec 15-Feb 14 -> Feb 15-Mar 10

Train: find best TP params per coin from 13 combos.
Test: apply OOS. Compare universal vs per-coin vs baseline (no TP).
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run8_2_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run8_2_results.json'

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
STOP_LOSS = 0.003

BREADTH_LONG_MAX = 0.20
BREADTH_SHORT_MIN = 0.50
ISO_SHORT_BREADTH_MAX = 0.20

ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}

TP_TARGETS = [0.003, 0.005, 0.007, 0.010, 0.015, 0.020]

COMBOS = []
COMBOS.append(('none', 0.0))
for tp in TP_TARGETS:
    COMBOS.append(('tp_immediate', tp))
for tp in TP_TARGETS:
    COMBOS.append(('tp_after_hold', tp))

WINDOWS = [
    {'name': 'W1', 'train_start': '2025-10-15', 'train_end': '2025-12-14',
     'test_start': '2025-12-15', 'test_end': '2026-01-14'},
    {'name': 'W2', 'train_start': '2025-11-15', 'train_end': '2026-01-14',
     'test_start': '2026-01-15', 'test_end': '2026-02-14'},
    {'name': 'W3', 'train_start': '2025-12-15', 'train_end': '2026-02-14',
     'test_start': '2026-02-15', 'test_end': '2026-03-10'},
]

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
                 tp_mode='none', tp_target=0.0):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None

    balance = INITIAL_CAPITAL
    peak_balance = INITIAL_CAPITAL
    max_drawdown = 0
    position = None
    entry_price = 0
    trades = []
    cooldown = 0
    candles_held = 0
    entry_type = None
    tp_exits = 0

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
            price_pnl = (price - entry_price) / entry_price
            exited = False

            # 1. Stop Loss
            if price_pnl <= -STOP_LOSS:
                balance -= balance * RISK * STOP_LOSS * LEVERAGE
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'type': 'loss', 'reason': 'SL'})
                exited = True

            # 2. Take Profit
            if not exited and tp_mode != 'none' and tp_target > 0:
                tp_can_fire = False
                if tp_mode == 'tp_immediate':
                    tp_can_fire = True
                elif tp_mode == 'tp_after_hold' and candles_held >= MIN_HOLD:
                    tp_can_fire = True
                if tp_can_fire and price_pnl >= tp_target:
                    balance += balance * RISK * tp_target * LEVERAGE
                    trades.append({'pnl_pct': tp_target * LEVERAGE * 100, 'type': 'win', 'reason': 'TP'})
                    exited = True
                    tp_exits += 1

            # 3. Signal exits
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20'] or row['z'] > 0.5:
                    balance += balance * RISK * price_pnl * LEVERAGE
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win',
                                   'reason': 'SMA' if row['c'] > row['sma20'] else 'Z0'})
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

            # 1. Stop Loss
            if price_pnl <= -STOP_LOSS:
                balance -= balance * RISK * STOP_LOSS * LEVERAGE
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'type': 'loss', 'reason': 'SL'})
                exited = True

            # 2. Take Profit
            if not exited and tp_mode != 'none' and tp_target > 0:
                tp_can_fire = False
                if tp_mode == 'tp_immediate':
                    tp_can_fire = True
                elif tp_mode == 'tp_after_hold' and candles_held >= MIN_HOLD:
                    tp_can_fire = True
                if tp_can_fire and price_pnl >= tp_target:
                    balance += balance * RISK * tp_target * LEVERAGE
                    trades.append({'pnl_pct': tp_target * LEVERAGE * 100, 'type': 'win', 'reason': 'TP'})
                    exited = True
                    tp_exits += 1

            # 3. Signal exits
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                    balance += balance * RISK * price_pnl * LEVERAGE
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win',
                                   'reason': 'SMA' if price < row['sma20'] else 'Z0'})
                    exited = True

            if exited:
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
                    entry_type = 'long'
                elif iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    position = 'short'
                    entry_price = price
                    entry_type = 'iso_short'
            elif market_mode == 'iso_short':
                if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    if b <= ISO_SHORT_BREADTH_MAX or ISO_SHORT_BREADTH_MAX >= 0.50:
                        position = 'short'
                        entry_price = price
                        entry_type = 'iso_short'
            elif market_mode == 'short':
                if short_entry_signal(row, short_strat):
                    position = 'short'
                    entry_price = price
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
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END'})
    elif position == 'short':
        price_pnl = (entry_price - df.iloc[-1]['c']) / entry_price
        balance += balance * RISK * price_pnl * LEVERAGE
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END'})

    if not trades:
        return None

    wins = [t for t in trades if t['pnl_pct'] > 0]
    losses = [t for t in trades if t['pnl_pct'] <= 0]
    tw = sum(t['pnl_pct'] for t in wins) if wins else 0
    tl = sum(t['pnl_pct'] for t in losses) if losses else 0

    return {
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'trades': len(trades),
        'wins': len(wins),
        'wr': len(wins) / len(trades) * 100 if trades else 0,
        'pf': abs(tw / tl) if tl != 0 else 0,
        'tp_exits': tp_exits,
    }


def save_checkpoint(data):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(data, f)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return None


def main():
    global ISO_SHORT_BREADTH_MAX

    print("=" * 90)
    print("RUN8.2 - WALK-FORWARD VALIDATION OF TAKE PROFIT")
    print("=" * 90)
    print(f"Train: 2 months | Test: 1 month | 3 windows")
    print(f"TP combos: {len(COMBOS)} (none + tp_immediate + tp_after_hold)")
    print(f"Hardcoded: SL=0.3%, trail_mode=none")
    print("=" * 90)

    # Load RUN6.1
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat

    # Load RUN8.1 results for universal params
    r81_file = '/home/scamarena/ProjectCoin/run8_1_results.json'
    universal_params = None
    if os.path.exists(r81_file):
        with open(r81_file, 'r') as f:
            r81 = json.load(f)
        if 'best_overall' in r81 and r81['best_overall']:
            universal_params = r81['best_overall']
            tp_str = f"tp={universal_params['tp_target']*100:.1f}%" if universal_params['tp_mode'] != 'none' else "no TP"
            print(f"Loaded universal params from RUN8.1: {universal_params['tp_mode']} {tp_str}")
    else:
        print("WARNING: run8_1_results.json not found, will search all combos in train")

    # Load data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    breadth, avg_z, avg_rsi, btc_z = build_market_data(all_data)

    # Load checkpoint
    checkpoint = load_checkpoint()
    all_results = checkpoint.get('all_results', {}) if checkpoint else {}
    done_keys = set(checkpoint.get('done_keys', [])) if checkpoint else set()

    total_tasks = len(COINS) * len(WINDOWS)
    done_count = len(done_keys)
    print(f"Progress: {done_count}/{total_tasks}")

    start_time = _time.time()
    tasks_this_run = 0

    for coin in COINS:
        if coin not in all_data:
            continue
        df = all_data[coin]

        if coin not in all_results:
            all_results[coin] = []

        for w in WINDOWS:
            task_key = f"{coin}_{w['name']}"
            if task_key in done_keys:
                continue

            if _shutdown:
                save_checkpoint({'all_results': all_results, 'done_keys': list(done_keys)})
                print(f"Saved checkpoint at {len(done_keys)}/{total_tasks}")
                sys.exit(0)

            train_df = df[(df.index >= w['train_start']) & (df.index < w['train_end'])]
            test_df = df[(df.index >= w['test_start']) & (df.index <= w['test_end'])]
            train_breadth = breadth[(breadth.index >= w['train_start']) & (breadth.index < w['train_end'])]
            test_breadth = breadth[(breadth.index >= w['test_start']) & (breadth.index <= w['test_end'])]
            train_avg_z = avg_z[(avg_z.index >= w['train_start']) & (avg_z.index < w['train_end'])]
            test_avg_z = avg_z[(avg_z.index >= w['test_start']) & (avg_z.index <= w['test_end'])]
            train_avg_rsi = avg_rsi[(avg_rsi.index >= w['train_start']) & (avg_rsi.index < w['train_end'])]
            test_avg_rsi = avg_rsi[(avg_rsi.index >= w['test_start']) & (avg_rsi.index <= w['test_end'])]
            train_btc_z = btc_z[(btc_z.index >= w['train_start']) & (btc_z.index < w['train_end'])] if btc_z is not None else None
            test_btc_z = btc_z[(btc_z.index >= w['test_start']) & (btc_z.index <= w['test_end'])] if btc_z is not None else None

            if len(train_df) < 100 or len(test_df) < 50:
                done_keys.add(task_key)
                continue

            long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
            short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')
            iso_short_strat = OPTIMAL_ISO_SHORT_STRAT.get(coin)

            # === BASELINE: no TP ===
            test_base = run_backtest(
                test_df, long_strat, short_strat, iso_short_strat,
                test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                tp_mode='none')

            # === TRAIN: find best TP combo ===
            best_train_score = -999
            best_train_combo = None
            best_train_result = None

            for tp_mode, tp_target in COMBOS:
                r = run_backtest(
                    train_df, long_strat, short_strat, iso_short_strat,
                    train_breadth, train_avg_z, train_avg_rsi, train_btc_z,
                    tp_mode=tp_mode, tp_target=tp_target)
                if r and r['trades'] >= 3:
                    score = r['pf'] * (r['wr'] / 100) ** 0.5
                    if score > best_train_score:
                        best_train_score = score
                        best_train_combo = (tp_mode, tp_target)
                        best_train_result = r

            # === TEST: per-coin best ===
            test_percoin = None
            if best_train_combo:
                tp_mode, tp_target = best_train_combo
                test_percoin = run_backtest(
                    test_df, long_strat, short_strat, iso_short_strat,
                    test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                    tp_mode=tp_mode, tp_target=tp_target)

            # === TEST: universal params ===
            test_universal = None
            if universal_params:
                test_universal = run_backtest(
                    test_df, long_strat, short_strat, iso_short_strat,
                    test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                    tp_mode=universal_params['tp_mode'],
                    tp_target=universal_params['tp_target'])

            window_result = {
                'window': w['name'],
                'train_best_combo': list(best_train_combo) if best_train_combo else None,
                'train_best_pf': best_train_result['pf'] if best_train_result else 0,
                'test_base_pf': test_base['pf'] if test_base else 0,
                'test_base_wr': test_base['wr'] if test_base else 0,
                'test_base_pnl': test_base['pnl'] if test_base else 0,
                'test_base_trades': test_base['trades'] if test_base else 0,
                'test_percoin_pf': test_percoin['pf'] if test_percoin else 0,
                'test_percoin_wr': test_percoin['wr'] if test_percoin else 0,
                'test_percoin_pnl': test_percoin['pnl'] if test_percoin else 0,
                'test_percoin_trades': test_percoin['trades'] if test_percoin else 0,
                'test_universal_pf': test_universal['pf'] if test_universal else 0,
                'test_universal_wr': test_universal['wr'] if test_universal else 0,
                'test_universal_pnl': test_universal['pnl'] if test_universal else 0,
                'test_universal_trades': test_universal['trades'] if test_universal else 0,
            }
            all_results[coin].append(window_result)
            done_keys.add(task_key)
            done_count += 1
            tasks_this_run += 1

            elapsed = _time.time() - start_time
            rate = tasks_this_run / elapsed if elapsed > 0 else 0
            eta_min = ((total_tasks - done_count) / rate / 60) if rate > 0 else 0
            print(f"  [{done_count}/{total_tasks}] {coin} {w['name']} "
                  f"| {rate:.2f}/s ETA:{eta_min:.1f}m")

        # Checkpoint after each coin
        save_checkpoint({'all_results': all_results, 'done_keys': list(done_keys)})

    # === PRINT RESULTS ===
    print(f"\n{'='*90}")
    print("WALK-FORWARD RESULTS BY COIN")
    print(f"{'='*90}")

    for coin, wins in all_results.items():
        if not wins:
            continue
        print(f"\n{coin}")
        print(f"  {'Win':<4} {'Base PF':<10} {'Base WR':<10} {'PerCoin PF':<12} {'PerCoin WR':<12} "
              f"{'Univ PF':<10} {'Univ WR':<10} {'Test#'}")
        print(f"  {'-'*95}")
        for w in wins:
            low_conf = " *" if w['test_base_trades'] < 3 else ""
            print(f"  {w['window']:<4} {w['test_base_pf']:<10.2f} {w['test_base_wr']:<10.1f} "
                  f"{w['test_percoin_pf']:<12.2f} {w['test_percoin_wr']:<12.1f} "
                  f"{w['test_universal_pf']:<10.2f} {w['test_universal_wr']:<10.1f} "
                  f"{w['test_base_trades']}{low_conf}")

    # === DEGRADATION ANALYSIS ===
    print(f"\n{'='*90}")
    print("DEGRADATION ANALYSIS")
    print(f"{'='*90}")

    train_pfs_pc = []
    test_pfs_pc = []
    test_pfs_univ = []
    test_pfs_base = []

    for coin, wins in all_results.items():
        for w in wins:
            if w['train_best_pf'] > 0:
                train_pfs_pc.append(w['train_best_pf'])
                test_pfs_pc.append(w['test_percoin_pf'])
            test_pfs_univ.append(w['test_universal_pf'])
            test_pfs_base.append(w['test_base_pf'])

    avg_train_pc = np.mean(train_pfs_pc) if train_pfs_pc else 0
    avg_test_pc = np.mean(test_pfs_pc) if test_pfs_pc else 0
    avg_test_univ = np.mean(test_pfs_univ) if test_pfs_univ else 0
    avg_test_base = np.mean(test_pfs_base) if test_pfs_base else 0

    deg_pc = (1 - avg_test_pc / avg_train_pc) * 100 if avg_train_pc > 0 else 0
    print(f"\n  Per-coin: Train PF {avg_train_pc:.2f} -> Test PF {avg_test_pc:.2f}  (degradation: {deg_pc:.1f}%)")
    print(f"  Universal: Test PF {avg_test_univ:.2f}")
    print(f"  Baseline (no TP): Test PF {avg_test_base:.2f}")

    if deg_pc > 40 or avg_test_univ > avg_test_pc:
        print(f"\n  RECOMMENDATION: Universal params preferred (per-coin degrades {deg_pc:.0f}%)")
        recommendation = 'universal'
    else:
        print(f"\n  RECOMMENDATION: Per-coin params viable (degradation {deg_pc:.0f}%)")
        recommendation = 'per_coin'

    # Is TP better than baseline?
    best_test = max(avg_test_univ, avg_test_pc)
    if best_test > avg_test_base:
        print(f"\n  VERDICT: TP ({best_test:.2f}) BEATS baseline ({avg_test_base:.2f})")
    else:
        print(f"\n  VERDICT: Baseline (no TP, {avg_test_base:.2f}) is already optimal or near-optimal")

    # Low confidence
    print(f"\n{'='*90}")
    print("LOW CONFIDENCE FLAGS (<3 trades in OOS)")
    print(f"{'='*90}")
    for coin, wins in all_results.items():
        low = sum(1 for w in wins if w['test_base_trades'] < 3)
        if low > 0:
            print(f"  {coin:<8} {low}/{len(wins)} windows with < 3 trades")

    # Save
    save_data = {
        'recommendation': recommendation,
        'avg_train_pf_percoin': avg_train_pc,
        'avg_test_pf_percoin': avg_test_pc,
        'avg_test_pf_universal': avg_test_univ,
        'avg_test_pf_baseline': avg_test_base,
        'degradation_percoin_pct': deg_pc,
        'universal_params': universal_params,
        'coin_results': all_results,
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(save_data, f, indent=2)
    print(f"\nResults saved to {RESULTS_FILE}")

    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("Checkpoint removed (clean finish)")


if __name__ == "__main__":
    main()
