#!/usr/bin/env python3
"""
RUN8.1 - Take Profit Optimization

Test whether adding a fixed TP target improves performance vs signal-only exits.

TP Modes:
  none          — current behavior: signal exits only (baseline)
  tp_immediate  — exit when pnl >= TP%, fires even during MIN_HOLD period
  tp_after_hold — exit when pnl >= TP%, only after MIN_HOLD candles

Grid:
  tp_mode:   [none, tp_immediate, tp_after_hold]
  tp_target: [0.003, 0.005, 0.007, 0.010, 0.015, 0.020]

  none:         1 combo
  tp_immediate: 6 combos
  tp_after_hold: 6 combos
  Total:        13 combos × 18 coins = 234 backtests

Hardcoded from RUN7: STOP_LOSS = 0.003, trail_mode = 'none'

Shadow tracking: when TP fires, track what baseline (no TP) would have done.
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run8_1_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run8_1_results.json'

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
STOP_LOSS = 0.003  # Hardcoded from RUN7

BREADTH_LONG_MAX = 0.20
BREADTH_SHORT_MIN = 0.50
ISO_SHORT_BREADTH_MAX = 0.20

ISO_SHORT_PARAMS = {
    'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
    'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
    'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8,
}

# TP parameter grid
TP_TARGETS = [0.003, 0.005, 0.007, 0.010, 0.015, 0.020]

# Build combo list: (tp_mode, tp_target)
COMBOS = []
COMBOS.append(('none', 0.0))
for tp in TP_TARGETS:
    COMBOS.append(('tp_immediate', tp))
for tp in TP_TARGETS:
    COMBOS.append(('tp_after_hold', tp))

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
                 tp_mode='none', tp_target=0.0):
    """
    Run combined backtest with take profit optimization.

    Exit priority:
      1. Stop Loss:   pnl <= -0.3%                     [always first]
      2. Take Profit: pnl >= +tp_target                 [immediate or after MIN_HOLD]
      3. Signal exits: (SMA, Z0) after MIN_HOLD in profit
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
    cooldown = 0
    candles_held = 0
    entry_type = None

    # TP tracking
    tp_exits = 0
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
            price_pnl = (price - entry_price) / entry_price

            exited = False
            exit_reason = None

            # 1. Stop Loss (always first)
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'dir': 'long',
                               'type': 'loss', 'reason': 'SL'})
                exited = True
                exit_reason = 'SL'

            # 2. Take Profit
            if not exited and tp_mode != 'none' and tp_target > 0:
                tp_can_fire = False
                if tp_mode == 'tp_immediate':
                    tp_can_fire = True
                elif tp_mode == 'tp_after_hold' and candles_held >= MIN_HOLD:
                    tp_can_fire = True

                if tp_can_fire and price_pnl >= tp_target:
                    # Book profit at exactly tp_target (limit order assumption)
                    profit = balance * RISK * tp_target * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': tp_target * LEVERAGE * 100, 'dir': 'long',
                                   'type': 'win', 'reason': 'TP'})
                    exited = True
                    exit_reason = 'TP'
                    tp_exits += 1

                    # Start shadow: what would baseline (no TP) have done?
                    shadow_positions.append({
                        'entry_price': entry_price,
                        'exit_pnl': tp_target,
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
            price_pnl = (entry_price - price) / entry_price

            exited = False
            exit_reason = None

            # 1. Stop Loss
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'dir': 'short',
                               'type': 'loss', 'reason': 'SL', 'sub': entry_type})
                exited = True
                exit_reason = 'SL'

            # 2. Take Profit
            if not exited and tp_mode != 'none' and tp_target > 0:
                tp_can_fire = False
                if tp_mode == 'tp_immediate':
                    tp_can_fire = True
                elif tp_mode == 'tp_after_hold' and candles_held >= MIN_HOLD:
                    tp_can_fire = True

                if tp_can_fire and price_pnl >= tp_target:
                    profit = balance * RISK * tp_target * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': tp_target * LEVERAGE * 100, 'dir': 'short',
                                   'type': 'win', 'reason': 'TP', 'sub': entry_type})
                    exited = True
                    exit_reason = 'TP'
                    tp_exits += 1

                    shadow_positions.append({
                        'entry_price': entry_price,
                        'exit_pnl': tp_target,
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

        # === Update shadow positions (counterfactual tracking) ===
        for sp in shadow_positions:
            if sp['shadow_outcome'] is not None:
                continue
            candles_since = row_idx - sp['start_row_idx']
            if candles_since >= SHADOW_MAX_CANDLES:
                sp['shadow_outcome'] = 'TIMEOUT'
                sp['shadow_pnl'] = None
                continue
            if sp['dir'] == 'long':
                shadow_pnl = (price - sp['entry_price']) / sp['entry_price']
                # Would baseline have hit SL?
                if shadow_pnl <= -STOP_LOSS:
                    sp['shadow_outcome'] = 'TP_SAVE'
                    sp['shadow_pnl'] = -STOP_LOSS
                # Would baseline have signal-exited?
                elif shadow_pnl > 0 and (candles_since + sp['candles_held_at_exit']) >= MIN_HOLD:
                    if row['c'] > row['sma20'] or row['z'] > 0.5:
                        if shadow_pnl > sp['exit_pnl']:
                            sp['shadow_outcome'] = 'TP_PREMATURE'
                        else:
                            sp['shadow_outcome'] = 'TP_PARTIAL_SAVE'
                        sp['shadow_pnl'] = shadow_pnl
            else:  # short
                shadow_pnl = (sp['entry_price'] - price) / sp['entry_price']
                if shadow_pnl <= -STOP_LOSS:
                    sp['shadow_outcome'] = 'TP_SAVE'
                    sp['shadow_pnl'] = -STOP_LOSS
                elif shadow_pnl > 0 and (candles_since + sp['candles_held_at_exit']) >= MIN_HOLD:
                    if price < row['sma20'] or row['z'] < ISO_SHORT_PARAMS['exit_z']:
                        if shadow_pnl > sp['exit_pnl']:
                            sp['shadow_outcome'] = 'TP_PREMATURE'
                        else:
                            sp['shadow_outcome'] = 'TP_PARTIAL_SAVE'
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

    # Shadow analysis
    tp_saves = 0
    tp_partial_saves = 0
    tp_premature = 0

    for sp in shadow_positions:
        if sp['shadow_outcome'] == 'TP_SAVE':
            tp_saves += 1
        elif sp['shadow_outcome'] == 'TP_PARTIAL_SAVE':
            tp_partial_saves += 1
        elif sp['shadow_outcome'] == 'TP_PREMATURE':
            tp_premature += 1

    net_impact = tp_saves + tp_partial_saves - tp_premature

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
        'tp_exits': tp_exits,
        'tp_saves': tp_saves,
        'tp_partial_saves': tp_partial_saves,
        'tp_premature': tp_premature,
        'net_impact': net_impact,
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
    print("RUN8.1 - TAKE PROFIT OPTIMIZATION")
    print("=" * 90)

    n_none = 1
    n_imm = len(TP_TARGETS)
    n_hold = len(TP_TARGETS)
    print(f"Modes: none ({n_none}), tp_immediate ({n_imm}), tp_after_hold ({n_hold})")
    print(f"Total: {len(COMBOS)} combos x {len(COINS)} coins = {len(COMBOS)*len(COINS)} backtests")
    print(f"Hardcoded: SL=0.3%, trail_mode=none")
    print(f"TP targets: {[f'{t*100:.1f}%' for t in TP_TARGETS]}")
    print(f"R:R ratios (vs SL=0.3%): {[f'{t/STOP_LOSS:.1f}:1' for t in TP_TARGETS]}")
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

    print(f"\nStarting grid search: {done}/{total} combos done")

    start_time = _time.time()
    combos_this_run = 0

    for combo_idx, (tp_mode, tp_target) in enumerate(COMBOS):
        combo_key = f"{tp_mode}_{tp_target}"
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
                tp_mode=tp_mode, tp_target=tp_target)
            if r:
                combo_results[coin] = r

        results[combo_key] = {
            'tp_mode': tp_mode, 'tp_target': tp_target,
            'coins': combo_results,
        }
        completed_combos.add(combo_key)
        done += 1
        combos_this_run += 1

        # Progress
        elapsed = _time.time() - start_time
        rate = combos_this_run / elapsed if elapsed > 0 else 0
        remaining = total - done
        eta_min = (remaining / rate / 60) if rate > 0 else 0

        # Summary for this combo
        if combo_results:
            avg_pf = np.mean([r['pf'] for r in combo_results.values() if r['trades'] > 0])
            avg_pnl = np.mean([r['pnl'] for r in combo_results.values()])
        else:
            avg_pf = 0
            avg_pnl = 0

        tp_str = f"tp={tp_target*100:.1f}%" if tp_mode != 'none' else "no TP"
        print(f"  [{done}/{total}] ({done/total*100:.0f}%) {tp_mode} {tp_str} -> "
              f"{len(combo_results)} coins avgPF={avg_pf:.2f} avgP&L={avg_pnl:+.1f}% "
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

    # Baseline: no TP
    baseline_key = 'none_0.0'
    if baseline_key in results:
        bl = results[baseline_key]['coins']
        b_wrs = [r['wr'] for r in bl.values() if r['trades'] > 0]
        b_pfs = [r['pf'] for r in bl.values() if r['trades'] > 0]
        b_pnls = [r['pnl'] for r in bl.values()]
        print(f"\n  BASELINE (SL=0.3%, no TP):")
        print(f"    Avg WR: {np.mean(b_wrs):.1f}%  Avg PF: {np.mean(b_pfs):.2f}  "
              f"Avg P&L: {np.mean(b_pnls):+.1f}%")

    # Results table by TP mode and target
    print(f"\n  {'Mode':<16} {'TP%':<8} {'R:R':<8} {'Avg WR':<10} {'Avg PF':<10} "
          f"{'Avg P&L':<12} {'TP#':<6} {'Save':<6} {'Part':<6} {'Prem':<6} {'Net'}")
    print(f"  {'-'*110}")

    for combo_key, data in sorted(results.items()):
        coins_r = data['coins']
        if not coins_r:
            continue
        wrs = [r['wr'] for r in coins_r.values() if r['trades'] > 0]
        pfs = [r['pf'] for r in coins_r.values() if r['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        tp_exits_total = sum(r['tp_exits'] for r in coins_r.values())
        saves_total = sum(r['tp_saves'] for r in coins_r.values())
        partial_total = sum(r['tp_partial_saves'] for r in coins_r.values())
        premature_total = sum(r['tp_premature'] for r in coins_r.values())
        net_total = saves_total + partial_total - premature_total

        tp_pct = f"{data['tp_target']*100:.1f}%" if data['tp_mode'] != 'none' else "-"
        rr = f"{data['tp_target']/STOP_LOSS:.1f}:1" if data['tp_target'] > 0 else "-"

        print(f"  {data['tp_mode']:<16} {tp_pct:<8} {rr:<8} {np.mean(wrs):<10.1f} "
              f"{np.mean(pfs):<10.2f} {np.mean(pnls):<12.1f} {tp_exits_total:<6} "
              f"{saves_total:<6} {partial_total:<6} {premature_total:<6} {net_total:+d}")

    # === BEST BY MODE ===
    best_by_mode = {}
    for combo_key, data in results.items():
        mode = data['tp_mode']
        coins_r = data['coins']
        if not coins_r:
            continue
        wrs = [r['wr'] for r in coins_r.values() if r['trades'] > 0]
        pfs = [r['pf'] for r in coins_r.values() if r['trades'] > 0]
        pnls = [r['pnl'] for r in coins_r.values()]
        if not pfs:
            continue
        avg_pf = np.mean(pfs)
        avg_wr = np.mean(wrs)
        avg_pnl = np.mean(pnls)
        score = avg_pf * (avg_wr / 100) ** 0.5

        if mode not in best_by_mode or score > best_by_mode[mode]['score']:
            best_by_mode[mode] = {
                'combo_key': combo_key,
                'score': score,
                'avg_wr': avg_wr,
                'avg_pf': avg_pf,
                'avg_pnl': avg_pnl,
                'tp_target': data['tp_target'],
            }

    print(f"\n{'='*90}")
    print("BEST BY MODE")
    print(f"{'='*90}")
    for mode in ['none', 'tp_immediate', 'tp_after_hold']:
        if mode in best_by_mode:
            b = best_by_mode[mode]
            tp_str = f"tp={b['tp_target']*100:.1f}%" if mode != 'none' else "no TP"
            print(f"\n  BEST {mode.upper()} ({tp_str}):")
            print(f"    Avg WR: {b['avg_wr']:.1f}%  Avg PF: {b['avg_pf']:.2f}  Avg P&L: {b['avg_pnl']:+.1f}%")

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
                'tp_mode': data['tp_mode'],
                'tp_target': data['tp_target'],
                'avg_wr': np.mean(wrs),
                'avg_pf': np.mean(pfs),
                'avg_pnl': np.mean(pnls),
            }

    print(f"\n{'='*90}")
    print("BEST OVERALL (universal params)")
    print(f"{'='*90}")
    if best_overall:
        tp_str = f"tp={best_overall['tp_target']*100:.1f}%" if best_overall['tp_mode'] != 'none' else "no TP"
        print(f"  Mode: {best_overall['tp_mode']} ({tp_str})")
        print(f"  Avg WR: {best_overall['avg_wr']:.1f}%  Avg PF: {best_overall['avg_pf']:.2f}  "
              f"Avg P&L: {best_overall['avg_pnl']:+.1f}%")

        # Compare vs baseline
        if baseline_key in results:
            bl = results[baseline_key]['coins']
            bl_pf = np.mean([r['pf'] for r in bl.values() if r['trades'] > 0])
            bl_pnl = np.mean([r['pnl'] for r in bl.values()])
            print(f"\n  vs Baseline: PF {bl_pf:.2f}->{best_overall['avg_pf']:.2f} "
                  f"({best_overall['avg_pf']-bl_pf:+.2f})  "
                  f"P&L {bl_pnl:+.1f}%->{best_overall['avg_pnl']:+.1f}% "
                  f"({best_overall['avg_pnl']-bl_pnl:+.1f}%)")

    # === BEST PER COIN ===
    print(f"\n{'='*90}")
    print("BEST TP PARAMS PER COIN")
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
                    'tp_mode': data['tp_mode'],
                    'tp_target': data['tp_target'],
                    'wr': r['wr'],
                    'pf': r['pf'],
                    'pnl': r['pnl'],
                    'tp_exits': r['tp_exits'],
                    'tp_saves': r['tp_saves'],
                    'tp_partial_saves': r['tp_partial_saves'],
                    'tp_premature': r['tp_premature'],
                    'net_impact': r['net_impact'],
                }

        if best_combo:
            best_per_coin[coin] = best_combo
            delta = best_combo['pnl'] - baseline_pnl
            tp_str = f"tp={best_combo['tp_target']*100:.1f}%" if best_combo['tp_mode'] != 'none' else "none"
            print(f"  {coin:<6} {best_combo['tp_mode']:<16} {tp_str:<10} | "
                  f"WR:{best_combo['wr']:.0f}% PF:{best_combo['pf']:.2f} P&L:{best_combo['pnl']:+.1f}% "
                  f"(vs base:{delta:+.1f}%) net_impact:{best_combo['net_impact']:+d}")

    # === EARLY STOP CHECK ===
    print(f"\n{'='*90}")
    print("EARLY STOP CHECK")
    print(f"{'='*90}")

    if baseline_key in results and best_overall:
        bl_coins = results[baseline_key]['coins']
        best_coins = results[best_overall['combo_key']]['coins']

        coins_tp_helps = 0
        coins_tp_hurts = 0
        for coin in COINS:
            if coin in bl_coins and coin in best_coins:
                if best_coins[coin]['pnl'] > bl_coins[coin]['pnl']:
                    coins_tp_helps += 1
                else:
                    coins_tp_hurts += 1

        # Check net_impact across all TP combos
        all_net_impacts = []
        for combo_key, data in results.items():
            if data['tp_mode'] == 'none':
                continue
            net = sum(r['net_impact'] for r in data['coins'].values())
            all_net_impacts.append(net)

        avg_net = np.mean(all_net_impacts) if all_net_impacts else 0
        print(f"  Best TP helps {coins_tp_helps}/{coins_tp_helps+coins_tp_hurts} coins vs baseline")
        print(f"  Average net_impact across all TP combos: {avg_net:+.1f}")

        if best_overall['tp_mode'] == 'none':
            print(f"\n  VERDICT: TP does NOT help. Best overall is baseline (no TP).")
            print(f"  RECOMMENDATION: Stop early. Do not proceed to RUN8.2.")
        elif avg_net < 0 and coins_tp_hurts > coins_tp_helps:
            print(f"\n  VERDICT: TP likely hurts. Net impact negative, hurts more coins than helps.")
            print(f"  RECOMMENDATION: Stop early. Do not proceed to RUN8.2.")
        else:
            print(f"\n  VERDICT: TP shows promise. Proceed to RUN8.2 for walk-forward validation.")

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
