#!/usr/bin/env python3
"""
RUN5.2 - Short Strategy Backtesting
Backtest 4 short strategies that activate when breadth >= 50% (market dump).
Mirror of long strategies but for overbought conditions.
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

SHORT_STRATEGIES = ['short_vwap_rev', 'short_bb_bounce', 'short_mean_rev', 'short_adr_rev']

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.005        # 0.5% stop loss (price rises above entry)
MIN_HOLD = 2

# Breadth thresholds to test: only enter shorts when breadth >= threshold
BREADTH_MIN_THRESHOLDS = [0.40, 0.50, 0.60]

# Parameter sets for short strategies
PARAM_SETS = {
    'Default': {
        'z_threshold': 1.5, 'bb_margin': 0.98,
        'vol_mult': 1.2, 'adr_pct': 0.25, 'exit_z': -0.5,
    },
    'Aggressive': {
        'z_threshold': 1.2, 'bb_margin': 0.99,
        'vol_mult': 1.0, 'adr_pct': 0.30, 'exit_z': -0.3,
    },
    'Conservative': {
        'z_threshold': 2.0, 'bb_margin': 0.97,
        'vol_mult': 1.5, 'adr_pct': 0.20, 'exit_z': -0.7,
    },
    'Balanced': {
        'z_threshold': 1.8, 'bb_margin': 0.98,
        'vol_mult': 1.3, 'adr_pct': 0.25, 'exit_z': -0.5,
    },
}


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
    df['vol_ma'] = df['v'].rolling(20).mean()
    df['adr_lo'] = df['l'].rolling(24).min()
    df['adr_hi'] = df['h'].rolling(24).max()
    return df


def build_market_breadth(all_data):
    """Fraction of coins with z < -1 at each timestamp."""
    z_frames = {}
    for coin, df in all_data.items():
        df_ind = calculate_indicators(df)
        z_frames[coin] = df_ind['z']
    z_df = pd.DataFrame(z_frames).dropna(how='all')
    breadth = (z_df < -1.0).sum(axis=1) / z_df.notna().sum(axis=1)
    return breadth


def short_entry_signal(row, strategy, params):
    """Short entry: overbought conditions."""
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False

    # Short entry guard: skip if already oversold
    if row['c'] < row['sma20'] or row['z'] < -0.5:
        return False

    if strategy == 'short_vwap_rev':
        return (row['z'] > params['z_threshold'] and
                row['c'] > row['sma20'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'short_bb_bounce':
        return (row['c'] >= row['bb_hi'] * params['bb_margin'] and
                row['v'] > row['vol_ma'] * (params['vol_mult'] + 0.1))
    elif strategy == 'short_mean_rev':
        return row['z'] > params['z_threshold']
    elif strategy == 'short_adr_rev':
        adr_range = row['adr_hi'] - row['adr_lo']
        return row['c'] >= row['adr_hi'] - adr_range * params['adr_pct']
    return False


def run_short_backtest(df, strategy, params, breadth=None, breadth_min=0.50):
    """Backtest a short strategy. PnL = (entry - exit) / entry."""
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
            # Short PnL: profit when price drops
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
            if short_entry_signal(row, strategy, params):
                # Breadth gate: only enter shorts during market dumps
                if breadth is not None and idx in breadth.index:
                    if breadth.loc[idx] < breadth_min:
                        continue  # Market not dumping enough for shorts
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
    print("RUN5.2 - SHORT STRATEGY BACKTESTING")
    print("=" * 90)
    print("Testing 4 short strategies for overbought conditions during market dumps")
    print(f"Strategies: {SHORT_STRATEGIES}")
    print(f"Breadth min thresholds: {BREADTH_MIN_THRESHOLDS}")
    print(f"Parameter sets: {list(PARAM_SETS.keys())}")
    print("=" * 90)

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build market breadth
    breadth = build_market_breadth(all_data)
    print(f"Breadth built: avg={breadth.mean():.1%}, pct>=50%: {(breadth >= 0.5).mean():.1%}")

    # === TEST MATRIX: 4 strats × 18 coins × 3 breadth × 4 param sets ===
    all_results = []

    for param_name, params in PARAM_SETS.items():
        for bmin in BREADTH_MIN_THRESHOLDS:
            for strat in SHORT_STRATEGIES:
                strat_pfs = []
                strat_wrs = []
                strat_trades = []
                strat_pnls = []
                coin_details = {}

                for coin, df in all_data.items():
                    r = run_short_backtest(df, strat, params, breadth, bmin)
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
                    'breadth_min': bmin,
                    'strategy': strat,
                    'avg_pf': np.mean(strat_pfs),
                    'avg_wr': np.mean(strat_wrs),
                    'total_trades': sum(strat_trades),
                    'avg_pnl': np.mean(strat_pnls),
                    'coins_traded': len(coin_details),
                    'coin_details': coin_details,
                })

    # Sort by composite score
    all_results.sort(key=lambda x: x['avg_pf'] * (x['avg_wr'] / 100) ** 0.5, reverse=True)

    # === PRINT ALL RESULTS ===
    print(f"\n{'='*90}")
    print("SHORT STRATEGY RESULTS (sorted by composite score)")
    print(f"{'='*90}")
    print(f"\n{'Params':<14} {'Breadth>=':<10} {'Strategy':<18} {'Avg PF':<8} {'Avg WR':<8} "
          f"{'Trades':<8} {'Coins':<6} {'Avg P&L'}")
    print("-" * 90)

    for r in all_results[:30]:
        bmin_str = f"{r['breadth_min']*100:.0f}%"
        marker = " ***" if r['avg_wr'] >= 55 else (" **" if r['avg_wr'] >= 50 else "")
        print(f"{r['params']:<14} {bmin_str:<10} {r['strategy']:<18} {r['avg_pf']:<8.2f} "
              f"{r['avg_wr']:<8.1f}% {r['total_trades']:<8} {r['coins_traded']:<6} "
              f"{r['avg_pnl']:+.1f}%{marker}")

    # === BEST SHORT STRATEGY PER COIN ===
    print(f"\n{'='*90}")
    print("BEST SHORT STRATEGY PER COIN")
    print(f"{'='*90}")

    optimal_short = {}
    best_short_params = {}

    for coin in COINS:
        best_score = -1
        best_entry = None

        for r in all_results:
            if coin in r['coin_details']:
                d = r['coin_details'][coin]
                score = d['pf'] * (d['wr'] / 100) ** 0.5
                if score > best_score and d['trades'] >= 2:
                    best_score = score
                    best_entry = {
                        'strategy': r['strategy'],
                        'params': r['params'],
                        'breadth_min': r['breadth_min'],
                        **d,
                    }

        if best_entry and best_entry['pf'] >= 1.0:
            optimal_short[coin] = best_entry['strategy']
            best_short_params[coin] = best_entry
            print(f"  {coin:<8} {best_entry['strategy']:<18} params={best_entry['params']:<14} "
                  f"bmin={best_entry['breadth_min']*100:.0f}% "
                  f"PF={best_entry['pf']:.2f} WR={best_entry['wr']:.1f}% "
                  f"trades={best_entry['trades']} P&L={best_entry['pnl']:+.1f}%")
        else:
            print(f"  {coin:<8} NO VIABLE SHORT STRATEGY")

    # === SUMMARY ===
    print(f"\n{'='*90}")
    print("SUMMARY")
    print(f"{'='*90}")

    viable = [c for c in COINS if c in optimal_short]
    print(f"\n  Viable short coins: {len(viable)}/{len(COINS)}")

    if viable:
        avg_pf = np.mean([best_short_params[c]['pf'] for c in viable])
        avg_wr = np.mean([best_short_params[c]['wr'] for c in viable])
        print(f"  Avg PF across viable: {avg_pf:.2f}")
        print(f"  Avg WR across viable: {avg_wr:.1f}%")

    # Best overall breadth_min
    if all_results:
        breadth_stats = {}
        for bmin in BREADTH_MIN_THRESHOLDS:
            br = [r for r in all_results if r['breadth_min'] == bmin]
            if br:
                breadth_stats[bmin] = {
                    'avg_pf': np.mean([r['avg_pf'] for r in br]),
                    'avg_wr': np.mean([r['avg_wr'] for r in br]),
                }
        print(f"\n  Breadth threshold comparison:")
        for bmin, stats in sorted(breadth_stats.items()):
            print(f"    breadth>={bmin*100:.0f}%: avg PF={stats['avg_pf']:.2f}, avg WR={stats['avg_wr']:.1f}%")

    # Print OPTIMAL_SHORT_STRAT dict for use in trader.py
    print(f"\n  OPTIMAL_SHORT_STRAT = {{")
    for coin in COINS:
        if coin in optimal_short:
            print(f"      '{coin}': '{optimal_short[coin]}',")
    print(f"  }}")

    # Save results
    save_data = {
        'optimal_short_strat': optimal_short,
        'best_short_params': {c: {k: v for k, v in d.items() if k != 'coin_details'}
                              for c, d in best_short_params.items()},
        'all_results': [{k: v for k, v in r.items() if k != 'coin_details'}
                        for r in all_results[:50]],
    }

    with open('/home/scamarena/ProjectCoin/run5_2_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run5_2_results.json")


if __name__ == "__main__":
    main()
