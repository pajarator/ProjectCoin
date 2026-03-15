#!/usr/bin/env python3
"""
RUN7.1 - Unified Stop Loss Optimization

Instead of a fixed -0.5% SL + bolted-on protection, optimize the stop loss as a single
system with three modes:

  none      — fixed SL, no trailing (current behavior is initial_sl=0.005, none)
  breakeven — after profit reaches activation, SL moves to entry price (0% loss)
  trail     — after profit reaches activation, SL trails at peak - trail_distance

Grid:
  initial_sl:       [0.003, 0.005, 0.007, 0.010]
  trail_mode:       [none, breakeven, trail]
  trail_activation: [0.001, 0.002, 0.003, 0.005]  (breakeven/trail only)
  trail_distance:   [0.001, 0.002, 0.003, 0.004]  (trail only)

  none:      4 combos
  breakeven: 4 × 4 = 16 combos
  trail:     4 × 4 × 4 = 64 combos
  Total:     84 combos × 18 coins = 1,512 backtests

Shadow tracking: for every trail/breakeven exit, track what would have happened
without trailing (just the initial SL + signal exits).
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run7_1_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run7_1_results.json'

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

# Parameter grids
INITIAL_SLS = [0.003, 0.005, 0.007, 0.010]
TRAIL_MODES = ['none', 'breakeven', 'trail']
TRAIL_ACTIVATIONS = [0.001, 0.002, 0.003, 0.005]
TRAIL_DISTANCES = [0.001, 0.002, 0.003, 0.004]

# Build combo list
COMBOS = []
for sl in INITIAL_SLS:
    COMBOS.append(('none', sl, 0.0, 0.0))
for sl in INITIAL_SLS:
    for act in TRAIL_ACTIVATIONS:
        COMBOS.append(('breakeven', sl, act, 0.0))
for sl in INITIAL_SLS:
    for act in TRAIL_ACTIVATIONS:
        for dist in TRAIL_DISTANCES:
            COMBOS.append(('trail', sl, act, dist))

SHADOW_MAX_CANDLES = 50

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, saving checkpoint after current combo...")


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
    """
    Run combined v7 backtest with unified stop loss.

    Exit priority:
      1. Initial SL (-initial_sl)
      2. Trailing/breakeven stop (if activated) — fires regardless of MIN_HOLD
      3. Signal exits (SMA, Z0) — after MIN_HOLD and in profit
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
    peak_price = 0     # highest price seen (longs)
    trough_price = 0   # lowest price seen (shorts)
    trades = []
    cooldown = 0
    candles_held = 0
    entry_type = None

    # Tracking
    trail_exits = 0
    shadow_positions = []

    for row_idx, (idx, row) in enumerate(df.iterrows()):
        price = row['c']

        b = breadth.loc[idx] if idx in breadth.index else 0
        if b <= BREADTH_LONG_MAX:
            market_mode = 'long'
        elif b >= BREADTH_SHORT_MIN:
            market_mode = 'short'
        else:
            market_mode = 'iso_short'

        # === EXIT LOGIC ===
        if position == 'long':
            candles_held += 1
            if row['h'] > peak_price:
                peak_price = row['h']
            price_pnl = (price - entry_price) / entry_price

            exited = False
            exit_reason = None

            # 1. Initial Stop Loss
            if price_pnl <= -initial_sl:
                loss = balance * RISK * initial_sl * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -initial_sl * LEVERAGE * 100, 'dir': 'long',
                               'type': 'loss', 'reason': 'SL'})
                exited = True
                exit_reason = 'SL'

            # 2. Trailing / breakeven stop (fires regardless of MIN_HOLD)
            if not exited and trail_mode != 'none':
                peak_pct = (peak_price - entry_price) / entry_price
                if peak_pct >= trail_activation:
                    triggered = False
                    if trail_mode == 'breakeven':
                        # SL moves to entry price
                        if price <= entry_price:
                            triggered = True
                            exit_reason = 'BE'
                    elif trail_mode == 'trail':
                        # SL trails at peak - trail_distance
                        trail_level = peak_price * (1 - trail_distance)
                        if price <= trail_level:
                            triggered = True
                            exit_reason = 'TRAIL'

                    if triggered:
                        profit = balance * RISK * price_pnl * LEVERAGE
                        balance += profit
                        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                                       'type': 'win' if price_pnl > 0 else 'loss',
                                       'reason': exit_reason})
                        exited = True
                        trail_exits += 1

                        # Start shadow: what if we stayed with just initial SL + signals?
                        shadow_positions.append({
                            'entry_price': entry_price,
                            'exit_pnl': price_pnl,
                            'dir': 'long',
                            'start_row_idx': row_idx,
                            'candles_held_at_exit': candles_held,
                            'shadow_outcome': None,
                            'shadow_pnl': None,
                        })

            # 3. Signal exits (SMA, Z0) — after MIN_HOLD and in profit
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20'] or row['z'] > 0.5:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    reason = 'SMA' if row['c'] > row['sma20'] else 'Z0'
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                                   'type': 'win', 'reason': reason})
                    exited = True

            if exited:
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

            # 1. Initial Stop Loss
            if price_pnl <= -initial_sl:
                loss = balance * RISK * initial_sl * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -initial_sl * LEVERAGE * 100, 'dir': 'short',
                               'type': 'loss', 'reason': 'SL', 'sub': entry_type})
                exited = True
                exit_reason = 'SL'

            # 2. Trailing / breakeven stop
            if not exited and trail_mode != 'none':
                trough_pct = (entry_price - trough_price) / entry_price
                if trough_pct >= trail_activation:
                    triggered = False
                    if trail_mode == 'breakeven':
                        if price >= entry_price:
                            triggered = True
                            exit_reason = 'BE'
                    elif trail_mode == 'trail':
                        trail_level = trough_price * (1 + trail_distance)
                        if price >= trail_level:
                            triggered = True
                            exit_reason = 'TRAIL'

                    if triggered:
                        profit = balance * RISK * price_pnl * LEVERAGE
                        balance += profit
                        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                                       'type': 'win' if price_pnl > 0 else 'loss',
                                       'reason': exit_reason, 'sub': entry_type})
                        exited = True
                        trail_exits += 1

                        shadow_positions.append({
                            'entry_price': entry_price,
                            'exit_pnl': price_pnl,
                            'dir': 'short',
                            'start_row_idx': row_idx,
                            'candles_held_at_exit': candles_held,
                            'shadow_outcome': None,
                            'shadow_pnl': None,
                        })

            # 3. Signal exits
            if not exited and price_pnl > 0 and candles_held >= MIN_HOLD:
                exit_cond = price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']
                if exit_cond:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    reason = 'SMA' if price < row['sma20'] else 'Z0'
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                                   'type': 'win', 'reason': reason, 'sub': entry_type})
                    exited = True

            if exited:
                position = None
                entry_type = None
                cooldown = 3
                candles_held = 0

        # === Update shadow positions (50-candle cap) ===
        for sp in shadow_positions:
            if sp['shadow_outcome'] is not None:
                continue
            candles_since = row_idx - sp['start_row_idx']
            if candles_since >= SHADOW_MAX_CANDLES:
                if sp['dir'] == 'long':
                    sp['shadow_pnl'] = (price - sp['entry_price']) / sp['entry_price']
                else:
                    sp['shadow_pnl'] = (sp['entry_price'] - price) / sp['entry_price']
                sp['shadow_outcome'] = 'TIMEOUT'
                continue
            if sp['dir'] == 'long':
                shadow_pnl = (price - sp['entry_price']) / sp['entry_price']
                if shadow_pnl <= -initial_sl:
                    sp['shadow_outcome'] = 'SL'
                    sp['shadow_pnl'] = -initial_sl
                elif shadow_pnl > 0 and (candles_since + sp['candles_held_at_exit']) >= MIN_HOLD:
                    if row['c'] > row['sma20'] or row['z'] > 0.5:
                        sp['shadow_outcome'] = 'SIGNAL'
                        sp['shadow_pnl'] = shadow_pnl
            else:
                shadow_pnl = (sp['entry_price'] - price) / sp['entry_price']
                if shadow_pnl <= -initial_sl:
                    sp['shadow_outcome'] = 'SL'
                    sp['shadow_pnl'] = -initial_sl
                elif shadow_pnl > 0 and (candles_since + sp['candles_held_at_exit']) >= MIN_HOLD:
                    if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                        sp['shadow_outcome'] = 'SIGNAL'
                        sp['shadow_pnl'] = shadow_pnl

        # === COOLDOWN ===
        if cooldown > 0:
            cooldown -= 1

        # === ENTRY LOGIC ===
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

        # Track equity/drawdown
        if balance > peak_balance:
            peak_balance = balance
        dd = (peak_balance - balance) / peak_balance * 100
        if dd > max_drawdown:
            max_drawdown = dd

    # Close open position at end
    if position == 'long':
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'long',
                       'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END'})
    elif position == 'short':
        price_pnl = (entry_price - df.iloc[-1]['c']) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'dir': 'short',
                       'type': 'win' if price_pnl > 0 else 'loss', 'reason': 'END', 'sub': entry_type})

    # Mark unresolved shadows
    for sp in shadow_positions:
        if sp['shadow_outcome'] is None:
            last_price = df.iloc[-1]['c']
            if sp['dir'] == 'long':
                sp['shadow_pnl'] = (last_price - sp['entry_price']) / sp['entry_price']
            else:
                sp['shadow_pnl'] = (sp['entry_price'] - last_price) / sp['entry_price']
            sp['shadow_outcome'] = 'END'

    if not trades:
        return None

    wins = [t for t in trades if t['pnl_pct'] > 0]
    losses = [t for t in trades if t['pnl_pct'] <= 0]
    tw = sum(t['pnl_pct'] for t in wins) if wins else 0
    tl = sum(t['pnl_pct'] for t in losses) if losses else 0

    # Shadow analysis: saves (shadow hit SL) vs premature (shadow got bigger win)
    trail_saves = 0
    trail_premature = 0
    save_pcts = []
    premature_costs = []

    for sp in shadow_positions:
        if sp['shadow_outcome'] == 'SL':
            trail_saves += 1
            save_pcts.append(sp['exit_pnl'] - sp['shadow_pnl'])
        elif sp['shadow_outcome'] == 'SIGNAL':
            if sp['shadow_pnl'] > sp['exit_pnl']:
                trail_premature += 1
                premature_costs.append(sp['shadow_pnl'] - sp['exit_pnl'])

    net_impact = trail_saves - trail_premature

    # Exit reason counts
    reason_counts = {}
    for t in trades:
        r = t.get('reason', '?')
        reason_counts[r] = reason_counts.get(r, 0) + 1

    return {
        'balance': balance,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'max_dd': max_drawdown,
        'trades': len(trades),
        'wins': len(wins),
        'wr': len(wins) / len(trades) * 100 if trades else 0,
        'pf': abs(tw / tl) if tl != 0 else 0,
        'trail_exits': trail_exits,
        'trail_saves': trail_saves,
        'trail_premature': trail_premature,
        'net_impact': net_impact,
        'avg_save_pct': np.mean(save_pcts) * 100 if save_pcts else 0,
        'avg_premature_cost': np.mean(premature_costs) * 100 if premature_costs else 0,
        'reason_counts': reason_counts,
    }


