#!/usr/bin/env python3
"""
RUN6.1 - Isolated Short Strategy Discovery
Backtest 9 ISO short strategies that activate when breadth is LOW (calm market)
and individual coins are overbought (z > +1.5). Mirror of best long setup.
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
STOP_LOSS = 0.005        # 0.5% stop loss
MIN_HOLD = 2

# Breadth thresholds: only enter ISO shorts when breadth <= threshold (calm market)
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


def load_cache(name):
    cache_file = f"{DATA_CACHE_DIR}/{name}_USDT_15m_5months.csv"
    if os.path.exists(cache_file):
        return pd.read_csv(cache_file, index_col=0, parse_dates=True)
    return None


def calculate_indicators(df):
    """Extended indicators including RSI, BB width, BB width avg."""
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

    # RSI (14-period)
    delta = df['c'].diff()
    gain = (delta.where(delta > 0, 0)).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
    rs = gain / loss
    df['rsi'] = 100 - (100 / (1 + rs))

    return df


def build_market_breadth(all_data):
    """Fraction of coins with z < -1 at each timestamp. Also returns avg_z."""
    z_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        z_frames[coin] = df_ind['z']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    avg_z = z_df.mean(axis=1)
    return breadth, avg_z, z_df


def build_market_rsi(all_data):
    """Average RSI across coins at each timestamp."""
    rsi_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        rsi_frames[coin] = df_ind['rsi']
    rsi_df = pd.DataFrame(rsi_frames).dropna(how='all')
    avg_rsi = rsi_df.mean(axis=1)
    return avg_rsi


def iso_short_entry_signal(row, strategy, params, market_ctx=None):
    """ISO short entry: coin-specific overbought conditions during calm market."""
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False

    # Entry guard: skip if price < SMA20 or z < -0.5
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
    """Backtest an ISO short strategy. PnL = (entry - exit) / entry."""
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

            # Stop loss: price rises above entry
            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'type': 'loss'})
                position = None
                cooldown = 3
                candles_held = 0
                continue

            # Take profit: after min hold, price drops below SMA20 or z < exit_z
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
            # Build market context for this timestamp
            market_ctx = {}
            if avg_z_series is not None and idx in avg_z_series.index:
                market_ctx['avg_z'] = avg_z_series.loc[idx]
            if avg_rsi_series is not None and idx in avg_rsi_series.index:
                market_ctx['avg_rsi'] = avg_rsi_series.loc[idx]
            if btc_z_series is not None and idx in btc_z_series.index:
                market_ctx['btc_z'] = btc_z_series.loc[idx]

            if iso_short_entry_signal(row, strategy, params, market_ctx):
                # Breadth gate: only enter ISO shorts when market is CALM
                if breadth is not None and idx in breadth.index:
                    if breadth.loc[idx] > breadth_max:
                        continue  # Market too bearish for isolated shorts
                position = True
                entry_price = price

    # Close any open short at end
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
        'strategy': strategy,
        'pf': abs(total_win / total_loss) if total_loss != 0 else 0,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'trades': len(trades),
        'wins': len(wins),
        'wr': len(wins) / len(trades) * 100,
        'avg_win': np.mean([t['pnl_pct'] for t in wins]) if wins else 0,
        'avg_loss': np.mean([t['pnl_pct'] for t in losses]) if losses else 0,
    }


def main():
    print("=" * 90)
    print("RUN6.1 - ISOLATED SHORT STRATEGY DISCOVERY")
    print("=" * 90)
    print("Testing 9 ISO short strategies for coin-specific overbought conditions during CALM markets")
    print(f"Strategies: {ISO_SHORT_STRATEGIES}")
    print(f"Breadth max thresholds: {BREADTH_MAX_THRESHOLDS}")
    print(f"Parameter sets: {list(PARAM_SETS.keys())}")
    print(f"Test matrix: {len(ISO_SHORT_STRATEGIES)} strats × {len(COINS)} coins × "
          f"{len(BREADTH_MAX_THRESHOLDS)} breadth × {len(PARAM_SETS)} params = "
          f"{len(ISO_SHORT_STRATEGIES)*len(COINS)*len(BREADTH_MAX_THRESHOLDS)*len(PARAM_SETS)} combos")
    print("=" * 90)

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build market breadth and cross-coin data
    breadth, avg_z, z_df = build_market_breadth(all_data)
    avg_rsi = build_market_rsi(all_data)
    print(f"Breadth built: avg={breadth.mean():.1%}, pct<=20%: {(breadth <= 0.2).mean():.1%}")

    # BTC z-score series for iso_divergence
    btc_z = None
    if 'BTC' in all_data:
        btc_df = calculate_indicators(all_data['BTC'])
        btc_z = btc_df['z']

    # === TEST MATRIX ===
    all_results = []
    total_combos = len(PARAM_SETS) * len(BREADTH_MAX_THRESHOLDS) * len(ISO_SHORT_STRATEGIES)
    combo_num = 0

    for param_name, params in PARAM_SETS.items():
        for bmax in BREADTH_MAX_THRESHOLDS:
            for strat in ISO_SHORT_STRATEGIES:
                combo_num += 1
                if combo_num % 20 == 0:
                    print(f"  Testing combo {combo_num}/{total_combos}...", end='\r')

                strat_pfs = []
                strat_wrs = []
                strat_trades = []
                strat_pnls = []
                coin_details = {}

                for coin, df in all_data.items():
                    r = run_iso_short_backtest(
                        df, strat, params, breadth, bmax,
                        avg_z_series=avg_z, avg_rsi_series=avg_rsi, btc_z_series=btc_z
                    )
                    if r and r['trades'] >= 2:
                        strat_pfs.append(r['pf'])
                        strat_wrs.append(r['wr'])
                        strat_trades.append(r['trades'])
                        strat_pnls.append(r['pnl'])
                        coin_details[coin] = r

                if not strat_pfs:
                    continue

                all_results.append({
                    'params': param_name,
                    'breadth_max': bmax,
                    'strategy': strat,
                    'avg_pf': np.mean(strat_pfs),
                    'avg_wr': np.mean(strat_wrs),
                    'total_trades': sum(strat_trades),
                    'avg_pnl': np.mean(strat_pnls),
                    'coins_traded': len(coin_details),
                    'coin_details': coin_details,
                })

    print()

    # Sort by composite score
    all_results.sort(key=lambda x: x['avg_pf'] * (x['avg_wr'] / 100) ** 0.5, reverse=True)

    # === PRINT ALL RESULTS ===
    print(f"\n{'='*90}")
    print("ISO SHORT STRATEGY RESULTS (sorted by composite score)")
    print(f"{'='*90}")
    print(f"\n{'Params':<14} {'Breadth<=':<10} {'Strategy':<18} {'Avg PF':<8} {'Avg WR':<8} "
          f"{'Trades':<8} {'Coins':<6} {'Avg P&L'}")
    print("-" * 90)

    for r in all_results[:40]:
        bmax_str = f"{r['breadth_max']*100:.0f}%"
        marker = " ***" if r['avg_wr'] >= 55 else (" **" if r['avg_wr'] >= 50 else "")
        print(f"{r['params']:<14} {bmax_str:<10} {r['strategy']:<18} {r['avg_pf']:<8.2f} "
              f"{r['avg_wr']:<8.1f}% {r['total_trades']:<8} {r['coins_traded']:<6} "
              f"{r['avg_pnl']:+.1f}%{marker}")

    # === BEST ISO SHORT STRATEGY PER COIN ===
    print(f"\n{'='*90}")
    print("BEST ISO SHORT STRATEGY PER COIN")
    print(f"{'='*90}")

    optimal_iso_short = {}
    best_iso_params = {}

    for coin in COINS:
        best_score = -1
        best_entry = None

        for r in all_results:
            if coin in r['coin_details']:
                d = r['coin_details'][coin]
                score = d['pf'] * (d['wr'] / 100) ** 0.5
                if score > best_score and d['trades'] >= 3:
                    best_score = score
                    best_entry = {
                        'strategy': r['strategy'],
                        'params': r['params'],
                        'breadth_max': r['breadth_max'],
                        **d,
                    }

        if best_entry and best_entry['pf'] >= 1.0:
            optimal_iso_short[coin] = best_entry['strategy']
            best_iso_params[coin] = best_entry
            print(f"  {coin:<8} {best_entry['strategy']:<18} params={best_entry['params']:<14} "
                  f"bmax={best_entry['breadth_max']*100:.0f}% "
                  f"PF={best_entry['pf']:.2f} WR={best_entry['wr']:.1f}% "
                  f"trades={best_entry['trades']} P&L={best_entry['pnl']:+.1f}%")
        else:
            # Fall back to best with >= 2 trades
            best_score2 = -1
            best_entry2 = None
            for r in all_results:
                if coin in r['coin_details']:
                    d = r['coin_details'][coin]
                    score = d['pf'] * (d['wr'] / 100) ** 0.5
                    if score > best_score2 and d['trades'] >= 2:
                        best_score2 = score
                        best_entry2 = {
                            'strategy': r['strategy'],
                            'params': r['params'],
                            'breadth_max': r['breadth_max'],
                            **d,
                        }
            if best_entry2 and best_entry2['pf'] >= 1.0:
                optimal_iso_short[coin] = best_entry2['strategy']
                best_iso_params[coin] = best_entry2
                print(f"  {coin:<8} {best_entry2['strategy']:<18} params={best_entry2['params']:<14} "
                      f"bmax={best_entry2['breadth_max']*100:.0f}% "
                      f"PF={best_entry2['pf']:.2f} WR={best_entry2['wr']:.1f}% "
                      f"trades={best_entry2['trades']} P&L={best_entry2['pnl']:+.1f}% (low-conf)")
            else:
                print(f"  {coin:<8} NO VIABLE ISO SHORT STRATEGY")

    # === SUMMARY ===
    print(f"\n{'='*90}")
    print("SUMMARY")
    print(f"{'='*90}")

    viable = [c for c in COINS if c in optimal_iso_short]
    print(f"\n  Viable ISO short coins: {len(viable)}/{len(COINS)}")

    if viable:
        avg_pf = np.mean([best_iso_params[c]['pf'] for c in viable])
        avg_wr = np.mean([best_iso_params[c]['wr'] for c in viable])
        avg_trades = np.mean([best_iso_params[c]['trades'] for c in viable])
        print(f"  Avg PF across viable: {avg_pf:.2f}")
        print(f"  Avg WR across viable: {avg_wr:.1f}%")
        print(f"  Avg trades per coin:  {avg_trades:.1f}")

    # Best breadth_max threshold
    if all_results:
        breadth_stats = {}
        for bmax in BREADTH_MAX_THRESHOLDS:
            br = [r for r in all_results if r['breadth_max'] == bmax]
            if br:
                breadth_stats[bmax] = {
                    'avg_pf': np.mean([r['avg_pf'] for r in br]),
                    'avg_wr': np.mean([r['avg_wr'] for r in br]),
                    'avg_trades': np.mean([r['total_trades'] for r in br]),
                }
        print(f"\n  Breadth threshold comparison (ISO shorts enter when breadth <= threshold):")
        for bmax, stats in sorted(breadth_stats.items()):
            print(f"    breadth<={bmax*100:.0f}%: avg PF={stats['avg_pf']:.2f}, "
                  f"avg WR={stats['avg_wr']:.1f}%, avg trades={stats['avg_trades']:.0f}")

    # Strategy comparison
    if all_results:
        strat_stats = {}
        for strat in ISO_SHORT_STRATEGIES:
            sr = [r for r in all_results if r['strategy'] == strat]
            if sr:
                strat_stats[strat] = {
                    'avg_pf': np.mean([r['avg_pf'] for r in sr]),
                    'avg_wr': np.mean([r['avg_wr'] for r in sr]),
                    'count': len(sr),
                }
        print(f"\n  Strategy comparison:")
        for strat, stats in sorted(strat_stats.items(), key=lambda x: -x[1]['avg_pf']):
            marker = " ***" if stats['avg_wr'] >= 55 else (" **" if stats['avg_wr'] >= 50 else "")
            print(f"    {strat:<18} avg PF={stats['avg_pf']:.2f}, avg WR={stats['avg_wr']:.1f}%{marker}")

    # Print OPTIMAL_ISO_SHORT_STRAT dict for use in trader.py
    print(f"\n  OPTIMAL_ISO_SHORT_STRAT = {{")
    for coin in COINS:
        if coin in optimal_iso_short:
            print(f"      '{coin}': '{optimal_iso_short[coin]}',")
    print(f"  }}")

    # Save results
    save_data = {
        'optimal_iso_short_strat': optimal_iso_short,
        'best_iso_params': {c: {k: v for k, v in d.items() if k != 'coin_details'}
                            for c, d in best_iso_params.items()},
        'all_results': [{k: v for k, v in r.items() if k != 'coin_details'}
                        for r in all_results[:80]],
    }

    with open('/home/scamarena/ProjectCoin/run6_1_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run6_1_results.json")


if __name__ == "__main__":
    main()
