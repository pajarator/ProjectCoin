#!/usr/bin/env python3
"""
RUN10.1 - Scalp Indicator Discovery

Goal: Find additional indicators that correlate with WINNING scalp trades,
so we can pre-filter and skip likely losers.

Method:
  1. Run all 3 scalp strategies (best RUN9 params) across 18 coins
  2. At each scalp entry, snapshot ~30+ indicators (1m + 15m level)
  3. Label each trade WIN or LOSS
  4. Statistical analysis: correlation, mutual information, decision tree
  5. Find optimal filter thresholds to boost win rate

Checkpoints, progress bar, SIGINT handling per project requirements.
"""
import pandas as pd
import numpy as np
import json
import os
import signal
import sys
import time as _time
from collections import defaultdict

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run10_1_checkpoint.json'
RESULTS_FILE = '/home/scamarena/ProjectCoin/run10_1_results.json'
TRADES_FILE = '/home/scamarena/ProjectCoin/run10_1_trades.json'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

# Best universal params from RUN9
SCALP_SL = 0.0010   # 0.10%
SCALP_TP = 0.0020   # 0.20%
VOL_SPIKE_MULT = 3.5
RSI_EXTREME = 20
STOCH_EXTREME = 5
BB_SQUEEZE_FACTOR = 0.4

LEVERAGE = 5

_shutdown = False

def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, saving checkpoint...")

signal.signal(signal.SIGINT, _sigint_handler)


def load_csv(name, tf):
    path = f"{DATA_CACHE_DIR}/{name}_USDT_{tf}_5months.csv"
    if os.path.exists(path):
        return pd.read_csv(path, index_col=0, parse_dates=True)
    return None


