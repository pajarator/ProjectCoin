#!/usr/bin/env python3
"""
RUN9.2 - Walk-Forward Validation of Scalp Strategy

3 windows (train 2mo, test 1mo):
  W1: Oct 15-Dec 14 -> Dec 15-Jan 14
  W2: Nov 15-Jan 14 -> Jan 15-Feb 14
  W3: Dec 15-Feb 14 -> Feb 15-Mar 10

Train: find best scalp params per coin from grid.
Test: apply OOS. Compare universal vs per-coin vs baseline (no scalps).
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run9_2_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run9_2_results.json'

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
REGIME_RISK = 0.10
SCALP_RISK = 0.05
REGIME_SL = 0.003
MIN_HOLD = 2

BREADTH_LONG_MAX = 0.20
BREADTH_SHORT_MIN = 0.50
ISO_SHORT_BREADTH_MAX = 0.20

ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}

# Scalp grid (same as run9_1)
SCALP_SLS = [0.0010, 0.0015, 0.0020, 0.0025]
SCALP_TPS = [0.0020, 0.0030, 0.0040, 0.0050]
VOL_SPIKE_MULTS = [2.5, 3.0, 3.5]
RSI_EXTREMES = [15, 20, 25]
STOCH_EXTREMES = [5, 10, 15]
BB_SQUEEZE_FACTORS = [0.4, 0.5, 0.6]

COMBOS = []
for ssl in SCALP_SLS:
    for stp in SCALP_TPS:
        for vsm in VOL_SPIKE_MULTS:
            for rsi_ex in RSI_EXTREMES:
                for stoch_ex in STOCH_EXTREMES:
                    for bbsf in BB_SQUEEZE_FACTORS:
                        COMBOS.append({
                            'scalp_sl': ssl, 'scalp_tp': stp,
                            'vol_spike_mult': vsm, 'rsi_extreme': rsi_ex,
                            'stoch_extreme': stoch_ex, 'bb_squeeze_factor': bbsf,
                        })

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


def load_cache_1m(name):
    path = f"{DATA_CACHE_DIR}/{name}_USDT_1m_5months.csv"
    if os.path.exists(path):
        return pd.read_csv(path, index_col=0, parse_dates=True)
    return None


def load_cache_15m(name):
    path = f"{DATA_CACHE_DIR}/{name}_USDT_15m_5months.csv"
    if os.path.exists(path):
        return pd.read_csv(path, index_col=0, parse_dates=True)
    return None


def calculate_15m_indicators(df):
    df = df.copy()
    df['sma20'] = df['c'].rolling(20).mean()
    df['sma9'] = df['c'].rolling(9).mean()
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
    typical_price = (df['h'] + df['l'] + df['c']) / 3
    df['vwap'] = (typical_price * df['v']).rolling(20).sum() / df['v'].rolling(20).sum()
    high_low = df['h'] - df['l']
    plus_dm = high_low.where((df['h'] - df['h'].shift()) > (df['l'].shift() - df['l']), 0)
    minus_dm = high_low.where((df['l'].shift() - df['l']) > (df['h'] - df['h'].shift()), 0)
    atr = (pd.concat([high_low, abs(df['h'] - df['c'].shift()), abs(df['l'] - df['c'].shift())], axis=1)
           .max(axis=1).rolling(14).mean())
    plus_di = 100 * (plus_dm.rolling(14).mean() / atr)
    minus_di = 100 * (minus_dm.rolling(14).mean() / atr)
    dx = 100 * abs(plus_di - minus_di) / (plus_di + minus_di)
    df['adx'] = dx.rolling(14).mean()
    return df


def calculate_1m_indicators(df):
    df = df.copy()
    delta = df['c'].diff()
    gain = (delta.where(delta > 0, 0)).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df['rsi'] = 100 - (100 / (1 + rs))
    df['vol_ma'] = df['v'].rolling(20).mean()
    lowest_low = df['l'].rolling(14).min()
    highest_high = df['h'].rolling(14).max()
    df['stoch_k'] = 100 * ((df['c'] - lowest_low) / (highest_high - lowest_low))
    df['stoch_d'] = df['stoch_k'].rolling(3).mean()
    df['stoch_k_prev'] = df['stoch_k'].shift(1)
    df['stoch_d_prev'] = df['stoch_d'].shift(1)
    df['bb_sma'] = df['c'].rolling(20).mean()
    df['bb_std'] = df['c'].rolling(20).std()
    df['bb_upper'] = df['bb_sma'] + 2 * df['bb_std']
    df['bb_lower'] = df['bb_sma'] - 2 * df['bb_std']
    df['bb_width'] = df['bb_upper'] - df['bb_lower']
    df['bb_width_avg'] = df['bb_width'].rolling(20).mean()
    return df


def build_market_breadth(all_15m_data):
    z_frames = {}
    rsi_frames = {}
    for coin, df in all_15m_data.items():
        df_ind = calculate_15m_indicators(df)
        z_frames[coin] = df_ind['z']
        rsi_frames[coin] = df_ind['rsi']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    rsi_df = pd.DataFrame(rsi_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    avg_z = z_df.mean(axis=1)
    avg_rsi = rsi_df.mean(axis=1)
    btc_z = None
    if 'BTC' in all_15m_data:
        btc_df = calculate_15m_indicators(all_15m_data['BTC'])
        btc_z = btc_df['z']
    return breadth, avg_z, avg_rsi, btc_z


def long_entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if row['c'] > row['sma20'] or row['z'] > 0.5:
        return False
    if strategy == 'vwap_reversion':
        return row['z'] < -1.5 and row['c'] < row['sma20'] and row['v'] > row['vol_ma'] * 1.2
    elif strategy == 'bb_bounce':
        return row['c'] <= row['bb_lo'] * 1.02 and row['v'] > row['vol_ma'] * 1.3
    elif strategy == 'adr_reversal':
        return row['c'] <= row['adr_lo'] + (row['adr_hi'] - row['adr_lo']) * 0.25
    elif strategy == 'dual_rsi':
        return row['z'] < -1.0
    elif strategy == 'mean_reversion':
        return row['z'] < -1.5
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
    if strategy == 'iso_relative_z':
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
    elif strategy == 'iso_mean_rev':
        return row['z'] > params['z_threshold']
    elif strategy == 'iso_vwap_rev':
        return (row['z'] > params['z_threshold'] and
                row['c'] > row['sma20'] and row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'iso_bb_bounce':
        return (row['c'] >= row['bb_hi'] * params['bb_margin'] and
                row['v'] > row['vol_ma'] * (params['vol_mult'] + 0.1))
    elif strategy == 'iso_adr_rev':
        adr_range = row['adr_hi'] - row['adr_lo']
        if adr_range <= 0:
            return False
        return (row['c'] >= row['adr_hi'] - adr_range * params['adr_pct'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'iso_vol_spike':
        return (row['z'] > 1.0 and row['v'] > row['vol_ma'] * params['vol_spike_mult'])
    elif strategy == 'iso_bb_squeeze':
        if pd.isna(row.get('bb_width_avg')) or row['bb_width_avg'] == 0:
            return False
        return (row['c'] >= row['bb_hi'] * 0.98 and
                row['bb_width'] < row['bb_width_avg'] * params['squeeze_factor'])
    return False


def scalp_entry_signal(row_1m, params):
    if pd.isna(row_1m.get('rsi')) or pd.isna(row_1m.get('vol_ma')) or row_1m['vol_ma'] == 0:
        return None, None
    vol_r = row_1m['v'] / row_1m['vol_ma']
    rsi_low = params['rsi_extreme']
    rsi_high = 100 - params['rsi_extreme']
    if vol_r > params['vol_spike_mult']:
        if row_1m['rsi'] < rsi_low:
            return 'long', 'scalp_vol_spike_rev'
        if row_1m['rsi'] > rsi_high:
            return 'short', 'scalp_vol_spike_rev'
    if not pd.isna(row_1m.get('stoch_k')) and not pd.isna(row_1m.get('stoch_d')):
        stoch_lo = params['stoch_extreme']
        stoch_hi = 100 - params['stoch_extreme']
        k, d = row_1m['stoch_k'], row_1m['stoch_d']
        k_prev = row_1m.get('stoch_k_prev', float('nan'))
        d_prev = row_1m.get('stoch_d_prev', float('nan'))
        if not pd.isna(k_prev) and not pd.isna(d_prev):
            if k_prev <= d_prev and k > d and k < stoch_lo and d < stoch_lo:
                return 'long', 'scalp_stoch_cross'
            if k_prev >= d_prev and k < d and k > stoch_hi and d > stoch_hi:
                return 'short', 'scalp_stoch_cross'
    if (not pd.isna(row_1m.get('bb_width_avg')) and row_1m['bb_width_avg'] > 0
            and not pd.isna(row_1m.get('bb_upper'))):
        squeeze = row_1m['bb_width'] < row_1m['bb_width_avg'] * params['bb_squeeze_factor']
        if squeeze and vol_r > 2.0:
            if row_1m['c'] > row_1m['bb_upper']:
                return 'long', 'scalp_bb_squeeze_break'
            if row_1m['c'] < row_1m['bb_lower']:
                return 'short', 'scalp_bb_squeeze_break'
    return None, None


def run_combined_backtest(df_15m, df_1m, long_strat, short_strat, iso_short_strat,
                          breadth, avg_z_series, avg_rsi_series, btc_z_series,
                          scalp_params, enable_scalps=True):
    """Run combined regime + scalp backtest (same logic as run9_1)."""
    df_15m = calculate_15m_indicators(df_15m)
    df_15m = df_15m.dropna(subset=['sma20', 'std20', 'rsi'])
    if len(df_15m) < 50:
        return None

    if enable_scalps:
        df_1m = calculate_1m_indicators(df_1m)
        df_1m = df_1m.dropna(subset=['rsi', 'vol_ma'])

    balance = INITIAL_CAPITAL
    peak_balance = INITIAL_CAPITAL
    max_drawdown = 0
    position = None
    trade_type = None
    entry_price = 0
    peak_price = 0
    trough_price = 0
    cooldown = 0
    candles_held = 0
    entry_type = None
    regime_trades = []
    scalp_trades = []
    all_trades = []
    scalp_sl = scalp_params.get('scalp_sl', 0.0015)
    scalp_tp = scalp_params.get('scalp_tp', 0.003)

    for idx, row in df_15m.iterrows():
        price = row['c']
        b = breadth.loc[idx] if idx in breadth.index else 0
        if b <= BREADTH_LONG_MAX:
            market_mode = 'long'
        elif b >= BREADTH_SHORT_MIN:
            market_mode = 'short'
        else:
            market_mode = 'iso_short'

        # Scalp exit
        if enable_scalps and position is not None and trade_type == 'scalp':
            next_15m = idx + pd.Timedelta(minutes=15)
            mask = (df_1m.index >= idx) & (df_1m.index < next_15m)
            for m_idx, m_row in df_1m.loc[mask].iterrows():
                p = m_row['c']
                pnl = ((p - entry_price) / entry_price if position == 'long'
                       else (entry_price - p) / entry_price)
                if pnl >= scalp_tp:
                    balance += balance * SCALP_RISK * scalp_tp * LEVERAGE
                    t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp', 'dir': position, 'reason': 'TP'}
                    all_trades.append(t); scalp_trades.append(t)
                    position = None; trade_type = None; cooldown = 0; break
                elif pnl <= -scalp_sl:
                    balance -= balance * SCALP_RISK * scalp_sl * LEVERAGE
                    t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp', 'dir': position, 'reason': 'SL'}
                    all_trades.append(t); scalp_trades.append(t)
                    position = None; trade_type = None; cooldown = 0; break

        # Regime exit
        if position is not None and trade_type == 'regime':
            candles_held += 1
            if position == 'long':
                if row['h'] > peak_price: peak_price = row['h']
                price_pnl = (price - entry_price) / entry_price
                exited = False
                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': 'SL'}
                    all_trades.append(t); regime_trades.append(t); exited = True
                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if row['c'] > row['sma20'] or row['z'] > 0.5:
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        reason = 'SMA' if row['c'] > row['sma20'] else 'Z0'
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': reason}
                        all_trades.append(t); regime_trades.append(t); exited = True
                if exited:
                    position = None; trade_type = None; entry_type = None; cooldown = 2; candles_held = 0
            elif position == 'short':
                if row['l'] < trough_price: trough_price = row['l']
                price_pnl = (entry_price - price) / entry_price
                exited = False
                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': 'SL'}
                    all_trades.append(t); regime_trades.append(t); exited = True
                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        reason = 'SMA' if price < row['sma20'] else 'Z0'
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': reason}
                        all_trades.append(t); regime_trades.append(t); exited = True
                if exited:
                    position = None; trade_type = None; entry_type = None; cooldown = 2; candles_held = 0

        if cooldown > 0: cooldown -= 1

        # Regime entry
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
                    position = 'long'; trade_type = 'regime'; entry_price = price
                    peak_price = row['h']; entry_type = 'long'
                elif iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    position = 'short'; trade_type = 'regime'; entry_price = price
                    trough_price = row['l']; entry_type = 'iso_short'
            elif market_mode == 'iso_short':
                if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    if b <= ISO_SHORT_BREADTH_MAX or ISO_SHORT_BREADTH_MAX >= 0.50:
                        position = 'short'; trade_type = 'regime'; entry_price = price
                        trough_price = row['l']; entry_type = 'iso_short'
            elif market_mode == 'short':
                if short_entry_signal(row, short_strat):
                    position = 'short'; trade_type = 'regime'; entry_price = price
                    trough_price = row['l']; entry_type = 'market_short'

        # Scalp entry
        if enable_scalps and position is None and cooldown == 0:
            next_15m = idx + pd.Timedelta(minutes=15)
            mask = (df_1m.index >= idx) & (df_1m.index < next_15m)
            window_1m = df_1m.loc[mask]
            for m_idx, m_row in window_1m.iterrows():
                if position is not None: break
                direction, strat_name = scalp_entry_signal(m_row, scalp_params)
                if direction is not None:
                    position = direction; trade_type = 'scalp'; entry_price = m_row['c']
                    remaining = window_1m.loc[window_1m.index > m_idx]
                    for r_idx, r_row in remaining.iterrows():
                        if position is None: break
                        p = r_row['c']
                        pnl_chk = ((p - entry_price) / entry_price if direction == 'long'
                                   else (entry_price - p) / entry_price)
                        if pnl_chk >= scalp_tp:
                            balance += balance * SCALP_RISK * scalp_tp * LEVERAGE
                            t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'TP', 'strat': strat_name}
                            all_trades.append(t); scalp_trades.append(t)
                            position = None; trade_type = None; break
                        elif pnl_chk <= -scalp_sl:
                            balance -= balance * SCALP_RISK * scalp_sl * LEVERAGE
                            t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'SL', 'strat': strat_name}
                            all_trades.append(t); scalp_trades.append(t)
                            position = None; trade_type = None; break

        if balance > peak_balance: peak_balance = balance
        dd = (peak_balance - balance) / peak_balance * 100
        if dd > max_drawdown: max_drawdown = dd

    # Close open
    if position is not None:
        last_price = df_15m.iloc[-1]['c']
        risk = REGIME_RISK if trade_type == 'regime' else SCALP_RISK
        if position == 'long':
            price_pnl = (last_price - entry_price) / entry_price
        else:
            price_pnl = (entry_price - last_price) / entry_price
        balance += balance * risk * price_pnl * LEVERAGE
        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': trade_type, 'dir': position, 'reason': 'END'}
        all_trades.append(t)
        if trade_type == 'regime': regime_trades.append(t)
        else: scalp_trades.append(t)

    if not all_trades:
        return None

    def calc_stats(tlist):
        if not tlist:
            return {'pf': 0, 'wr': 0, 'trades': 0, 'wins': 0}
        wins = [t for t in tlist if t['pnl_pct'] > 0]
        losses = [t for t in tlist if t['pnl_pct'] <= 0]
        tw = sum(t['pnl_pct'] for t in wins) if wins else 0
        tl = sum(t['pnl_pct'] for t in losses) if losses else 0
        return {
            'pf': abs(tw / tl) if tl != 0 else (999 if tw > 0 else 0),
            'wr': len(wins) / len(tlist) * 100,
            'trades': len(tlist),
            'wins': len(wins),
        }

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'all': calc_stats(all_trades),
        'regime': calc_stats(regime_trades),
        'scalp': calc_stats(scalp_trades),
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
    print("=" * 90)
    print("RUN9.2 - WALK-FORWARD VALIDATION OF SCALP STRATEGY")
    print("=" * 90)
    print(f"Train: 2 months | Test: 1 month | 3 windows")
    print(f"Scalp combos: {len(COMBOS)}")
    print("=" * 90)

    # Load ISO short strategies
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat

    # Load RUN9.1 universal params
    r91_file = '/home/scamarena/ProjectCoin/run9_1_results.json'
    universal_params = None
    if os.path.exists(r91_file):
        with open(r91_file, 'r') as f:
            r91 = json.load(f)
        if 'best_params' in r91 and r91['best_params']:
            universal_params = r91['best_params']
            print(f"Loaded universal scalp params from RUN9.1: {universal_params}")
    else:
        print("WARNING: run9_1_results.json not found, will search all combos in train")

    # Load data
    all_15m = {}
    all_1m = {}
    for coin in COINS:
        df_15m = load_cache_15m(coin)
        df_1m = load_cache_1m(coin)
        if df_15m is not None: all_15m[coin] = df_15m
        if df_1m is not None: all_1m[coin] = df_1m
    print(f"\nLoaded {len(all_15m)} coins (15m), {len(all_1m)} coins (1m)")

    if len(all_1m) == 0:
        print("ERROR: No 1m data. Run run9_0_fetch_1m.py first.")
        sys.exit(1)

    breadth, avg_z, avg_rsi, btc_z = build_market_breadth(all_15m)

    # Checkpoint
    checkpoint = load_checkpoint()
    all_results = checkpoint.get('all_results', {}) if checkpoint else {}
    done_keys = set(checkpoint.get('done_keys', [])) if checkpoint else set()

    total_tasks = len(COINS) * len(WINDOWS)
    done_count = len(done_keys)
    print(f"Progress: {done_count}/{total_tasks}")

    start_time = _time.time()
    tasks_this_run = 0

    for coin in COINS:
        if coin not in all_15m or coin not in all_1m:
            continue
        df_15m_full = all_15m[coin]
        df_1m_full = all_1m[coin]

        if coin not in all_results:
            all_results[coin] = []

        long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
        short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')
        iso_short_strat = OPTIMAL_ISO_SHORT_STRAT.get(coin)

        for w in WINDOWS:
            task_key = f"{coin}_{w['name']}"
            if task_key in done_keys:
                continue

            if _shutdown:
                save_checkpoint({'all_results': all_results, 'done_keys': list(done_keys)})
                print(f"Saved checkpoint at {len(done_keys)}/{total_tasks}")
                sys.exit(0)

            train_15m = df_15m_full[(df_15m_full.index >= w['train_start']) & (df_15m_full.index < w['train_end'])]
            test_15m = df_15m_full[(df_15m_full.index >= w['test_start']) & (df_15m_full.index <= w['test_end'])]
            train_1m = df_1m_full[(df_1m_full.index >= w['train_start']) & (df_1m_full.index < w['train_end'])]
            test_1m = df_1m_full[(df_1m_full.index >= w['test_start']) & (df_1m_full.index <= w['test_end'])]

            train_breadth = breadth[(breadth.index >= w['train_start']) & (breadth.index < w['train_end'])]
            test_breadth = breadth[(breadth.index >= w['test_start']) & (breadth.index <= w['test_end'])]
            train_avg_z = avg_z[(avg_z.index >= w['train_start']) & (avg_z.index < w['train_end'])]
            test_avg_z = avg_z[(avg_z.index >= w['test_start']) & (avg_z.index <= w['test_end'])]
            train_avg_rsi = avg_rsi[(avg_rsi.index >= w['train_start']) & (avg_rsi.index < w['train_end'])]
            test_avg_rsi = avg_rsi[(avg_rsi.index >= w['test_start']) & (avg_rsi.index <= w['test_end'])]
            train_btc_z = btc_z[(btc_z.index >= w['train_start']) & (btc_z.index < w['train_end'])] if btc_z is not None else None
            test_btc_z = btc_z[(btc_z.index >= w['test_start']) & (btc_z.index <= w['test_end'])] if btc_z is not None else None

            if len(train_15m) < 100 or len(test_15m) < 50:
                done_keys.add(task_key)
                continue

            # Baseline: no scalps
            test_base = run_combined_backtest(
                test_15m, test_1m, long_strat, short_strat, iso_short_strat,
                test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                scalp_params={}, enable_scalps=False)

            # Train: find best scalp combo
            best_train_score = -999
            best_train_combo = None
            best_train_result = None

            for params in COMBOS:
                r = run_combined_backtest(
                    train_15m, train_1m, long_strat, short_strat, iso_short_strat,
                    train_breadth, train_avg_z, train_avg_rsi, train_btc_z,
                    scalp_params=params, enable_scalps=True)
                if r and r['all']['trades'] >= 3:
                    score = r['all']['pf'] * (r['all']['wr'] / 100) ** 0.5
                    if score > best_train_score:
                        best_train_score = score
                        best_train_combo = params
                        best_train_result = r

            # Test: per-coin best
            test_percoin = None
            if best_train_combo:
                test_percoin = run_combined_backtest(
                    test_15m, test_1m, long_strat, short_strat, iso_short_strat,
                    test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                    scalp_params=best_train_combo, enable_scalps=True)

            # Test: universal params
            test_universal = None
            if universal_params:
                test_universal = run_combined_backtest(
                    test_15m, test_1m, long_strat, short_strat, iso_short_strat,
                    test_breadth, test_avg_z, test_avg_rsi, test_btc_z,
                    scalp_params=universal_params, enable_scalps=True)

            window_result = {
                'window': w['name'],
                'train_best_combo': best_train_combo,
                'train_best_pf': best_train_result['all']['pf'] if best_train_result else 0,
                'test_base_pf': test_base['all']['pf'] if test_base else 0,
                'test_base_wr': test_base['all']['wr'] if test_base else 0,
                'test_base_pnl': test_base['pnl'] if test_base else 0,
                'test_base_trades': test_base['all']['trades'] if test_base else 0,
                'test_percoin_pf': test_percoin['all']['pf'] if test_percoin else 0,
                'test_percoin_wr': test_percoin['all']['wr'] if test_percoin else 0,
                'test_percoin_pnl': test_percoin['pnl'] if test_percoin else 0,
                'test_percoin_trades': test_percoin['all']['trades'] if test_percoin else 0,
                'test_percoin_scalps': test_percoin['scalp']['trades'] if test_percoin else 0,
                'test_universal_pf': test_universal['all']['pf'] if test_universal else 0,
                'test_universal_wr': test_universal['all']['wr'] if test_universal else 0,
                'test_universal_pnl': test_universal['pnl'] if test_universal else 0,
                'test_universal_trades': test_universal['all']['trades'] if test_universal else 0,
                'test_universal_scalps': test_universal['scalp']['trades'] if test_universal else 0,
            }
            all_results[coin].append(window_result)
            done_keys.add(task_key)
            done_count += 1
            tasks_this_run += 1

            elapsed = _time.time() - start_time
            rate = tasks_this_run / elapsed if elapsed > 0 else 0
            eta_min = ((total_tasks - done_count) / rate / 60) if rate > 0 else 0
            print(f"  [{done_count}/{total_tasks}] {coin} {w['name']} "
                  f"| base_pnl={window_result['test_base_pnl']:+.1f}% "
                  f"percoin_pnl={window_result['test_percoin_pnl']:+.1f}% "
                  f"univ_pnl={window_result['test_universal_pnl']:+.1f}% "
                  f"| {rate:.2f}/s ETA:{eta_min:.1f}m")

        save_checkpoint({'all_results': all_results, 'done_keys': list(done_keys)})

    # === PRINT RESULTS ===
    print(f"\n{'='*90}")
    print("WALK-FORWARD RESULTS BY COIN")
    print(f"{'='*90}")

    for coin, wins in all_results.items():
        if not wins: continue
        print(f"\n{coin}")
        print(f"  {'Win':<4} {'Base PF':<10} {'Base WR':<10} {'PerCoin PF':<12} {'PerCoin WR':<12} "
              f"{'Univ PF':<10} {'Univ WR':<10} {'Scalps#'}")
        print(f"  {'-'*95}")
        for w in wins:
            low_conf = " *" if w['test_base_trades'] < 3 else ""
            print(f"  {w['window']:<4} {w['test_base_pf']:<10.2f} {w['test_base_wr']:<10.1f} "
                  f"{w['test_percoin_pf']:<12.2f} {w['test_percoin_wr']:<12.1f} "
                  f"{w['test_universal_pf']:<10.2f} {w['test_universal_wr']:<10.1f} "
                  f"{w['test_universal_scalps']}{low_conf}")

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
    print(f"  Baseline (no scalps): Test PF {avg_test_base:.2f}")

    if deg_pc > 40 or avg_test_univ > avg_test_pc:
        print(f"\n  RECOMMENDATION: Universal params preferred (per-coin degrades {deg_pc:.0f}%)")
        recommendation = 'universal'
    else:
        print(f"\n  RECOMMENDATION: Per-coin params viable (degradation {deg_pc:.0f}%)")
        recommendation = 'per_coin'

    best_test = max(avg_test_univ, avg_test_pc)
    if best_test > avg_test_base:
        print(f"\n  VERDICT: Scalps ({best_test:.2f}) BEAT baseline ({avg_test_base:.2f})")
    else:
        print(f"\n  VERDICT: Baseline ({avg_test_base:.2f}) is already optimal — scalps don't help OOS")

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