def save_checkpoint(results, completed_combos):
    data = {
        'results': results,
        'completed_combos': completed_combos,
    }
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
    print("RUN7.1 - UNIFIED STOP LOSS OPTIMIZATION")
    print("=" * 90)

    n_none = len(INITIAL_SLS)
    n_be = len(INITIAL_SLS) * len(TRAIL_ACTIVATIONS)
    n_trail = len(INITIAL_SLS) * len(TRAIL_ACTIVATIONS) * len(TRAIL_DISTANCES)
    print(f"Modes: none ({n_none}), breakeven ({n_be}), trail ({n_trail})")
    print(f"Total: {len(COMBOS)} combos × {len(COINS)} coins = {len(COMBOS)*len(COINS)} backtests")
    print(f"Current baseline: initial_sl=0.005, trail_mode=none")
    print("=" * 90)

    # Load RUN6.1 ISO short strategies
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            for coin, strat in r61['optimal_iso_short_strat'].items():
                OPTIMAL_ISO_SHORT_STRAT[coin] = strat
            print(f"Loaded RUN6.1 ISO short strategies for {len(OPTIMAL_ISO_SHORT_STRAT)} coins")
        if 'best_iso_params' in r61:
            bmaxes = [d.get('breadth_max', 0.20) for d in r61['best_iso_params'].values()]
            if bmaxes:
                from collections import Counter
                ISO_SHORT_BREADTH_MAX = Counter(bmaxes).most_common(1)[0][0]
                print(f"ISO_SHORT_BREADTH_MAX set to {ISO_SHORT_BREADTH_MAX*100:.0f}%")

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    breadth, avg_z, avg_rsi, btc_z = build_market_data(all_data)

    # Load checkpoint
    checkpoint = load_checkpoint()
    results = {}
    completed_combos = set()

    if checkpoint:
        results = checkpoint['results']
        completed_combos = set(checkpoint['completed_combos'])
        print(f"Resumed from checkpoint: {len(completed_combos)}/{len(COMBOS)} combos completed")

    # === GRID SEARCH ===
    total = len(COMBOS)
    done = len(completed_combos)

    mode_counts = {'none': 0, 'breakeven': 0, 'trail': 0}
    mode_done = {'none': 0, 'breakeven': 0, 'trail': 0}
    for m, _, _, _ in COMBOS:
        mode_counts[m] += 1
    for ck in completed_combos:
        m = ck.split('_')[0]
        if m in mode_done:
            mode_done[m] += 1

    print(f"\nStarting grid search: {done}/{total} combos done")
    print(f"  none: {mode_done['none']}/{mode_counts['none']}  "
          f"breakeven: {mode_done['breakeven']}/{mode_counts['breakeven']}  "
          f"trail: {mode_done['trail']}/{mode_counts['trail']}")

    start_time = _time.time()
    combos_this_run = 0

    for combo_idx, (trail_mode, sl, act, dist) in enumerate(COMBOS):
        combo_key = f"{trail_mode}_{sl}_{act}_{dist}"
        if combo_key in completed_combos:
            continue

        if _shutdown:
            print(f"\nShutdown. Saving checkpoint at {done}/{total}...")
            save_checkpoint(results, list(completed_combos))
            sys.exit(0)

        combo_results = {}
        for coin in COINS:
            if coin not in all_data:
                continue
            r = run_backtest(
                all_data[coin],
                OPTIMAL_LONG_STRAT.get(coin, 'vwap_reversion'),
                OPTIMAL_SHORT_STRAT.get(coin, 'short_mean_rev'),
                OPTIMAL_ISO_SHORT_STRAT.get(coin),
                breadth, avg_z, avg_rsi, btc_z,
                initial_sl=sl, trail_mode=trail_mode,
                trail_activation=act, trail_distance=dist)
            if r:
                combo_results[coin] = r

        results[combo_key] = {
            'trail_mode': trail_mode, 'initial_sl': sl,
            'trail_activation': act, 'trail_distance': dist,
            'coins': combo_results,
        }
        completed_combos.add(combo_key)
        done += 1
        combos_this_run += 1
        mode_done[trail_mode] += 1

        # Progress every combo
        elapsed = _time.time() - start_time
        rate = combos_this_run / elapsed if elapsed > 0 else 0
        remaining = total - done
        eta_min = (remaining / rate / 60) if rate > 0 else 0

        print(f"  [{done}/{total}] ({done/total*100:.0f}%) {trail_mode} "
              f"[{mode_done[trail_mode]}/{mode_counts[trail_mode]}] "
              f"sl={sl} act={act} dist={dist} -> {len(combo_results)} coins "
              f"| {rate:.1f}/s ETA:{eta_min:.1f}m")

        # Checkpoint every 20 combos
        if done % 20 == 0:
            save_checkpoint(results, list(completed_combos))

    # Final checkpoint
    save_checkpoint(results, list(completed_combos))

    # === ANALYSIS ===
    print(f"\n{'='*90}")
    print("RESULTS SUMMARY")
    print(f"{'='*90}")

    # Current baseline: sl=0.005, none
    baseline_key = 'none_0.005_0.0_0.0'
    if baseline_key in results:
        bl = results[baseline_key]['coins']
        b_wrs = [r['wr'] for r in bl.values() if r['trades'] > 0]
        b_pfs = [r['pf'] for r in bl.values() if r['trades'] > 0]
        b_pnls = [r['pnl'] for r in bl.values()]
        print(f"\n  CURRENT BASELINE (sl=0.5%, none):")
        print(f"    Avg WR: {np.mean(b_wrs):.1f}%  Avg PF: {np.mean(b_pfs):.2f}  Avg P&L: {np.mean(b_pnls):+.1f}%")

    # Best combo per mode
    best_by_mode = {}
    for combo_key, data in results.items():
        mode = data['trail_mode']
        coins_r = data['coins']
        if not coins_r:
            continue

        wrs = [r['wr'] for r in coins_r.values() if r['trades'] > 0]
        pfs = [r['pf'] for r in coins_r.values() if r['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        nets = [r['net_impact'] for r in coins_r.values() if r['trail_exits'] > 0]

        if not pfs:
            continue

        avg_wr = np.mean(wrs)
        avg_pf = np.mean(pfs)
        avg_pnl = np.mean(pnls)
        avg_net = np.mean(nets) if nets else 0

        score = avg_pf * (avg_wr / 100) ** 0.5

        if mode not in best_by_mode or score > best_by_mode[mode]['score']:
            best_by_mode[mode] = {
                'combo_key': combo_key,
                'score': score,
                'avg_wr': avg_wr,
                'avg_pf': avg_pf,
                'avg_pnl': avg_pnl,
                'avg_net_impact': avg_net,
                'initial_sl': data['initial_sl'],
                'trail_activation': data['trail_activation'],
                'trail_distance': data['trail_distance'],
            }

    for mode in ['none', 'breakeven', 'trail']:
        if mode in best_by_mode:
            b = best_by_mode[mode]
            print(f"\n  BEST {mode.upper()}:")
            print(f"    Params: sl={b['initial_sl']}, act={b['trail_activation']}, dist={b['trail_distance']}")
            print(f"    Avg WR: {b['avg_wr']:.1f}%  Avg PF: {b['avg_pf']:.2f}  Avg P&L: {b['avg_pnl']:+.1f}%")
            if mode != 'none':
                print(f"    Net impact: {b['avg_net_impact']:+.1f} (saves - prematures)")

    # === BEST OVERALL ===
    best_overall_score = -999
    best_overall = None
    for combo_key, data in results.items():
        coins_r = data['coins']
        if len(coins_r) < len(all_data) * 0.5:
            continue
        pfs = [r['pf'] for r in coins_r.values() if r['trades'] > 0]
        wrs = [r['wr'] for r in coins_r.values() if r['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        if not pfs:
            continue
        score = np.mean(pfs) * (np.mean(wrs) / 100) ** 0.5
        if score > best_overall_score:
            best_overall_score = score
            best_overall = {
                'combo_key': combo_key,
                'trail_mode': data['trail_mode'],
                'initial_sl': data['initial_sl'],
                'trail_activation': data['trail_activation'],
                'trail_distance': data['trail_distance'],
                'avg_wr': np.mean(wrs),
                'avg_pf': np.mean(pfs),
                'avg_pnl': np.mean(pnls),
            }

    print(f"\n{'='*90}")
    print("BEST OVERALL (universal params)")
    print(f"{'='*90}")
    if best_overall:
        print(f"  Mode: {best_overall['trail_mode']}")
        print(f"  Params: sl={best_overall['initial_sl']}, act={best_overall['trail_activation']}, "
              f"dist={best_overall['trail_distance']}")
        print(f"  Avg WR: {best_overall['avg_wr']:.1f}%  Avg PF: {best_overall['avg_pf']:.2f}  "
              f"Avg P&L: {best_overall['avg_pnl']:+.1f}%")

    # === BEST PER COIN ===
    print(f"\n{'='*90}")
    print("BEST STOP LOSS PARAMS PER COIN")
    print(f"{'='*90}")

    baseline_coins = results.get(baseline_key, {}).get('coins', {})
    best_per_coin = {}

    for coin in COINS:
        if coin not in all_data:
            continue

        baseline_pnl = baseline_coins.get(coin, {}).get('pnl', 0)
        best_score = -999
        best_combo = None

        for combo_key, data in results.items():
            if coin not in data['coins']:
                continue
            r = data['coins'][coin]
            if r['trades'] < 3:
                continue
            score = r['pf'] * (r['wr'] / 100) ** 0.5
            if score > best_score:
                best_score = score
                best_combo = {
                    'trail_mode': data['trail_mode'],
                    'initial_sl': data['initial_sl'],
                    'trail_activation': data['trail_activation'],
                    'trail_distance': data['trail_distance'],
                    'wr': r['wr'],
                    'pf': r['pf'],
                    'pnl': r['pnl'],
                    'trail_exits': r['trail_exits'],
                    'trail_saves': r['trail_saves'],
                    'trail_premature': r['trail_premature'],
                    'net_impact': r['net_impact'],
                }

        if best_combo:
            best_per_coin[coin] = best_combo
            delta = best_combo['pnl'] - baseline_pnl
            print(f"  {coin:<6} {best_combo['trail_mode']:<11} sl={best_combo['initial_sl']:.3f} "
                  f"act={best_combo['trail_activation']:.3f} dist={best_combo['trail_distance']:.3f} | "
                  f"WR:{best_combo['wr']:.0f}% PF:{best_combo['pf']:.2f} P&L:{best_combo['pnl']:+.1f}% "
                  f"(vs base:{delta:+.1f}%)")

    # === SL SIZE ANALYSIS ===
    print(f"\n{'='*90}")
    print("STOP LOSS SIZE ANALYSIS (none mode only)")
    print(f"{'='*90}")

    for sl in INITIAL_SLS:
        key = f"none_{sl}_0.0_0.0"
        if key not in results:
            continue
        coins_r = results[key]['coins']
        if not coins_r:
            continue
        wrs = [r['wr'] for r in coins_r.values() if r['trades'] > 0]
        pfs = [r['pf'] for r in coins_r.values() if r['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        total_trades = sum(r['trades'] for r in coins_r.values())
        current = " ← CURRENT" if sl == 0.005 else ""
        print(f"  sl={sl*100:.1f}%: WR={np.mean(wrs):.1f}% PF={np.mean(pfs):.2f} "
              f"P&L={np.mean(pnls):+.1f}% trades={total_trades}{current}")

    # Save results
    save_data = {
        'best_by_mode': best_by_mode,
        'best_overall': best_overall,
        'best_per_coin': best_per_coin,
        'baseline_key': baseline_key,
        'total_combos': len(COMBOS),
        'total_backtests': len(COMBOS) * len(all_data),
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to {RESULTS_FILE}")

    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("Checkpoint removed (clean finish)")


if __name__ == "__main__":
    main()