def calc_1m_indicators(df):
    """Extended 1m indicators for discovery."""
    df = df.copy()

    # RSI
    delta = df['c'].diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df['rsi'] = 100 - (100 / (1 + rs))
    df['rsi_prev'] = df['rsi'].shift(1)
    df['rsi_slope'] = df['rsi'] - df['rsi'].shift(3)  # RSI momentum over 3 bars

    # Stochastic
    lowest_low = df['l'].rolling(14).min()
    highest_high = df['h'].rolling(14).max()
    df['stoch_k'] = 100 * ((df['c'] - lowest_low) / (highest_high - lowest_low))
    df['stoch_d'] = df['stoch_k'].rolling(3).mean()
    df['stoch_k_prev'] = df['stoch_k'].shift(1)
    df['stoch_d_prev'] = df['stoch_d'].shift(1)

    # Bollinger Bands
    df['bb_sma'] = df['c'].rolling(20).mean()
    df['bb_std'] = df['c'].rolling(20).std()
    df['bb_upper'] = df['bb_sma'] + 2 * df['bb_std']
    df['bb_lower'] = df['bb_sma'] - 2 * df['bb_std']
    df['bb_width'] = df['bb_upper'] - df['bb_lower']
    df['bb_width_avg'] = df['bb_width'].rolling(20).mean()
    # BB %B: where price is within bands (0=lower, 1=upper)
    bb_range = df['bb_upper'] - df['bb_lower']
    df['bb_pctb'] = (df['c'] - df['bb_lower']) / bb_range.replace(0, np.nan)

    # Volume
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['vol_ratio'] = df['v'] / df['vol_ma'].replace(0, np.nan)
    df['vol_trend'] = df['v'].rolling(5).mean() / df['v'].rolling(20).mean().replace(0, np.nan)

    # Price momentum / ROC
    df['roc_3'] = (df['c'] - df['c'].shift(3)) / df['c'].shift(3) * 100
    df['roc_5'] = (df['c'] - df['c'].shift(5)) / df['c'].shift(5) * 100
    df['roc_10'] = (df['c'] - df['c'].shift(10)) / df['c'].shift(10) * 100

    # ATR (14-period)
    high_low = df['h'] - df['l']
    high_close = abs(df['h'] - df['c'].shift())
    low_close = abs(df['l'] - df['c'].shift())
    tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    df['atr'] = tr.rolling(14).mean()
    df['atr_ratio'] = df['atr'] / df['c'] * 100  # ATR as % of price

    # Candle characteristics
    body = abs(df['c'] - df['o'])
    full_range = (df['h'] - df['l']).replace(0, np.nan)
    df['body_ratio'] = body / full_range  # 0=doji, 1=marubozu
    df['upper_wick'] = (df['h'] - df[['c','o']].max(axis=1)) / full_range
    df['lower_wick'] = (df[['c','o']].min(axis=1) - df['l']) / full_range
    df['is_green'] = (df['c'] > df['o']).astype(float)

    # Recent candle pattern (last 3)
    df['green_count_3'] = df['is_green'].rolling(3).sum()
    df['avg_body_3'] = body.rolling(3).mean() / df['c'] * 100

    # SMA distance
    df['sma9'] = df['c'].rolling(9).mean()
    df['sma20'] = df['c'].rolling(20).mean()
    df['dist_sma9'] = (df['c'] - df['sma9']) / df['sma9'] * 100
    df['dist_sma20'] = (df['c'] - df['sma20']) / df['sma20'] * 100

    # Z-score (1m level)
    df['z_1m'] = (df['c'] - df['sma20']) / df['bb_std'].replace(0, np.nan)

    # EMA 5/12 spread (micro trend)
    df['ema5'] = df['c'].ewm(span=5).mean()
    df['ema12'] = df['c'].ewm(span=12).mean()
    df['ema_spread'] = (df['ema5'] - df['ema12']) / df['c'] * 100

    # MFI (Money Flow Index) - simplified
    typical = (df['h'] + df['l'] + df['c']) / 3
    money_flow = typical * df['v']
    pos_flow = money_flow.where(typical > typical.shift(), 0).rolling(14).sum()
    neg_flow = money_flow.where(typical <= typical.shift(), 0).rolling(14).sum()
    mfr = pos_flow / neg_flow.replace(0, np.nan)
    df['mfi'] = 100 - (100 / (1 + mfr))

    # OBV slope (On Balance Volume trend)
    obv_sign = np.sign(df['c'].diff())
    df['obv'] = (obv_sign * df['v']).cumsum()
    df['obv_slope'] = (df['obv'] - df['obv'].shift(5)) / df['obv'].shift(5).abs().replace(0, np.nan) * 100

    # Williams %R
    df['williams_r'] = -100 * (highest_high - df['c']) / (highest_high - lowest_low).replace(0, np.nan)

    # Consecutive up/down bars
    up = (df['c'] > df['c'].shift(1)).astype(int)
    down = (df['c'] < df['c'].shift(1)).astype(int)
    # count consecutive ups
    groups_up = (up != up.shift()).cumsum()
    df['consec_up'] = up.groupby(groups_up).cumsum()
    groups_down = (down != down.shift()).cumsum()
    df['consec_down'] = down.groupby(groups_down).cumsum()

    # Spread: high-low as % of close (intrabar volatility)
    df['spread_pct'] = (df['h'] - df['l']) / df['c'] * 100

    # Volume delta proxy: (close - low) / (high - low) * volume
    # > 0.5 means buying pressure, < 0.5 selling pressure
    df['vol_delta_proxy'] = ((df['c'] - df['l']) / full_range) * 2 - 1  # -1 to +1

    return df


