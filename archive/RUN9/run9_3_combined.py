#!/usr/bin/env python3
"""
RUN9.3 - Combined Backtest: v8 (Regime Only) vs v9 (Regime + Scalps)

Full portfolio comparison with per-coin breakdown, scalp vs regime trade stats,
and exit reason distribution.
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run9_3_results.json'

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

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received...")


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
        if pd.isna(row.get('rsi')): return False
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
        if adr_range <= 0: return False
        return (row['c'] >= row['adr_hi'] - adr_range * params['adr_pct'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'iso_vol_spike':
        return (row['z'] > 1.0 and row['v'] > row['vol_ma'] * params['vol_spike_mult'])
    elif strategy == 'iso_bb_squeeze':
        if pd.isna(row.get('bb_width_avg')) or row['bb_width_avg'] == 0: return False
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
        if row_1m['rsi'] < rsi_low: return 'long', 'scalp_vol_spike_rev'
        if row_1m['rsi'] > rsi_high: return 'short', 'scalp_vol_spike_rev'
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
            if row_1m['c'] > row_1m['bb_upper']: return 'long', 'scalp_bb_squeeze_break'
            if row_1m['c'] < row_1m['bb_lower']: return 'short', 'scalp_bb_squeeze_break'
    return None, None


def run_combined_backtest(df_15m, df_1m, long_strat, short_strat, iso_short_strat,
                          breadth, avg_z_series, avg_rsi_series, btc_z_series,
                          scalp_params, enable_scalps=True):
    """Run combined regime + scalp backtest with detailed tracking."""
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
    reason_counts = {}

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
                    t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp', 'dir': position, 'reason': 'SCALP_TP'}
                    all_trades.append(t); scalp_trades.append(t)
                    reason_counts['SCALP_TP'] = reason_counts.get('SCALP_TP', 0) + 1
                    position = None; trade_type = None; cooldown = 0; break
                elif pnl <= -scalp_sl:
                    balance -= balance * SCALP_RISK * scalp_sl * LEVERAGE
                    t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp', 'dir': position, 'reason': 'SCALP_SL'}
                    all_trades.append(t); scalp_trades.append(t)
                    reason_counts['SCALP_SL'] = reason_counts.get('SCALP_SL', 0) + 1
                    position = None; trade_type = None; cooldown = 0; break

        # Regime exit
        if position is not None and trade_type == 'regime':
            candles_held += 1
            if position == 'long':
                if row['h'] > peak_price: peak_price = row['h']
                price_pnl = (price - entry_price) / entry_price
                exited = False; exit_reason = None
                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': 'SL'}
                    all_trades.append(t); regime_trades.append(t); exited = True; exit_reason = 'SL'
                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if row['c'] > row['sma20'] or row['z'] > 0.5:
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        exit_reason = 'SMA' if row['c'] > row['sma20'] else 'Z0'
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': exit_reason}
                        all_trades.append(t); regime_trades.append(t); exited = True
                if exited:
                    reason_counts[exit_reason] = reason_counts.get(exit_reason, 0) + 1
                    position = None; trade_type = None; entry_type = None; cooldown = 2; candles_held = 0
            elif position == 'short':
                if row['l'] < trough_price: trough_price = row['l']
                price_pnl = (entry_price - price) / entry_price
                exited = False; exit_reason = None
                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': 'SL'}
                    all_trades.append(t); regime_trades.append(t); exited = True; exit_reason = 'SL'
                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                        exit_reason = 'SMA' if price < row['sma20'] else 'Z0'
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': exit_reason}
                        all_trades.append(t); regime_trades.append(t); exited = True
                if exited:
                    reason_counts[exit_reason] = reason_counts.get(exit_reason, 0) + 1
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
                    for r_idx, r_row in window_1m.loc[window_1m.index > m_idx].iterrows():
                        if position is None: break
                        p = r_row['c']
                        pnl_chk = ((p - entry_price) / entry_price if direction == 'long'
                                   else (entry_price - p) / entry_price)
                        if pnl_chk >= scalp_tp:
                            balance += balance * SCALP_RISK * scalp_tp * LEVERAGE
                            t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'SCALP_TP', 'strat': strat_name}
                            all_trades.append(t); scalp_trades.append(t)
                            reason_counts['SCALP_TP'] = reason_counts.get('SCALP_TP', 0) + 1
                            position = None; trade_type = None; break
                        elif pnl_chk <= -scalp_sl:
                            balance -= balance * SCALP_RISK * scalp_sl * LEVERAGE
                            t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'SCALP_SL', 'strat': strat_name}
                            all_trades.append(t); scalp_trades.append(t)
                            reason_counts['SCALP_SL'] = reason_counts.get('SCALP_SL', 0) + 1
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
        reason_counts['END'] = reason_counts.get('END', 0) + 1

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

    scalp_by_strat = {}
    for t in scalp_trades:
        s = t.get('strat', 'unknown')
        if s not in scalp_by_strat:
            scalp_by_strat[s] = []
        scalp_by_strat[s].append(t)

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'all': calc_stats(all_trades),
        'regime': calc_stats(regime_trades),
        'scalp': calc_stats(scalp_trades),
        'scalp_by_strat': {s: calc_stats(tl) for s, tl in scalp_by_strat.items()},
        'reason_counts': reason_counts,
    }


def main():
    print("=" * 90)
    print("RUN9.3 - COMBINED BACKTEST: v8 (Regime Only) vs v9 (Regime + Scalps)")
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

    # Load scalp params from run9_1/run9_2
    scalp_params = None

    r92_file = '/home/scamarena/ProjectCoin/run9_2_results.json'
    r91_file = '/home/scamarena/ProjectCoin/run9_1_results.json'

    if os.path.exists(r92_file):
        with open(r92_file, 'r') as f:
            r92 = json.load(f)
        if 'universal_params' in r92 and r92['universal_params']:
            scalp_params = r92['universal_params']
            print(f"RUN9.2 recommendation: {r92.get('recommendation', '?')}")

    if scalp_params is None and os.path.exists(r91_file):
        with open(r91_file, 'r') as f:
            r91 = json.load(f)
        if 'best_params' in r91 and r91['best_params']:
            scalp_params = r91['best_params']

    if scalp_params:
        print(f"Scalp params: {scalp_params}")
    else:
        print("WARNING: No scalp params found. Using defaults.")
        scalp_params = {
            'scalp_sl': 0.0015, 'scalp_tp': 0.003,
            'vol_spike_mult': 3.0, 'rsi_extreme': 20,
            'stoch_extreme': 10, 'bb_squeeze_factor': 0.5,
        }

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

    # === RUN BOTH MODES ===
    results_v8 = {}
    results_v9 = {}

    print("\nRunning backtests...")
    for i, coin in enumerate(COINS):
        if _shutdown: break
        if coin not in all_15m or coin not in all_1m: continue

        long_strat = OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion')
        short_strat = OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev')
        iso_short_strat = OPTIMAL_ISO_SHORT_STRAT.get(coin)

        # v8: regime only
        r = run_combined_backtest(all_15m[coin], all_1m[coin], long_strat, short_strat, iso_short_strat,
                                  breadth, avg_z, avg_rsi, btc_z,
                                  scalp_params={}, enable_scalps=False)
        if r: results_v8[coin] = r

        # v9: regime + scalps
        r = run_combined_backtest(all_15m[coin], all_1m[coin], long_strat, short_strat, iso_short_strat,
                                  breadth, avg_z, avg_rsi, btc_z,
                                  scalp_params=scalp_params, enable_scalps=True)
        if r: results_v9[coin] = r

        print(f"  [{i+1}/{len(COINS)}] {coin} done")

    # === COMPARISON TABLE ===
    print(f"\n{'='*90}")
    print("v8 (REGIME ONLY) vs v9 (REGIME + SCALPS)")
    print(f"{'='*90}")

    def portfolio_stats(results):
        if not results: return {}
        wrs = [r['all']['wr'] for r in results.values() if r['all']['trades'] > 0]
        pfs = [r['all']['pf'] for r in results.values() if r['all']['trades'] > 0]
        pnls = [r['pnl'] for r in results.values()]
        dds = [r['max_dd'] for r in results.values()]
        return {
            'avg_wr': np.mean(wrs) if wrs else 0,
            'avg_pf': np.mean(pfs) if pfs else 0,
            'avg_pnl': np.mean(pnls) if pnls else 0,
            'avg_dd': np.mean(dds) if dds else 0,
            'total_trades': sum(r['all']['trades'] for r in results.values()),
            'total_pnl': sum(r['pnl'] for r in results.values()),
        }

    s_v8 = portfolio_stats(results_v8)
    s_v9 = portfolio_stats(results_v9)

    print(f"\n{'Mode':<30} {'Avg WR':<10} {'Avg PF':<10} {'Trades':<10} {'Avg MaxDD':<12} {'Avg P&L':<12} {'Total P&L'}")
    print("-" * 100)
    if s_v8:
        print(f"{'v8 (regime only)':<30} {s_v8['avg_wr']:<10.1f}% {s_v8['avg_pf']:<10.2f} "
              f"{s_v8['total_trades']:<10} {s_v8['avg_dd']:<12.1f}% {s_v8['avg_pnl']:<12.1f}% {s_v8['total_pnl']:+.1f}%")
    if s_v9:
        print(f"{'v9 (regime + scalps)':<30} {s_v9['avg_wr']:<10.1f}% {s_v9['avg_pf']:<10.2f} "
              f"{s_v9['total_trades']:<10} {s_v9['avg_dd']:<12.1f}% {s_v9['avg_pnl']:<12.1f}% {s_v9['total_pnl']:+.1f}%")

    if s_v8 and s_v9:
        print(f"\n  Delta:")
        print(f"    WR:     {s_v9['avg_wr']-s_v8['avg_wr']:+.1f}%")
        print(f"    PF:     {s_v9['avg_pf']-s_v8['avg_pf']:+.2f}")
        print(f"    P&L:    {s_v9['avg_pnl']-s_v8['avg_pnl']:+.1f}%")
        print(f"    MaxDD:  {s_v9['avg_dd']-s_v8['avg_dd']:+.1f}%")
        print(f"    Trades: {s_v9['total_trades']-s_v8['total_trades']:+d}")

    # === PER-COIN BREAKDOWN ===
    print(f"\n{'='*90}")
    print("PER-COIN BREAKDOWN (v8 -> v9)")
    print(f"{'='*90}")

    print(f"\n{'Coin':<8} {'v8 WR':<10} {'v9 WR':<10} {'v8 PF':<10} {'v9 PF':<10} "
          f"{'v8 P&L':<10} {'v9 P&L':<10} {'Scalps':<8} {'Scalp WR':<10} {'Better?'}")
    print("-" * 100)

    coins_better = 0
    coins_worse = 0
    for coin in COINS:
        if coin not in results_v8 or coin not in results_v9:
            continue
        rv8 = results_v8[coin]
        rv9 = results_v9[coin]
        better = "YES" if rv9['pnl'] > rv8['pnl'] else "NO"
        if rv9['pnl'] > rv8['pnl']: coins_better += 1
        else: coins_worse += 1
        scalp_wr = f"{rv9['scalp']['wr']:.0f}%" if rv9['scalp']['trades'] > 0 else "-"
        print(f"{coin:<8} {rv8['all']['wr']:<10.1f} {rv9['all']['wr']:<10.1f} "
              f"{rv8['all']['pf']:<10.2f} {rv9['all']['pf']:<10.2f} "
              f"{rv8['pnl']:<10.1f} {rv9['pnl']:<10.1f} {rv9['scalp']['trades']:<8} {scalp_wr:<10} {better}")

    print(f"\n  Better: {coins_better}  Worse: {coins_worse}")

    # === SCALP STRATEGY BREAKDOWN ===
    if results_v9:
        print(f"\n{'='*90}")
        print("SCALP STRATEGY BREAKDOWN (v9)")
        print(f"{'='*90}")

        total_by_strat = {}
        for r in results_v9.values():
            for strat, stats in r.get('scalp_by_strat', {}).items():
                if strat not in total_by_strat:
                    total_by_strat[strat] = {'trades': 0, 'wins': 0, 'pnl_sum': 0}
                total_by_strat[strat]['trades'] += stats['trades']
                total_by_strat[strat]['wins'] += stats['wins']

        print(f"\n  {'Strategy':<25} {'Trades':<10} {'Wins':<10} {'WR'}")
        print(f"  {'-'*55}")
        for strat, stats in sorted(total_by_strat.items(), key=lambda x: -x[1]['trades']):
            wr = stats['wins'] / stats['trades'] * 100 if stats['trades'] > 0 else 0
            print(f"  {strat:<25} {stats['trades']:<10} {stats['wins']:<10} {wr:.1f}%")

    # === EXIT REASON DISTRIBUTION ===
    if results_v9:
        print(f"\n{'='*90}")
        print("EXIT REASON DISTRIBUTION (v9)")
        print(f"{'='*90}")
        total_reasons = {}
        for r in results_v9.values():
            for reason, count in r.get('reason_counts', {}).items():
                total_reasons[reason] = total_reasons.get(reason, 0) + count
        total = sum(total_reasons.values())
        for reason, count in sorted(total_reasons.items(), key=lambda x: -x[1]):
            print(f"  {reason:<12} {count:>5}  ({count/total*100:.1f}%)")

    # Save
    save_data = {
        'scalp_params': scalp_params,
        'portfolio_v8': s_v8,
        'portfolio_v9': s_v9,
        'coins_better': coins_better,
        'coins_worse': coins_worse,
        'per_coin_v8': {c: {'pnl': r['pnl'], 'max_dd': r['max_dd'], 'all': r['all'],
                             'regime': r['regime']}
                        for c, r in results_v8.items()},
        'per_coin_v9': {c: {'pnl': r['pnl'], 'max_dd': r['max_dd'], 'all': r['all'],
                             'regime': r['regime'], 'scalp': r['scalp'],
                             'scalp_by_strat': r.get('scalp_by_strat', {}),
                             'reason_counts': r.get('reason_counts', {})}
                        for c, r in results_v9.items()},
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(save_data, f, indent=2)
    print(f"\nResults saved to {RESULTS_FILE}")


if __name__ == "__main__":
    main()
