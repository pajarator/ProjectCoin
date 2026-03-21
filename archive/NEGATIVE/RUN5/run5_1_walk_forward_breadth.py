#!/usr/bin/env python3
"""
RUN5.1 - Walk-Forward Validation of v5 Breadth Filter
Same walk-forward as run4_3 (train 2mo, test 1mo, 3 windows) but with
breadth filter applied. Validates BREADTH_MAX=0.20 doesn't overfit.
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

OPTIMAL_STRAT = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

STRATEGIES = ['mean_reversion', 'vwap_reversion', 'bb_bounce', 'adr_reversal', 'dual_rsi']

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10
STOP_LOSS = 0.005
MIN_HOLD = 2

# Breadth thresholds to test during training
BREADTH_THRESHOLDS = [1.0, 0.50, 0.40, 0.30, 0.25, 0.20, 0.15]
# Fixed threshold for out-of-sample testing
BREADTH_MAX_FIXED = 0.20

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


def entry_signal(row, strategy):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
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


def run_backtest(df, strategy, breadth=None, breadth_max=1.0):
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
            price_pnl = (price - entry_price) / entry_price

            if price_pnl <= -STOP_LOSS:
                loss = balance * RISK * STOP_LOSS * LEVERAGE
                balance -= loss
                trades.append({'pnl_pct': -STOP_LOSS * LEVERAGE * 100, 'type': 'loss'})
                position = None
                cooldown = 3
                candles_held = 0
                continue

            if price_pnl > 0 and candles_held >= MIN_HOLD:
                if row['c'] > row['sma20']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append({'pnl_pct': price_pnl * LEVERAGE * 100, 'type': 'win'})
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] > 0.5:
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
            if entry_signal(row, strategy):
                if breadth is not None and idx in breadth.index:
                    if breadth.loc[idx] > breadth_max:
                        continue
                position = True
                entry_price = price

    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
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
    print("RUN5.1 - WALK-FORWARD VALIDATION OF v5 BREADTH FILTER")
    print("=" * 90)
    print(f"Train: 2 months | Test: 1 month | 3 windows")
    print(f"Train: find best strategy+breadth threshold per coin")
    print(f"Test: fixed BREADTH_MAX={BREADTH_MAX_FIXED*100:.0f}%")
    print("=" * 90)

    # Load all data
    all_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            all_data[coin] = df
    print(f"\nLoaded {len(all_data)} coins")

    # Build full breadth series
    breadth = build_market_breadth(all_data)
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

            if len(train_df) < 100 or len(test_df) < 50:
                continue

            # Train: find best strategy + breadth threshold combo
            best_train_score = -1
            best_train_strat = None
            best_train_bmax = 1.0
            best_train_pf = 0

            for strat in STRATEGIES:
                for bmax in BREADTH_THRESHOLDS:
                    r = run_backtest(train_df, strat, train_breadth, bmax)
                    if r and r['trades'] >= 3:
                        score = r['pf'] * (r['wr'] / 100) ** 0.5
                        if score > best_train_score:
                            best_train_score = score
                            best_train_strat = strat
                            best_train_bmax = bmax
                            best_train_pf = r['pf']

            if best_train_strat is None:
                continue

            # Test: run with FIXED breadth threshold (not trained one)
            test_result_fixed = run_backtest(test_df, best_train_strat, test_breadth, BREADTH_MAX_FIXED)
            # Also test without breadth filter for comparison
            test_result_none = run_backtest(test_df, best_train_strat, None, 1.0)
            # Also test with assigned strategy + fixed breadth
            assigned = OPTIMAL_STRAT[coin]
            test_assigned = run_backtest(test_df, assigned, test_breadth, BREADTH_MAX_FIXED)

            coin_windows.append({
                'window': w['name'],
                'train_best_strat': best_train_strat,
                'train_best_bmax': best_train_bmax,
                'train_pf': best_train_pf,
                'test_pf_fixed': test_result_fixed['pf'] if test_result_fixed else 0,
                'test_wr_fixed': test_result_fixed['wr'] if test_result_fixed else 0,
                'test_pf_no_breadth': test_result_none['pf'] if test_result_none else 0,
                'test_wr_no_breadth': test_result_none['wr'] if test_result_none else 0,
                'assigned_pf': test_assigned['pf'] if test_assigned else 0,
                'assigned_wr': test_assigned['wr'] if test_assigned else 0,
            })

        if coin_windows:
            all_results[coin] = coin_windows

    # === PRINT RESULTS ===
    print(f"\n{'='*90}")
    print("WALK-FORWARD RESULTS BY COIN (with breadth filter)")
    print(f"{'='*90}")

    for coin, wins in all_results.items():
        assigned = OPTIMAL_STRAT[coin]
        print(f"\n{coin} (assigned: {assigned})")
        print(f"  {'Win':<4} {'Train Strat':<16} {'Train Bmax':<11} {'Train PF':<10} "
              f"{'Test PF(B20)':<13} {'Test WR(B20)':<13} {'Test PF(noB)':<13} {'Assgn PF'}")
        print(f"  {'-'*105}")
        for w in wins:
            bmax_str = f"{w['train_best_bmax']*100:.0f}%" if w['train_best_bmax'] < 1.0 else "OFF"
            print(f"  {w['window']:<4} {w['train_best_strat']:<16} {bmax_str:<11} "
                  f"{w['train_pf']:<10.2f} {w['test_pf_fixed']:<13.2f} "
                  f"{w['test_wr_fixed']:<13.1f} {w['test_pf_no_breadth']:<13.2f} "
                  f"{w['assigned_pf']:.2f}")

    # === DEGRADATION ANALYSIS ===
    print(f"\n{'='*90}")
    print("BREADTH FILTER DEGRADATION ANALYSIS")
    print(f"{'='*90}")

    train_pfs = []
    test_pfs_fixed = []
    test_pfs_none = []

    for coin, wins in all_results.items():
        for w in wins:
            train_pfs.append(w['train_pf'])
            test_pfs_fixed.append(w['test_pf_fixed'])
            test_pfs_none.append(w['test_pf_no_breadth'])

    avg_train = np.mean(train_pfs) if train_pfs else 0
    avg_test_fixed = np.mean(test_pfs_fixed) if test_pfs_fixed else 0
    avg_test_none = np.mean(test_pfs_none) if test_pfs_none else 0
    degradation_fixed = (1 - avg_test_fixed / avg_train) * 100 if avg_train > 0 else 0
    degradation_none = (1 - avg_test_none / avg_train) * 100 if avg_train > 0 else 0

    print(f"\n  Avg Train PF:              {avg_train:.2f}")
    print(f"  Avg Test PF (B<=20%):      {avg_test_fixed:.2f}  (degradation: {degradation_fixed:.1f}%)")
    print(f"  Avg Test PF (no breadth):  {avg_test_none:.2f}  (degradation: {degradation_none:.1f}%)")

    # Breadth filter improvement
    breadth_improvement = avg_test_fixed - avg_test_none
    print(f"\n  Breadth filter impact on OOS: {breadth_improvement:+.2f} PF")

    if degradation_fixed < 20:
        verdict = "LOW"
    elif degradation_fixed < 40:
        verdict = "MODERATE"
    else:
        verdict = "HIGH"

    print(f"\n  Verdict: {verdict} overfitting risk for breadth filter (BREADTH_MAX={BREADTH_MAX_FIXED*100:.0f}%)")

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

    print(f"\n  Consistent coins: {consistent}/{total_coins} ({consistent/total_coins*100:.0f}%)")

    # Save results
    save_data = {
        'breadth_max_fixed': BREADTH_MAX_FIXED,
        'avg_train_pf': avg_train,
        'avg_test_pf_with_breadth': avg_test_fixed,
        'avg_test_pf_no_breadth': avg_test_none,
        'degradation_pct': degradation_fixed,
        'verdict': verdict,
        'consistent_coins': consistent,
        'total_coins': total_coins,
        'coin_results': {coin: wins for coin, wins in all_results.items()},
    }

    with open('/home/scamarena/ProjectCoin/run5_1_results.json', 'w') as f:
        json.dump(save_data, f, indent=2)

    print(f"\nResults saved to run5_1_results.json")


if __name__ == "__main__":
    main()
