#!/usr/bin/env python3
"""
RUN9.1 - Scalp Strategy Grid Search (1m Candles)

Tests 3 scalp strategies overlaid on the existing regime system:
  1. scalp_vol_spike_rev  — Volume spike + RSI extreme reversal
  2. scalp_stoch_cross    — Stochastic extreme crossover
  3. scalp_bb_squeeze_break — BB squeeze breakout with volume

Grid: 1,296 combos × 18 coins. Shadow tracking: regime+scalps vs regime-only.
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run9_1_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run9_1_results.json'

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
REGIME_SL = 0.003       # 0.3% from RUN7
MIN_HOLD = 2

BREADTH_LONG_MAX = 0.20
BREADTH_SHORT_MIN = 0.50
ISO_SHORT_BREADTH_MAX = 0.20

ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}

# Grid parameters
SCALP_SLS = [0.0010, 0.0015, 0.0020, 0.0025]
SCALP_TPS = [0.0020, 0.0030, 0.0040, 0.0050]
VOL_SPIKE_MULTS = [2.5, 3.0, 3.5]
RSI_EXTREMES = [15, 20, 25]
STOCH_EXTREMES = [5, 10, 15]
BB_SQUEEZE_FACTORS = [0.4, 0.5, 0.6]

# Build combo list
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

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, saving checkpoint after current combo...")


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
    """Calculate regime indicators on 15m data."""
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
    """Calculate scalp indicators on 1m data."""
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
    """Build market breadth from 15m data for all coins."""
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
    elif strategy == 'iso_vol_spike':
        return (row['z'] > 1.0 and
                row['v'] > row['vol_ma'] * params['vol_spike_mult'])
    elif strategy == 'iso_bb_squeeze':
        if pd.isna(row.get('bb_width_avg')) or row['bb_width_avg'] == 0:
            return False
        return (row['c'] >= row['bb_hi'] * 0.98 and
                row['bb_width'] < row['bb_width_avg'] * params['squeeze_factor'])
    return False


def scalp_entry_signal(row_1m, params):
    """Check scalp entry on 1m indicators. Returns (direction, strategy_name) or (None, None)."""
    if pd.isna(row_1m.get('rsi')) or pd.isna(row_1m.get('vol_ma')) or row_1m['vol_ma'] == 0:
        return None, None

    vol_r = row_1m['v'] / row_1m['vol_ma']
    rsi_low = params['rsi_extreme']
    rsi_high = 100 - params['rsi_extreme']

    # 1. scalp_vol_spike_rev
    if vol_r > params['vol_spike_mult']:
        if row_1m['rsi'] < rsi_low:
            return 'long', 'scalp_vol_spike_rev'
        if row_1m['rsi'] > rsi_high:
            return 'short', 'scalp_vol_spike_rev'

    # 2. scalp_stoch_cross
    if not pd.isna(row_1m.get('stoch_k')) and not pd.isna(row_1m.get('stoch_d')):
        stoch_lo = params['stoch_extreme']
        stoch_hi = 100 - params['stoch_extreme']
        k = row_1m['stoch_k']
        d = row_1m['stoch_d']
        k_prev = row_1m.get('stoch_k_prev', float('nan'))
        d_prev = row_1m.get('stoch_d_prev', float('nan'))
        if not pd.isna(k_prev) and not pd.isna(d_prev):
            if k_prev <= d_prev and k > d and k < stoch_lo and d < stoch_lo:
                return 'long', 'scalp_stoch_cross'
            if k_prev >= d_prev and k < d and k > stoch_hi and d > stoch_hi:
                return 'short', 'scalp_stoch_cross'

    # 3. scalp_bb_squeeze_break
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
    """
    Run the combined regime + scalp backtest.
    Uses 15m data for regime, 1m data for scalps.
    Iterates on 15m candles; within each 15m window, checks 1m candles for scalps.
    """
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

    position = None       # None, 'long', 'short'
    trade_type = None     # 'regime' or 'scalp'
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

        # === SCALP EXIT (check 1m candles for open scalp positions) ===
        if enable_scalps and position is not None and trade_type == 'scalp':
            next_15m = idx + pd.Timedelta(minutes=15)
            mask = (df_1m.index >= idx) & (df_1m.index < next_15m)
            window_1m = df_1m.loc[mask]

            for m_idx, m_row in window_1m.iterrows():
                p = m_row['c']
                if position == 'long':
                    pnl = (p - entry_price) / entry_price
                else:
                    pnl = (entry_price - p) / entry_price

                if pnl >= scalp_tp:
                    profit = balance * SCALP_RISK * scalp_tp * LEVERAGE
                    balance += profit
                    t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp',
                         'dir': position, 'reason': 'TP'}
                    all_trades.append(t)
                    scalp_trades.append(t)
                    position = None
                    trade_type = None
                    cooldown = 0
                    break
                elif pnl <= -scalp_sl:
                    loss = balance * SCALP_RISK * scalp_sl * LEVERAGE
                    balance -= loss
                    t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp',
                         'dir': position, 'reason': 'SL'}
                    all_trades.append(t)
                    scalp_trades.append(t)
                    position = None
                    trade_type = None
                    cooldown = 0
                    break

        # === REGIME EXIT ===
        if position is not None and trade_type == 'regime':
            candles_held += 1

            if position == 'long':
                if row['h'] > peak_price:
                    peak_price = row['h']
                price_pnl = (price - entry_price) / entry_price
                exited = False

                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': 'SL'}
                    all_trades.append(t)
                    regime_trades.append(t)
                    exited = True

                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if row['c'] > row['sma20'] or row['z'] > 0.5:
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        reason = 'SMA' if row['c'] > row['sma20'] else 'Z0'
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'long', 'reason': reason}
                        all_trades.append(t)
                        regime_trades.append(t)
                        exited = True

                if exited:
                    position = None
                    trade_type = None
                    entry_type = None
                    cooldown = 2
                    candles_held = 0

            elif position == 'short':
                if row['l'] < trough_price:
                    trough_price = row['l']
                price_pnl = (entry_price - price) / entry_price
                exited = False

                if price_pnl <= -REGIME_SL:
                    balance -= balance * REGIME_RISK * REGIME_SL * LEVERAGE
                    t = {'pnl_pct': -REGIME_SL * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': 'SL'}
                    all_trades.append(t)
                    regime_trades.append(t)
                    exited = True

                if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                    if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                        balance += balance * REGIME_RISK * price_pnl * LEVERAGE
                        reason = 'SMA' if price < row['sma20'] else 'Z0'
                        t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': 'short', 'reason': reason}
                        all_trades.append(t)
                        regime_trades.append(t)
                        exited = True

                if exited:
                    position = None
                    trade_type = None
                    entry_type = None
                    cooldown = 2
                    candles_held = 0

        # === COOLDOWN ===
        if cooldown > 0:
            cooldown -= 1

        # === REGIME ENTRY ===
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
                    trade_type = 'regime'
                    entry_price = price
                    peak_price = row['h']
                    entry_type = 'long'
                elif iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    position = 'short'
                    trade_type = 'regime'
                    entry_price = price
                    trough_price = row['l']
                    entry_type = 'iso_short'
            elif market_mode == 'iso_short':
                if iso_short_strat and iso_short_entry_signal(row, iso_short_strat, ISO_SHORT_PARAMS, market_ctx):
                    if b <= ISO_SHORT_BREADTH_MAX or ISO_SHORT_BREADTH_MAX >= 0.50:
                        position = 'short'
                        trade_type = 'regime'
                        entry_price = price
                        trough_price = row['l']
                        entry_type = 'iso_short'
            elif market_mode == 'short':
                if short_entry_signal(row, short_strat):
                    position = 'short'
                    trade_type = 'regime'
                    entry_price = price
                    trough_price = row['l']
                    entry_type = 'market_short'

        # === SCALP ENTRY (only when no position after regime check) ===
        if enable_scalps and position is None and cooldown == 0:
            next_15m = idx + pd.Timedelta(minutes=15)
            mask = (df_1m.index >= idx) & (df_1m.index < next_15m)
            window_1m = df_1m.loc[mask]

            for m_idx, m_row in window_1m.iterrows():
                if position is not None:
                    break

                direction, strat_name = scalp_entry_signal(m_row, scalp_params)
                if direction is not None:
                    position = direction
                    trade_type = 'scalp'
                    entry_price = m_row['c']

                    # Check remaining 1m candles in this window for TP/SL
                    remaining = window_1m.loc[window_1m.index > m_idx]
                    for r_idx, r_row in remaining.iterrows():
                        if position is None:
                            break
                        p = r_row['c']
                        if direction == 'long':
                            pnl_chk = (p - entry_price) / entry_price
                        else:
                            pnl_chk = (entry_price - p) / entry_price

                        if pnl_chk >= scalp_tp:
                            profit = balance * SCALP_RISK * scalp_tp * LEVERAGE
                            balance += profit
                            t = {'pnl_pct': scalp_tp * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'TP', 'strat': strat_name}
                            all_trades.append(t)
                            scalp_trades.append(t)
                            position = None
                            trade_type = None
                            break
                        elif pnl_chk <= -scalp_sl:
                            loss = balance * SCALP_RISK * scalp_sl * LEVERAGE
                            balance -= loss
                            t = {'pnl_pct': -scalp_sl * LEVERAGE * 100, 'type': 'scalp',
                                 'dir': direction, 'reason': 'SL', 'strat': strat_name}
                            all_trades.append(t)
                            scalp_trades.append(t)
                            position = None
                            trade_type = None
                            break

        # Drawdown tracking
        if balance > peak_balance:
            peak_balance = balance
        dd = (peak_balance - balance) / peak_balance * 100
        if dd > max_drawdown:
            max_drawdown = dd

    # Close open position at end
    if position is not None:
        last_price = df_15m.iloc[-1]['c']
        if trade_type == 'regime':
            if position == 'long':
                price_pnl = (last_price - entry_price) / entry_price
            else:
                price_pnl = (entry_price - last_price) / entry_price
            balance += balance * REGIME_RISK * price_pnl * LEVERAGE
            t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'regime', 'dir': position, 'reason': 'END'}
            regime_trades.append(t)
            all_trades.append(t)
        elif trade_type == 'scalp':
            if position == 'long':
                price_pnl = (last_price - entry_price) / entry_price
            else:
                price_pnl = (entry_price - last_price) / entry_price
            balance += balance * SCALP_RISK * price_pnl * LEVERAGE
            t = {'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'scalp', 'dir': position, 'reason': 'END'}
            scalp_trades.append(t)
            all_trades.append(t)

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
    }


def save_checkpoint(results, completed_combos):
    data = {'results': results, 'completed_combos': completed_combos}
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(data, f)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return None


def main():
    print("=" * 90)
    print("RUN9.1 - SCALP STRATEGY GRID SEARCH (1m Candles)")
    print("=" * 90)
    print(f"Total: {len(COMBOS)} combos x {len(COINS)} coins = {len(COMBOS)*len(COINS)} backtests")
    print(f"Grid: scalp_sl={SCALP_SLS}, scalp_tp={SCALP_TPS}")
    print(f"       vol_spike_mult={VOL_SPIKE_MULTS}, rsi_extreme={RSI_EXTREMES}")
    print(f"       stoch_extreme={STOCH_EXTREMES}, bb_squeeze_factor={BB_SQUEEZE_FACTORS}")
    print("=" * 90)

    # Load ISO short strategies from RUN6.1
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat
            print(f"Loaded RUN6.1 ISO short strategies for {len(OPTIMAL_ISO_SHORT_STRAT)} coins")

    # Load data
    all_15m = {}
    all_1m = {}
    for coin in COINS:
        df_15m = load_cache_15m(coin)
        df_1m = load_cache_1m(coin)
        if df_15m is not None:
            all_15m[coin] = df_15m
        if df_1m is not None:
            all_1m[coin] = df_1m

    print(f"\nLoaded {len(all_15m)} coins (15m), {len(all_1m)} coins (1m)")

    if len(all_1m) == 0:
        print("ERROR: No 1m data found! Run run9_0_fetch_1m.py first.")
        sys.exit(1)

    breadth, avg_z, avg_rsi, btc_z = build_market_breadth(all_15m)

    # === BASELINE (regime-only) ===
    print("\nRunning baseline (regime-only) backtests...")
    baseline_results = {}
    for coin in COINS:
        if coin not in all_15m or coin not in all_1m:
            continue
        r = run_combined_backtest(
            all_15m[coin], all_1m[coin],
            OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion'),
            OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev'),
            OPTIMAL_ISO_SHORT_STRAT.get(coin),
            breadth, avg_z, avg_rsi, btc_z,
            scalp_params={}, enable_scalps=False)
        if r:
            baseline_results[coin] = r
            print(f"  {coin}: PF={r['regime']['pf']:.2f} WR={r['regime']['wr']:.0f}% "
                  f"P&L={r['pnl']:+.1f}% trades={r['regime']['trades']}")

    # === SIGNAL FREQUENCY CHECK ===
    print("\nChecking scalp signal frequency (mid-range params)...")
    mid_params = {
        'scalp_sl': 0.0015, 'scalp_tp': 0.003,
        'vol_spike_mult': 3.0, 'rsi_extreme': 20,
        'stoch_extreme': 10, 'bb_squeeze_factor': 0.5,
    }
    for coin in COINS:
        if coin not in all_1m:
            continue
        r = run_combined_backtest(
            all_15m[coin], all_1m[coin],
            OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion'),
            OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev'),
            OPTIMAL_ISO_SHORT_STRAT.get(coin),
            breadth, avg_z, avg_rsi, btc_z,
            scalp_params=mid_params, enable_scalps=True)
        if r:
            n = r['scalp']['trades']
            flag = " *** LOW" if n < 20 else ""
            by_strat = ', '.join(f"{s}={st['trades']}" for s, st in r['scalp_by_strat'].items())
            print(f"  {coin}: {n} scalp signals ({by_strat}){flag}")

    # === GRID SEARCH ===
    checkpoint = load_checkpoint()
    results = {}
    completed_combos = set()

    if checkpoint:
        results = checkpoint['results']
        completed_combos = set(checkpoint['completed_combos'])
        print(f"\nResumed from checkpoint: {len(completed_combos)}/{len(COMBOS)} combos completed")

    total = len(COMBOS)
    done = len(completed_combos)
    start_time = _time.time()
    combos_this_run = 0

    print(f"\nStarting grid search: {done}/{total} combos done")

    for combo_idx, params in enumerate(COMBOS):
        combo_key = (f"sl{params['scalp_sl']}_tp{params['scalp_tp']}_"
                     f"vm{params['vol_spike_mult']}_rsi{params['rsi_extreme']}_"
                     f"st{params['stoch_extreme']}_bb{params['bb_squeeze_factor']}")

        if combo_key in completed_combos:
            continue

        if _shutdown:
            print(f"\nShutdown. Saving checkpoint at {done}/{total}...")
            save_checkpoint(results, list(completed_combos))
            sys.exit(0)

        combo_results = {}
        for coin in COINS:
            if coin not in all_15m or coin not in all_1m:
                continue
            r = run_combined_backtest(
                all_15m[coin], all_1m[coin],
                OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion'),
                OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev'),
                OPTIMAL_ISO_SHORT_STRAT.get(coin),
                breadth, avg_z, avg_rsi, btc_z,
                scalp_params=params, enable_scalps=True)
            if r:
                base_pnl = baseline_results.get(coin, {}).get('pnl', 0)
                r['net_impact'] = r['pnl'] - base_pnl
                combo_results[coin] = r

        results[combo_key] = {'params': params, 'coins': combo_results}
        completed_combos.add(combo_key)
        done += 1
        combos_this_run += 1

        elapsed = _time.time() - start_time
        rate = combos_this_run / elapsed if elapsed > 0 else 0
        remaining = total - done
        eta_min = (remaining / rate / 60) if rate > 0 else 0

        avg_impact = np.mean([r['net_impact'] for r in combo_results.values()]) if combo_results else 0
        print(f"  [{done}/{total}] ({done/total*100:.0f}%) "
              f"sl={params['scalp_sl']} tp={params['scalp_tp']} "
              f"vm={params['vol_spike_mult']} rsi={params['rsi_extreme']} "
              f"st={params['stoch_extreme']} bb={params['bb_squeeze_factor']} "
              f"| avg_impact={avg_impact:+.1f}% | {rate:.2f}/s ETA:{eta_min:.1f}m")

        if done % 20 == 0:
            save_checkpoint(results, list(completed_combos))

    save_checkpoint(results, list(completed_combos))

    # === ANALYSIS ===
    print(f"\n{'='*90}")
    print("RESULTS SUMMARY")
    print(f"{'='*90}")

    if baseline_results:
        base_pnls = [r['pnl'] for r in baseline_results.values()]
        base_pfs = [r['regime']['pf'] for r in baseline_results.values() if r['regime']['trades'] > 0]
        base_wrs = [r['regime']['wr'] for r in baseline_results.values() if r['regime']['trades'] > 0]
        print(f"\n  BASELINE (regime-only):")
        print(f"    Avg WR: {np.mean(base_wrs):.1f}%  Avg PF: {np.mean(base_pfs):.2f}  Avg P&L: {np.mean(base_pnls):+.1f}%")

    best_score = -999
    best_combo = None
    best_key = None
    positive_impact_combos = 0

    for combo_key, data in results.items():
        coins_r = data['coins']
        if not coins_r:
            continue

        impacts = [r['net_impact'] for r in coins_r.values()]
        if np.mean(impacts) > 0:
            positive_impact_combos += 1

        pfs = [r['all']['pf'] for r in coins_r.values() if r['all']['trades'] > 0]
        wrs = [r['all']['wr'] for r in coins_r.values() if r['all']['trades'] > 0]
        if not pfs:
            continue

        score = np.mean(pfs) * (np.mean(wrs) / 100) ** 0.5
        if score > best_score:
            best_score = score
            best_combo = data
            best_key = combo_key

    print(f"\n  Combos with positive net impact: {positive_impact_combos}/{len(results)} "
          f"({positive_impact_combos/len(results)*100:.0f}%)" if results else "")

    if best_combo:
        coins_r = best_combo['coins']
        impacts = [r['net_impact'] for r in coins_r.values()]
        pfs = [r['all']['pf'] for r in coins_r.values() if r['all']['trades'] > 0]
        wrs = [r['all']['wr'] for r in coins_r.values() if r['all']['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        scalp_counts = [r['scalp']['trades'] for r in coins_r.values()]

        print(f"\n  BEST COMBO:")
        print(f"    Params: {best_combo['params']}")
        print(f"    Avg WR: {np.mean(wrs):.1f}%  Avg PF: {np.mean(pfs):.2f}  Avg P&L: {np.mean(pnls):+.1f}%")
        print(f"    Avg net impact: {np.mean(impacts):+.1f}%")
        print(f"    Avg scalp trades: {np.mean(scalp_counts):.0f}")

        print(f"\n  PER-COIN BREAKDOWN (best combo):")
        print(f"    {'Coin':<8} {'Base P&L':<12} {'With Scalp':<12} {'Impact':<10} {'Scalp#':<8} {'Scalp WR':<10} {'Scalp PF'}")
        print(f"    {'-'*75}")
        coins_better = 0
        coins_worse = 0
        for coin in COINS:
            if coin not in coins_r:
                continue
            r = coins_r[coin]
            base_pnl = baseline_results.get(coin, {}).get('pnl', 0)
            impact = r['net_impact']
            if impact > 0:
                coins_better += 1
            else:
                coins_worse += 1
            print(f"    {coin:<8} {base_pnl:<12.1f} {r['pnl']:<12.1f} {impact:<+10.1f} "
                  f"{r['scalp']['trades']:<8} {r['scalp']['wr']:<10.1f} {r['scalp']['pf']:.2f}")
        print(f"\n    Better: {coins_better}  Worse: {coins_worse}")

    if results and positive_impact_combos < len(results) * 0.3:
        print(f"\n  *** EARLY STOP SIGNAL: Only {positive_impact_combos}/{len(results)} combos positive. "
              f"Scalping may not add value. ***")

    # Save results
    save_data = {
        'best_combo_key': best_key,
        'best_params': best_combo['params'] if best_combo else None,
        'baseline': {c: {'pnl': r['pnl'], 'regime': r['regime']} for c, r in baseline_results.items()},
        'positive_impact_combos': positive_impact_combos,
        'total_combos': len(results),
        'best_per_coin': {},
    }

    for coin in COINS:
        best_coin_score = -999
        best_coin_params = None
        for combo_key, data in results.items():
            if coin not in data['coins']:
                continue
            r = data['coins'][coin]
            if r['all']['trades'] < 3:
                continue
            score = r['all']['pf'] * (r['all']['wr'] / 100) ** 0.5
            if score > best_coin_score:
                best_coin_score = score
                best_coin_params = {
                    'params': data['params'],
                    'pnl': r['pnl'],
                    'pf': r['all']['pf'],
                    'wr': r['all']['wr'],
                    'scalp_trades': r['scalp']['trades'],
                    'net_impact': r['net_impact'],
                }
        if best_coin_params:
            save_data['best_per_coin'][coin] = best_coin_params

    with open(RESULTS_FILE, 'w') as f:
        json.dump(save_data, f, indent=2)
    print(f"\nResults saved to {RESULTS_FILE}")

    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("Checkpoint removed (clean finish)")


if __name__ == "__main__":
    main()
