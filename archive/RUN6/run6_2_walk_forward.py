#!/usr/bin/env python3
"""
RUN6.2 - Walk-Forward Validation of ISO Short Strategies
Same walk-forward as run5_1 (train 2mo, test 1mo, 3 windows) but for
ISO short strategies. Validates that coin-specific overbought shorts survive OOS.
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

ISO_SHORT_STRATEGIES = [
    'iso_mean_rev', 'iso_vwap_rev', 'iso_bb_bounce', 'iso_adr_rev',
    'iso_relative_z', 'iso_rsi_extreme', 'iso_divergence',
    'iso_vol_spike', 'iso_bb_squeeze',
]

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.005
MIN_HOLD = 2

# Breadth thresholds to test during training
BREADTH_MAX_THRESHOLDS = [0.10, 0.15, 0.20, 0.30, 0.50]

PARAM_SETS = {
    'Default':      {'z_threshold': 1.5, 'bb_margin': 0.98, 'vol_mult': 1.2,
                     'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
                     'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.8},
    'Aggressive':   {'z_threshold': 1.2, 'bb_margin': 0.99, 'vol_mult': 1.0,
                     'adr_pct': 0.30, 'exit_z': -0.3, 'z_spread': 1.0,
                     'rsi_threshold': 70, 'vol_spike_mult': 1.5, 'squeeze_factor': 0.9},
    'Conservative': {'z_threshold': 2.0, 'bb_margin': 0.97, 'vol_mult': 1.5,
                     'adr_pct': 0.20, 'exit_z': -0.7, 'z_spread': 2.0,
                     'rsi_threshold': 80, 'vol_spike_mult': 2.5, 'squeeze_factor': 0.6},
    'Balanced':     {'z_threshold': 1.8, 'bb_margin': 0.98, 'vol_mult': 1.3,
                     'adr_pct': 0.25, 'exit_z': -0.5, 'z_spread': 1.5,
                     'rsi_threshold': 75, 'vol_spike_mult': 2.0, 'squeeze_factor': 0.7},
}

WINDOWS = [
    {'name': 'W1', 'train_start': '2025-10-15', 'train_end': '2025-12-14',
     'test_start': '2025-12-15', 'test_end': '2026-01-14'},
    {'name': 'W2', 'train_start': '2025-11-15', 'train_end': '2026-01-14',
     'test_start': '2026-01-15', 'test_end': '2026-02-14'},
    {'name': 'W3', 'train_start': '2025-12-15', 'train_end': '2026-02-14',
     'test_start': '2026-02-15', 'test_end': '2026-03-10'},
]


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
    df['bb_hi'] = df['sma20'] + 2 * df['std20']
    df['bb_lo'] = df['sma20'] - 2 * df['std20']
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
    """Build breadth, avg_z, avg_rsi, and BTC z-score series."""
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


def run_iso_short_backtest(df, strategy, params, breadth=None, breadth_max=0.20,
                           avg_z_series=None, avg_rsi_series=None, btc_z_series=None):
    df = calculate_indicators(df)
    df = df.dropna()
    if len(df) < 50:
        return None

    balance = INITIAL_CAPITAL
    position = None
    entry_price = 0
    trades = []
    cooldown = 0
    candles_held = 0

    for idx, row in df.iterrows():
        price = row['c']

        if position:
            candles_held += 1
            price_pnl = (entry_price - price) / entry_price

            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'type': 'loss'})
                position = None
                cooldown = 3
                candles_held = 0
                continue

            if price_pnl > 0 and candles_held >= MIN_HOLD:
                if price < row['sma20']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] < params['exit_z']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue

        if cooldown > 0:
            cooldown -= 1

        if not position and cooldown == 0:
            market_ctx = {}
            if avg_z_series is not None and idx in avg_z_series.index:
                market_ctx['avg_z'] = avg_z_series.loc[idx]
            if avg_rsi_series is not None and idx in avg_rsi_series.index:
                market_ctx['avg_rsi'] = avg_rsi_series.loc[idx]
            if btc_z_series is not None and idx in btc_z_series.index:
                market_ctx['btc_z'] = btc_z_series.loc[idx]

            if iso_short_entry_signal(row, strategy, params, market_ctx):
                if breadth is not None and idx in breadth.index:
                    if breadth.loc[idx] > breadth_max:
                        continue
                position = True
                entry_price = price

    if position:
        price_pnl = (entry_price - df.iloc[-1]['c']) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win' if price_pnl > 0 else 'loss'})

    if not trades:
        return None

    wins = [t for t in trades if t['pnl_pct'] > 0]
    losses = [t for t in trades if t['pnl_pct'] <= 0]
    total_win = sum(t['pnl_pct'] for t in wins) if wins else 0
    total_loss = sum(t['pnl_pct'] for t in losses) if losses else 0

    return {
        'pf': abs(total_win / total_loss) if total_loss != 0 else 0,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'trades': len(trades),
        'wins': len(wins),
        'wr': len(wins) / len(trades) * 100,
    }


def main():
    print("=" * 90)
    print("RUN6.2 - WALK-FORWARD VALIDATION OF ISO SHORT STRATEGIES")
    print("=" * 90)
    print(f"Train: 2 months | Test: 1 month | 3 windows")
    print(f"Train: find best ISO short strategy + breadth_max + params per coin")
    print(f"Test: run OOS with best breadth_max from run6_1")
    print("=" * 90)

    # Load run6_1 results for fixed breadth_max
    r61_file = '/home/scamarena/ProjectCoin/run6_1_results.json'
    BREADTH_MAX_FIXED = 0.20  # default
    OPTIMAL_ISO_SHORT = {}
    if os.path.exists(r61_file):
        with open(r61_file, 'r') as f:
            r61 = json.load(f)
        if 'optimal_iso_short_strat' in r61:
            OPTIMAL_ISO_SHORT = r61['optimal_iso_short_strat']
            print(f"Loaded run6_1 optimal ISO short strategies for {len(OPTIMAL_ISO_SHORT)} coins")
        # Find most common breadth_max from best params
        if 'best_iso_params' in r61:
            bmaxes = [d.get('breadth_max', 0.20) for d in r61['best_iso_params'].values()]
            if bmaxes:
                from collections import Counter
                most_common = Counter(bmaxes).most_common(1)[0][0]
                BREADTH_MAX_FIXED = most_common
                print(f"Using most common breadth_max from run6_1: {BREADTH_MAX_FIXED*100:.0f}%")
    else:
        print("WARNING: run6_1_results.json not found, using defaults")

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build full market data
    breadth, avg_z, avg_rsi, btc_z = build_market_data(all_data)
    print(f"Breadth built: {len(breadth)} timestamps, avg={breadth.mean():.1%}")

    all_results = {}

    for coin in COINS:
        if coin not in all_data:
            continue
        df = all_data[coin]
        coin_windows = []

        for w in WINDOWS:
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
                continue

            # Train: find best ISO short strategy + breadth_max + params
            best_train_score = -1
            best_train_strat = None
            best_train_bmax = 0.20
            best_train_pf = 0
            best_train_params_name = 'Default'

            for param_name, params in PARAM_SETS.items():
                for bmax in BREADTH_MAX_THRESHOLDS:
                    for strat in ISO_SHORT_STRATEGIES:
                        r = run_iso_short_backtest(
                            train_df, strat, params, train_breadth, bmax,
                            train_avg_z, train_avg_rsi, train_btc_z
                        )
                        if r and r['trades'] >= 3:
                            score = r['pf'] * (r['wr'] / 100) ** 0.5
                            if score > best_train_score:
                                best_train_score = score
                                best_train_strat = strat
                                best_train_bmax = bmax
                                best_train_pf = r['pf']
                                best_train_params_name = param_name

            if best_train_strat is None:
                continue

            # Test: run with FIXED breadth_max
            best_params = PARAM_SETS[best_train_params_name]
            test_result_fixed = run_iso_short_backtest(
                test_df, best_train_strat, best_params, test_breadth, BREADTH_MAX_FIXED,
                test_avg_z, test_avg_rsi, test_btc_z
            )

            # Also test with assigned strategy from run6_1 + fixed breadth
            assigned = OPTIMAL_ISO_SHORT.get(coin)
            test_assigned = None
            if assigned:
                test_assigned = run_iso_short_backtest(
                    test_df, assigned, PARAM_SETS['Default'], test_breadth, BREADTH_MAX_FIXED,
                    test_avg_z, test_avg_rsi, test_btc_z
                )

            coin_windows.append({
                'window': w['name'],
                'train_best_strat': best_train_strat,
                'train_best_bmax': best_train_bmax,
                'train_best_params': best_train_params_name,
                'train_pf': best_train_pf,
                'test_pf_fixed': test_result_fixed['pf'] if test_result_fixed else 0,
                'test_wr_fixed': test_result_fixed['wr'] if test_result_fixed else 0,
                'test_trades_fixed': test_result_fixed['trades'] if test_result_fixed else 0,
                'assigned_strat': assigned or 'none',
                'assigned_pf': test_assigned['pf'] if test_assigned else 0,
                'assigned_wr': test_assigned['wr'] if test_assigned else 0,
                'assigned_trades': test_assigned['trades'] if test_assigned else 0,
            })

        if coin_windows:
            all_results[coin] = coin_windows

    # === PRINT RESULTS ===
    print(f"\n{'='*90}")
    print("WALK-FORWARD RESULTS BY COIN (ISO shorts)")
    print(f"{'='*90}")

    for coin, wins in all_results.items():
        assigned = OPTIMAL_ISO_SHORT.get(coin, 'none')
        print(f"\n{coin} (assigned: {assigned})")
        print(f"  {'Win':<4} {'Train Strat':<18} {'Params':<14} {'Train Bmax':<11} {'Train PF':<10} "
              f"{'Test PF':<10} {'Test WR':<10} {'Test #':<8} {'Assgn PF'}")
        print(f"  {'-'*110}")
        for w in wins:
            bmax_str = f"{w['train_best_bmax']*100:.0f}%"
            low_conf = " *" if w['test_trades_fixed'] < 3 else ""
            print(f"  {w['window']:<4} {w['train_best_strat']:<18} {w['train_best_params']:<14} "
                  f"{bmax_str:<11} {w['train_pf']:<10.2f} {w['test_pf_fixed']:<10.2f} "
                  f"{w['test_wr_fixed']:<10.1f} {w['test_trades_fixed']:<8}{low_conf}"
                  f"  {w['assigned_pf']:.2f}")

    # === DEGRADATION ANALYSIS ===
    print(f"\n{'='*90}")
    print("ISO SHORT DEGRADATION ANALYSIS")
    print(f"{'='*90}")

    train_pfs = []
    test_pfs = []

    for coin, wins in all_results.items():
        for w in wins:
            train_pfs.append(w['train_pf'])
            test_pfs.append(w['test_pf_fixed'])

    avg_train = np.mean(train_pfs) if train_pfs else 0
    avg_test = np.mean(test_pfs) if test_pfs else 0
    degradation = (1 - avg_test / avg_train) * 100 if avg_train > 0 else 0

    print(f"\n  Avg Train PF:         {avg_train:.2f}")
    print(f"  Avg Test PF (OOS):    {avg_test:.2f}  (degradation: {degradation:.1f}%)")

    if degradation < 20:
        verdict = "LOW"
    elif degradation < 40:
        verdict = "MODERATE"
    else:
        verdict = "HIGH"

    print(f"\n  Verdict: {verdict} overfitting risk for ISO short strategies")

    # === LOW CONFIDENCE FLAG ===
    print(f"\n{'='*90}")
    print("LOW CONFIDENCE FLAGS (< 3 trades in OOS window)")
    print(f"{'='*90}")

    for coin, wins in all_results.items():
        low_conf_count = sum(1 for w in wins if w['test_trades_fixed'] < 3)
        if low_conf_count > 0:
            print(f"  {coin:<8} {low_conf_count}/{len(wins)} windows with < 3 trades")

    # === CONSISTENCY CHECK ===
    print(f"\n{'='*90}")
    print("CONSISTENCY (profitable in how many OOS windows?)")
    print(f"{'='*90}")

    consistent = 0
    total_coins = 0
    for coin, wins in all_results.items():
        profitable = sum(1 for w in wins if w['test_pf_fixed'] >= 1.0)
        pct = profitable / len(wins) * 100
        status = "OK" if pct >= 67 else "WEAK"
        print(f"  {coin:<8} {profitable}/{len(wins)} windows profitable ({pct:.0f}%) - {status}")
        if pct >= 67:
            consistent += 1
        total_coins += 1

    print(f"\n  Consistent coins: {consistent}/{total_coins} ({consistent/total_coins*100:.0f}%)" if total_coins > 0 else "")

    # Save results
    save_data = {
        'breadth_max_fixed': BREADTH_MAX_FIXED,
        'avg_train_pf': avg_train,
        'avg_test_pf': avg_test,
        'degradation_pct': degradation,
        'verdict': verdict,
        'consistent_coins': consistent,
        'total_coins': total_coins,
        'coin_results': {coin: wins for coin, wins in all_results.items()},
    }

    with open('/home/scamarena/ProjectCoin/run6_2_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run6_2_results.json")


if __name__ == "__main__":
    main()