def calc_15m_indicators(df):
    """15m indicators."""
    df = df.copy()
    df['sma20'] = df['c'].rolling(20).mean()
    df['sma9'] = df['c'].rolling(9).mean()
    df['std20'] = df['c'].rolling(20).std()
    df['z'] = (df['c'] - df['sma20']) / df['std20'].replace(0, np.nan)
    df['bb_lo'] = df['sma20'] - 2 * df['std20']
    df['bb_hi'] = df['sma20'] + 2 * df['std20']
    df['vol_ma'] = df['v'].rolling(20).mean()

    delta = df['c'].diff()
    gain = delta.where(delta > 0, 0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    df['rsi'] = 100 - (100 / (1 + gain / loss))

    # ADX
    high_low = df['h'] - df['l']
    plus_dm = high_low.where((df['h'] - df['h'].shift()) > (df['l'].shift() - df['l']), 0)
    minus_dm = high_low.where((df['l'].shift() - df['l']) > (df['h'] - df['h'].shift()), 0)
    atr = pd.concat([high_low, abs(df['h'] - df['c'].shift()), abs(df['l'] - df['c'].shift())], axis=1).max(axis=1).rolling(14).mean()
    plus_di = 100 * (plus_dm.rolling(14).mean() / atr.replace(0, np.nan))
    minus_di = 100 * (minus_dm.rolling(14).mean() / atr.replace(0, np.nan))
    dx = 100 * abs(plus_di - minus_di) / (plus_di + minus_di).replace(0, np.nan)
    df['adx'] = dx.rolling(14).mean()

    # SMA20 slope (trend direction)
    df['sma20_slope'] = (df['sma20'] - df['sma20'].shift(3)) / df['sma20'].shift(3) * 100

    # VWAP
    typical = (df['h'] + df['l'] + df['c']) / 3
    df['vwap'] = (typical * df['v']).rolling(20).sum() / df['v'].rolling(20).sum().replace(0, np.nan)
    df['dist_vwap'] = (df['c'] - df['vwap']) / df['vwap'] * 100

    return df


def build_market_breadth(all_15m):
    """Build market breadth from 15m data."""
    z_frames = {}
    for coin, df in all_15m.items():
        df_ind = calc_15m_indicators(df)
        z_frames[coin] = df_ind['z']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    avg_z = z_df.mean(axis=1)
    return breadth, avg_z


def scalp_entry_check(row_1m):
    """Check scalp entry. Returns (direction, strategy_name) or (None, None)."""
    if pd.isna(row_1m.get('rsi')) or pd.isna(row_1m.get('vol_ma')) or row_1m['vol_ma'] == 0:
        return None, None

    vol_r = row_1m['v'] / row_1m['vol_ma']

    # 1. scalp_vol_spike_rev
    if vol_r > VOL_SPIKE_MULT:
        if row_1m['rsi'] < RSI_EXTREME:
            return 'long', 'scalp_vol_spike_rev'
        if row_1m['rsi'] > 100 - RSI_EXTREME:
            return 'short', 'scalp_vol_spike_rev'

    # 2. scalp_stoch_cross
    if not pd.isna(row_1m.get('stoch_k')) and not pd.isna(row_1m.get('stoch_d')):
        k = row_1m['stoch_k']
        d = row_1m['stoch_d']
        k_prev = row_1m.get('stoch_k_prev', np.nan)
        d_prev = row_1m.get('stoch_d_prev', np.nan)
        if not pd.isna(k_prev) and not pd.isna(d_prev):
            if k_prev <= d_prev and k > d and k < STOCH_EXTREME and d < STOCH_EXTREME:
                return 'long', 'scalp_stoch_cross'
            if k_prev >= d_prev and k < d and k > 100 - STOCH_EXTREME and d > 100 - STOCH_EXTREME:
                return 'short', 'scalp_stoch_cross'

    # 3. scalp_bb_squeeze_break
    if (not pd.isna(row_1m.get('bb_width_avg')) and row_1m['bb_width_avg'] > 0
            and not pd.isna(row_1m.get('bb_upper'))):
        squeeze = row_1m['bb_width'] < row_1m['bb_width_avg'] * BB_SQUEEZE_FACTOR
        if squeeze and vol_r > 2.0:
            if row_1m['c'] > row_1m['bb_upper']:
                return 'long', 'scalp_bb_squeeze_break'
            if row_1m['c'] < row_1m['bb_lower']:
                return 'short', 'scalp_bb_squeeze_break'

    return None, None


def snapshot_indicators(row_1m, row_15m, direction, breadth_val, avg_z_val):
    """Capture all indicator values at the moment of scalp entry."""
    snap = {}

    # === 1m indicators ===
    for col in ['rsi', 'rsi_slope', 'stoch_k', 'stoch_d', 'bb_pctb', 'bb_width',
                'bb_width_avg', 'vol_ratio', 'vol_trend', 'roc_3', 'roc_5', 'roc_10',
                'atr_ratio', 'body_ratio', 'upper_wick', 'lower_wick', 'is_green',
                'green_count_3', 'avg_body_3', 'dist_sma9', 'dist_sma20', 'z_1m',
                'ema_spread', 'mfi', 'obv_slope', 'williams_r', 'consec_up',
                'consec_down', 'spread_pct', 'vol_delta_proxy']:
        val = row_1m.get(col, np.nan)
        snap[f'1m_{col}'] = float(val) if not pd.isna(val) else None

    # === 15m indicators (context) ===
    if row_15m is not None:
        for col in ['z', 'rsi', 'adx', 'sma20_slope', 'dist_vwap']:
            val = row_15m.get(col, np.nan)
            snap[f'15m_{col}'] = float(val) if not pd.isna(val) else None
    else:
        for col in ['z', 'rsi', 'adx', 'sma20_slope', 'dist_vwap']:
            snap[f'15m_{col}'] = None

    # === Market context ===
    snap['breadth'] = float(breadth_val) if not pd.isna(breadth_val) else None
    snap['avg_z'] = float(avg_z_val) if not pd.isna(avg_z_val) else None

    # === Direction-adjusted indicators ===
    # For longs, positive momentum is good. For shorts, negative momentum is good.
    # Normalize so "good direction" = positive
    sign = 1.0 if direction == 'long' else -1.0
    if snap['1m_roc_3'] is not None:
        snap['dir_roc_3'] = snap['1m_roc_3'] * sign
    else:
        snap['dir_roc_3'] = None
    if snap['1m_ema_spread'] is not None:
        snap['dir_ema_spread'] = snap['1m_ema_spread'] * sign
    else:
        snap['dir_ema_spread'] = None
    if snap['15m_sma20_slope'] is not None:
        snap['dir_15m_slope'] = snap['15m_sma20_slope'] * sign
    else:
        snap['dir_15m_slope'] = None

    return snap


def run_scalp_collection(coin, df_1m, df_15m_ind, breadth, avg_z):
    """
    Run scalp strategies on 1m data for one coin.
    For each trade, record entry indicators + outcome (win/loss).
    """
    trades = []
    i = 0
    n = len(df_1m)

    while i < n:
        if _shutdown:
            break

        row = df_1m.iloc[i]
        direction, strat_name = scalp_entry_check(row)

        if direction is None:
            i += 1
            continue

        entry_price = row['c']
        entry_ts = df_1m.index[i]

        # Get closest 15m row
        row_15m = None
        if df_15m_ind is not None:
            # Find the 15m candle that contains this timestamp
            mask_15m = df_15m_ind.index <= entry_ts
            if mask_15m.any():
                row_15m = df_15m_ind.loc[df_15m_ind.index[mask_15m][-1]]

        # Get breadth/avg_z at this time
        b_val = np.nan
        az_val = np.nan
        if breadth is not None:
            mask_b = breadth.index <= entry_ts
            if mask_b.any():
                b_val = breadth.iloc[mask_b.sum() - 1]
        if avg_z is not None:
            mask_az = avg_z.index <= entry_ts
            if mask_az.any():
                az_val = avg_z.iloc[mask_az.sum() - 1]

        # Snapshot indicators at entry
        snap = snapshot_indicators(row, row_15m, direction, b_val, az_val)

        # Simulate trade: check subsequent 1m candles for TP/SL
        outcome = None
        pnl_pct = 0
        exit_reason = None
        bars_held = 0

        for j in range(i + 1, min(i + 60, n)):  # max 60 bars (1 hour)
            p = df_1m.iloc[j]['c']
            if direction == 'long':
                pnl = (p - entry_price) / entry_price
            else:
                pnl = (entry_price - p) / entry_price

            bars_held = j - i

            if pnl >= SCALP_TP:
                outcome = 'win'
                pnl_pct = SCALP_TP * LEVERAGE * 100
                exit_reason = 'TP'
                break
            elif pnl <= -SCALP_SL:
                outcome = 'loss'
                pnl_pct = -SCALP_SL * LEVERAGE * 100
                exit_reason = 'SL'
                break
        else:
            # Timed out after 60 bars
            p = df_1m.iloc[min(i + 60, n - 1)]['c']
            if direction == 'long':
                pnl = (p - entry_price) / entry_price
            else:
                pnl = (entry_price - p) / entry_price
            pnl_pct = pnl * LEVERAGE * 100
            outcome = 'win' if pnl > 0 else 'loss'
            exit_reason = 'TIMEOUT'

        trade = {
            'coin': coin,
            'ts': str(entry_ts),
            'direction': direction,
            'strategy': strat_name,
            'outcome': outcome,
            'pnl_pct': round(pnl_pct, 4),
            'exit_reason': exit_reason,
            'bars_held': bars_held,
            'indicators': snap,
        }
        trades.append(trade)

        # Skip ahead past this trade (cooldown = bars held + 1)
        i += max(bars_held, 1) + 1
        continue

    return trades


def analyze_trades(all_trades):
    """Statistical analysis to find indicator correlations with wins."""
    if not all_trades:
        return {}

    # Build DataFrame from indicator snapshots
    rows = []
    for t in all_trades:
        row = dict(t['indicators'])
        row['outcome'] = 1 if t['outcome'] == 'win' else 0
        row['strategy'] = t['strategy']
        row['direction'] = t['direction']
        row['coin'] = t['coin']
        row['pnl_pct'] = t['pnl_pct']
        rows.append(row)

    df = pd.DataFrame(rows)

    # Get all indicator columns
    ind_cols = [c for c in df.columns if c.startswith(('1m_', '15m_', 'breadth', 'avg_z', 'dir_'))]

    results = {
        'total_trades': len(df),
        'wins': int(df['outcome'].sum()),
        'losses': int((1 - df['outcome']).sum()),
        'win_rate': float(df['outcome'].mean() * 100),
    }

    # === 1. Point-biserial correlation with outcome ===
    correlations = {}
    for col in ind_cols:
        valid = df[[col, 'outcome']].dropna()
        if len(valid) < 30:
            continue
        corr = valid[col].astype(float).corr(valid['outcome'].astype(float))
        if not pd.isna(corr):
            correlations[col] = round(corr, 4)

    # Sort by absolute correlation
    sorted_corr = sorted(correlations.items(), key=lambda x: abs(x[1]), reverse=True)
    results['correlations_top20'] = sorted_corr[:20]

    # === 2. Mean comparison: wins vs losses ===
    mean_diffs = {}
    for col in ind_cols:
        valid = df[[col, 'outcome']].dropna()
        if len(valid) < 30:
            continue
        win_mean = valid.loc[valid['outcome'] == 1, col].mean()
        loss_mean = valid.loc[valid['outcome'] == 0, col].mean()
        win_std = valid.loc[valid['outcome'] == 1, col].std()
        loss_std = valid.loc[valid['outcome'] == 0, col].std()
        if pd.isna(win_mean) or pd.isna(loss_mean):
            continue
        # Effect size (Cohen's d)
        pooled_std = np.sqrt((win_std**2 + loss_std**2) / 2)
        if pooled_std > 0:
            cohens_d = (win_mean - loss_mean) / pooled_std
        else:
            cohens_d = 0
        mean_diffs[col] = {
            'win_mean': round(float(win_mean), 4),
            'loss_mean': round(float(loss_mean), 4),
            'cohens_d': round(float(cohens_d), 4),
        }

    sorted_diffs = sorted(mean_diffs.items(), key=lambda x: abs(x[1]['cohens_d']), reverse=True)
    results['mean_diffs_top20'] = sorted_diffs[:20]

    # === 3. Per-strategy analysis ===
    strat_analysis = {}
    for strat in df['strategy'].unique():
        sdf = df[df['strategy'] == strat]
        strat_analysis[strat] = {
            'trades': len(sdf),
            'win_rate': round(float(sdf['outcome'].mean() * 100), 1),
        }
        # Top correlations for this strategy
        strat_corrs = {}
        for col in ind_cols:
            valid = sdf[[col, 'outcome']].dropna()
            if len(valid) < 20:
                continue
            corr = valid[col].astype(float).corr(valid['outcome'].astype(float))
            if not pd.isna(corr):
                strat_corrs[col] = round(corr, 4)
        sorted_sc = sorted(strat_corrs.items(), key=lambda x: abs(x[1]), reverse=True)
        strat_analysis[strat]['top_correlations'] = sorted_sc[:10]

    results['per_strategy'] = strat_analysis

    # === 4. Quintile analysis for top indicators ===
    # For the top correlated indicators, show win rate by quintile
    quintile_analysis = {}
    top_indicators = [c for c, _ in sorted_corr[:15]]
    for col in top_indicators:
        valid = df[[col, 'outcome']].dropna()
        if len(valid) < 50:
            continue
        try:
            valid['quintile'] = pd.qcut(valid[col].astype(float), 5, labels=False, duplicates='drop')
            qstats = valid.groupby('quintile').agg(
                count=('outcome', 'count'),
                win_rate=('outcome', 'mean'),
                avg_val=(col, 'mean')
            ).reset_index()
            quintile_analysis[col] = [
                {
                    'quintile': int(row['quintile']),
                    'count': int(row['count']),
                    'win_rate': round(float(row['win_rate'] * 100), 1),
                    'avg_val': round(float(row['avg_val']), 4),
                }
                for _, row in qstats.iterrows()
            ]
        except Exception:
            continue

    results['quintile_analysis'] = quintile_analysis

    # === 5. Simple filter candidates ===
    # Find thresholds that split trades into high/low WR groups
    filter_candidates = []
    for col in top_indicators:
        valid = df[[col, 'outcome']].dropna()
        if len(valid) < 50:
            continue
        vals = valid[col].astype(float)
        base_wr = valid['outcome'].mean()

        best_improvement = 0
        best_threshold = None
        best_direction = None
        best_kept_pct = 0
        best_filtered_wr = 0

        for pct in [20, 25, 30, 33, 40]:
            # Try filtering bottom N%
            thresh_lo = vals.quantile(pct / 100)
            kept = valid[vals >= thresh_lo]
            if len(kept) < 20:
                continue
            wr = kept['outcome'].mean()
            improvement = wr - base_wr
            if improvement > best_improvement:
                best_improvement = improvement
                best_threshold = float(thresh_lo)
                best_direction = 'keep_above'
                best_kept_pct = len(kept) / len(valid) * 100
                best_filtered_wr = wr * 100

            # Try filtering top N%
            thresh_hi = vals.quantile(1 - pct / 100)
            kept = valid[vals <= thresh_hi]
            if len(kept) < 20:
                continue
            wr = kept['outcome'].mean()
            improvement = wr - base_wr
            if improvement > best_improvement:
                best_improvement = improvement
                best_threshold = float(thresh_hi)
                best_direction = 'keep_below'
                best_kept_pct = len(kept) / len(valid) * 100
                best_filtered_wr = wr * 100

        if best_threshold is not None and best_improvement > 0.02:  # >2% WR improvement
            filter_candidates.append({
                'indicator': col,
                'threshold': round(best_threshold, 6),
                'direction': best_direction,
                'base_wr': round(float(base_wr * 100), 1),
                'filtered_wr': round(best_filtered_wr, 1),
                'improvement': round(float(best_improvement * 100), 1),
                'kept_pct': round(best_kept_pct, 1),
            })

    filter_candidates.sort(key=lambda x: x['improvement'], reverse=True)
    results['filter_candidates'] = filter_candidates[:15]

    # === 6. Combined filter test ===
    # Try combining the top 2-3 filters
    if len(filter_candidates) >= 2:
        combined_results = []
        # Test pairs
        for i_f in range(min(5, len(filter_candidates))):
            for j_f in range(i_f + 1, min(5, len(filter_candidates))):
                f1 = filter_candidates[i_f]
                f2 = filter_candidates[j_f]

                c1 = f1['indicator']
                c2 = f2['indicator']
                valid = df[[c1, c2, 'outcome']].dropna()
                if len(valid) < 50:
                    continue

                mask = pd.Series(True, index=valid.index)
                if f1['direction'] == 'keep_above':
                    mask &= valid[c1].astype(float) >= f1['threshold']
                else:
                    mask &= valid[c1].astype(float) <= f1['threshold']
                if f2['direction'] == 'keep_above':
                    mask &= valid[c2].astype(float) >= f2['threshold']
                else:
                    mask &= valid[c2].astype(float) <= f2['threshold']

                kept = valid[mask]
                if len(kept) < 20:
                    continue

                combo_wr = kept['outcome'].mean() * 100
                base_wr_val = valid['outcome'].mean() * 100
                combined_results.append({
                    'filters': [f1['indicator'], f2['indicator']],
                    'combo_wr': round(combo_wr, 1),
                    'base_wr': round(base_wr_val, 1),
                    'improvement': round(combo_wr - base_wr_val, 1),
                    'kept_trades': len(kept),
                    'total_trades': len(valid),
                    'kept_pct': round(len(kept) / len(valid) * 100, 1),
                })

        combined_results.sort(key=lambda x: x['improvement'], reverse=True)
        results['combined_filters_top10'] = combined_results[:10]

    # === 7. Per-direction analysis ===
    for dir_name in ['long', 'short']:
        ddf = df[df['direction'] == dir_name]
        if len(ddf) < 30:
            continue
        dir_corrs = {}
        for col in ind_cols:
            valid = ddf[[col, 'outcome']].dropna()
            if len(valid) < 20:
                continue
            corr = valid[col].astype(float).corr(valid['outcome'].astype(float))
            if not pd.isna(corr):
                dir_corrs[col] = round(corr, 4)
        sorted_dc = sorted(dir_corrs.items(), key=lambda x: abs(x[1]), reverse=True)
        results[f'correlations_{dir_name}_top10'] = sorted_dc[:10]

    return results


def main():
    print("=" * 90)
    print("RUN10.1 - SCALP INDICATOR DISCOVERY")
    print("=" * 90)
    print(f"Scalp params: SL={SCALP_SL*100:.2f}% TP={SCALP_TP*100:.2f}% "
          f"VMult={VOL_SPIKE_MULT} RSI_ext={RSI_EXTREME} Stoch_ext={STOCH_EXTREME} BB_sq={BB_SQUEEZE_FACTOR}")
    print(f"Coins: {len(COINS)}")
    print("=" * 90)

    # Load checkpoint
    checkpoint = None
    completed_coins = set()
    all_trades = []
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            checkpoint = json.load(f)
        completed_coins = set(checkpoint.get('completed_coins', []))
        all_trades = checkpoint.get('trades', [])
        print(f"Resumed from checkpoint: {len(completed_coins)}/{len(COINS)} coins, {len(all_trades)} trades")

    # Load all 15m data for breadth
    print("\nLoading 15m data for market breadth...")
    all_15m = {}
    for coin in COINS:
        df = load_csv(coin, '15m')
        if df is not None:
            all_15m[coin] = df
    print(f"  Loaded {len(all_15m)} coins")

    breadth, avg_z = build_market_breadth(all_15m)

    # Process each coin
    total_coins = len(COINS)
    start_time = _time.time()
    coins_this_run = 0

    for coin_idx, coin in enumerate(COINS):
        if coin in completed_coins:
            continue

        if _shutdown:
            print(f"\nSaving checkpoint ({len(completed_coins)}/{total_coins} coins)...")
            with open(CHECKPOINT_FILE, 'w') as f:
                json.dump({'completed_coins': list(completed_coins), 'trades': all_trades}, f)
            sys.exit(0)

        print(f"\n[{coin_idx+1}/{total_coins}] Processing {coin}...")

        df_1m = load_csv(coin, '1m')
        if df_1m is None:
            print(f"  SKIP: no 1m data")
            completed_coins.add(coin)
            continue

        # Calculate indicators
        print(f"  Calculating 1m indicators ({len(df_1m)} candles)...")
        df_1m_ind = calc_1m_indicators(df_1m)
        df_1m_ind = df_1m_ind.dropna(subset=['rsi', 'vol_ma'])

        df_15m_ind = None
        if coin in all_15m:
            df_15m_ind = calc_15m_indicators(all_15m[coin])

        # Collect trades
        print(f"  Running scalp trades...")
        coin_trades = run_scalp_collection(coin, df_1m_ind, df_15m_ind, breadth, avg_z)

        wins = sum(1 for t in coin_trades if t['outcome'] == 'win')
        losses = sum(1 for t in coin_trades if t['outcome'] == 'loss')
        wr = wins / len(coin_trades) * 100 if coin_trades else 0

        print(f"  {coin}: {len(coin_trades)} trades, {wins}W/{losses}L, WR={wr:.1f}%")

        # Per-strategy breakdown
        strat_counts = defaultdict(lambda: [0, 0])
        for t in coin_trades:
            if t['outcome'] == 'win':
                strat_counts[t['strategy']][0] += 1
            else:
                strat_counts[t['strategy']][1] += 1
        for s, (w, l) in strat_counts.items():
            total_s = w + l
            print(f"    {s}: {total_s} trades, WR={w/total_s*100:.1f}%")

        all_trades.extend(coin_trades)
        completed_coins.add(coin)
        coins_this_run += 1

        elapsed = _time.time() - start_time
        rate = coins_this_run / elapsed if elapsed > 0 else 0
        remaining = total_coins - len(completed_coins)
        eta = remaining / rate / 60 if rate > 0 else 0
        print(f"  Progress: {len(completed_coins)}/{total_coins}, ETA: {eta:.1f}m")

        # Checkpoint every 3 coins
        if len(completed_coins) % 3 == 0:
            with open(CHECKPOINT_FILE, 'w') as f:
                json.dump({'completed_coins': list(completed_coins), 'trades': all_trades}, f)

    # Save all trades
    print(f"\n{'='*90}")
    print(f"TRADE COLLECTION COMPLETE: {len(all_trades)} total trades")
    print(f"{'='*90}")

    # Save raw trades
    with open(TRADES_FILE, 'w') as f:
        json.dump(all_trades, f)
    print(f"Raw trades saved to {TRADES_FILE}")

    # === ANALYSIS ===
    print(f"\n{'='*90}")
    print("INDICATOR ANALYSIS")
    print(f"{'='*90}")

    results = analyze_trades(all_trades)

    print(f"\nTotal: {results.get('total_trades', 0)} trades, "
          f"{results.get('wins', 0)}W / {results.get('losses', 0)}L, "
          f"WR = {results.get('win_rate', 0):.1f}%")

    # Print correlations
    print(f"\n--- TOP 20 INDICATOR CORRELATIONS WITH WIN ---")
    print(f"{'Indicator':<30} {'Corr':>8}")
    print("-" * 40)
    for col, corr in results.get('correlations_top20', []):
        print(f"  {col:<28} {corr:>+8.4f}")

    # Print mean differences
    print(f"\n--- TOP 20 MEAN DIFFERENCES (Win vs Loss) ---")
    print(f"{'Indicator':<30} {'Win Mean':>10} {'Loss Mean':>10} {'Cohen d':>10}")
    print("-" * 65)
    for col, stats in results.get('mean_diffs_top20', []):
        print(f"  {col:<28} {stats['win_mean']:>10.4f} {stats['loss_mean']:>10.4f} {stats['cohens_d']:>+10.4f}")

    # Print per-strategy
    print(f"\n--- PER-STRATEGY ANALYSIS ---")
    for strat, sdata in results.get('per_strategy', {}).items():
        print(f"\n  {strat}: {sdata['trades']} trades, WR={sdata['win_rate']:.1f}%")
        print(f"    Top correlations:")
        for col, corr in sdata.get('top_correlations', [])[:5]:
            print(f"      {col:<28} {corr:>+8.4f}")

    # Print quintile analysis
    print(f"\n--- QUINTILE ANALYSIS (Win Rate by Indicator Quintile) ---")
    for col, quintiles in results.get('quintile_analysis', {}).items():
        print(f"\n  {col}:")
        for q in quintiles:
            bar = "█" * int(q['win_rate'] / 2)
            print(f"    Q{q['quintile']}: WR={q['win_rate']:>5.1f}% (n={q['count']:>5}) avg={q['avg_val']:>10.4f} {bar}")

    # Print filter candidates
    print(f"\n--- FILTER CANDIDATES (>2% WR improvement) ---")
    print(f"{'Indicator':<30} {'Direction':<12} {'Threshold':>10} {'Base WR':>8} {'Filt WR':>8} {'Δ WR':>6} {'Kept%':>6}")
    print("-" * 85)
    for fc in results.get('filter_candidates', []):
        print(f"  {fc['indicator']:<28} {fc['direction']:<12} {fc['threshold']:>10.4f} "
              f"{fc['base_wr']:>7.1f}% {fc['filtered_wr']:>7.1f}% {fc['improvement']:>+5.1f}% {fc['kept_pct']:>5.1f}%")

    # Print combined filters
    print(f"\n--- COMBINED FILTER TESTS ---")
    for cf in results.get('combined_filters_top10', []):
        print(f"  {cf['filters'][0]} + {cf['filters'][1]}")
        print(f"    WR: {cf['base_wr']:.1f}% → {cf['combo_wr']:.1f}% (+{cf['improvement']:.1f}%) "
              f"| Kept: {cf['kept_trades']}/{cf['total_trades']} ({cf['kept_pct']:.1f}%)")

    # Print per-direction
    for dir_name in ['long', 'short']:
        key = f'correlations_{dir_name}_top10'
        if key in results:
            print(f"\n--- TOP CORRELATIONS ({dir_name.upper()} trades only) ---")
            for col, corr in results[key]:
                print(f"  {col:<28} {corr:>+8.4f}")

    # Save results
    with open(RESULTS_FILE, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {RESULTS_FILE}")

    # Cleanup checkpoint
    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("Checkpoint removed (clean finish)")

    print(f"\nDone in {(_time.time() - start_time) / 60:.1f} minutes")


if __name__ == "__main__":
    main()
