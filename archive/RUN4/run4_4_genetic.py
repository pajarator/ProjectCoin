#!/usr/bin/env python3
"""
RUN4.4 - Genetic Algorithm Strategy Optimization
Evolve strategy parameters to find better combinations.
Population: 20, Generations: 10, Selection: top 50% by PF.
"""
import pandas as pd
import numpy as np
import json
import os
import random

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

LEVERAGE = 5
INITIAL_CAPITAL = 100
RISK = 0.10

# Parameter ranges for mutation
PARAM_RANGES = {
    'stop_loss': (0.002, 0.02),        # 0.2% - 2%
    'min_hold': (1, 16),               # 1-16 candles
    'z_threshold': (-2.5, -0.5),       # z-score entry
    'bb_margin': (1.0, 1.05),          # BB lower band margin
    'vol_mult': (0.8, 2.0),            # volume multiplier
    'adr_pct': (0.15, 0.40),           # ADR reversal percentile
    'exit_z': (0.0, 1.5),             # z-score exit threshold
}

def random_individual():
    return {
        'stop_loss': round(random.uniform(*PARAM_RANGES['stop_loss']), 4),
        'min_hold': random.randint(*PARAM_RANGES['min_hold']),
        'z_threshold': round(random.uniform(*PARAM_RANGES['z_threshold']), 2),
        'bb_margin': round(random.uniform(*PARAM_RANGES['bb_margin']), 3),
        'vol_mult': round(random.uniform(*PARAM_RANGES['vol_mult']), 2),
        'adr_pct': round(random.uniform(*PARAM_RANGES['adr_pct']), 2),
        'exit_z': round(random.uniform(*PARAM_RANGES['exit_z']), 2),
    }

def seed_individual():
    """The current RUN4.1/4.2 optimal params as a seed."""
    return {
        'stop_loss': 0.005,
        'min_hold': 2,
        'z_threshold': -1.5,
        'bb_margin': 1.02,
        'vol_mult': 1.2,
        'adr_pct': 0.25,
        'exit_z': 0.5,
    }

def crossover(parent1, parent2):
    child = {}
    for key in parent1:
        child[key] = parent1[key] if random.random() < 0.5 else parent2[key]
    return child

def mutate(individual, mutation_rate=0.3):
    ind = individual.copy()
    for key in ind:
        if random.random() < mutation_rate:
            lo, hi = PARAM_RANGES[key]
            if isinstance(lo, int):
                ind[key] = random.randint(lo, hi)
            else:
                # Small gaussian perturbation
                range_size = hi - lo
                ind[key] = round(ind[key] + random.gauss(0, range_size * 0.2), 4)
                ind[key] = max(lo, min(hi, ind[key]))
    return ind

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

def entry_signal(row, strategy, params):
    if pd.isna(row.get('z')) or pd.isna(row.get('v')):
        return False
    if strategy == 'mean_reversion':
        return row['z'] < params['z_threshold']
    elif strategy == 'vwap_reversion':
        return (row['z'] < params['z_threshold'] and
                row['c'] < row['sma20'] and
                row['v'] > row['vol_ma'] * params['vol_mult'])
    elif strategy == 'bb_bounce':
        return (row['c'] <= row['bb_lo'] * params['bb_margin'] and
                row['v'] > row['vol_ma'] * (params['vol_mult'] + 0.1))
    elif strategy == 'adr_reversal':
        return row['c'] <= row['adr_lo'] + (row['adr_hi'] - row['adr_lo']) * params['adr_pct']
    elif strategy == 'dual_rsi':
        return row['z'] < params['z_threshold'] + 0.5
    return False

# Strategy assignments from RUN4.2
OPTIMAL = {
    'DASH': 'vwap_reversion', 'UNI': 'vwap_reversion', 'NEAR': 'vwap_reversion',
    'ADA': 'vwap_reversion', 'LTC': 'vwap_reversion', 'SHIB': 'vwap_reversion',
    'LINK': 'vwap_reversion', 'ETH': 'vwap_reversion', 'DOT': 'vwap_reversion',
    'XRP': 'vwap_reversion', 'ATOM': 'vwap_reversion', 'SOL': 'vwap_reversion',
    'DOGE': 'bb_bounce', 'XLM': 'dual_rsi', 'AVAX': 'adr_reversal',
    'ALGO': 'adr_reversal', 'BNB': 'vwap_reversion', 'BTC': 'bb_bounce',
}

def run_backtest(df, strategy, params):
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

            if price_pnl <= -params['stop_loss']:
                loss = balance * RISK * params['stop_loss'] * LEVERAGE
                balance -= loss
                trades.append(-params['stop_loss'] * LEVERAGE * 100)
                position = None
                cooldown = 3
                candles_held = 0
                continue

            if price_pnl > 0 and candles_held >= params['min_hold']:
                if row['c'] > row['sma20']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append(price_pnl * LEVERAGE * 100)
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue
                if row['z'] > params['exit_z']:
                    profit = balance * RISK * price_pnl * LEVERAGE
                    balance += profit
                    trades.append(price_pnl * LEVERAGE * 100)
                    position = None
                    cooldown = 3
                    candles_held = 0
                    continue

        if cooldown > 0:
            cooldown -= 1

        if not position and cooldown == 0:
            if entry_signal(row, strategy, params):
                position = True
                entry_price = price

    if position:
        price_pnl = (df.iloc[-1]['c'] - entry_price) / entry_price
        profit = balance * RISK * price_pnl * LEVERAGE
        balance += profit
        trades.append(price_pnl * LEVERAGE * 100)

    if not trades:
        return {'pf': 0, 'pnl': 0, 'trades': 0, 'wr': 0}

    wins = [t for t in trades if t > 0]
    losses = [t for t in trades if t <= 0]
    total_win = sum(wins) if wins else 0
    total_loss = sum(losses) if losses else 0
    pf = abs(total_win / total_loss) if total_loss != 0 else 0

    return {
        'pf': pf,
        'pnl': (balance - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100,
        'trades': len(trades),
        'wr': len(wins) / len(trades) * 100,
    }

def evaluate(individual, coin_data):
    """Evaluate individual across all coins. Returns avg PF."""
    pfs = []
    for coin, df in coin_data.items():
        strategy = OPTIMAL[coin]
        result = run_backtest(df, strategy, individual)
        if result and result['pf'] > 0:
            pfs.append(result['pf'])
    return np.mean(pfs) if pfs else 0

def evaluate_detailed(individual, coin_data):
    """Detailed evaluation for reporting."""
    results = {}
    for coin, df in coin_data.items():
        strategy = OPTIMAL[coin]
        result = run_backtest(df, strategy, individual)
        if result:
            results[coin] = result
    return results

def main():
    print("=" * 80)
    print("RUN4.4 - GENETIC ALGORITHM OPTIMIZATION")
    print("=" * 80)
    print("Evolving strategy parameters across all coins")
    print("Population: 20 | Generations: 10 | Selection: top 50%")
    print("=" * 80)

    # Load all data
    print("\nLoading data...")
    coin_data = {}
    for coin in COINS:
        df = load_cache(coin)
        if df is not None:
            coin_data[coin] = df
    print(f"Loaded {len(coin_data)} coins")

    # Initialize population: seed + 19 random
    population = [seed_individual()]
    for _ in range(19):
        population.append(random_individual())

    # Evaluate seed
    seed_fitness = evaluate(population[0], coin_data)
    print(f"\nSeed (current params): Avg PF = {seed_fitness:.3f}")
    print(f"  Params: {population[0]}")

    best_ever = None
    best_ever_fitness = 0

    for gen in range(10):
        # Evaluate all
        fitnesses = [(ind, evaluate(ind, coin_data)) for ind in population]
        fitnesses.sort(key=lambda x: -x[1])

        gen_best = fitnesses[0]
        gen_avg = np.mean([f[1] for f in fitnesses])
        gen_worst = fitnesses[-1][1]

        if gen_best[1] > best_ever_fitness:
            best_ever = gen_best[0].copy()
            best_ever_fitness = gen_best[1]

        print(f"\nGen {gen+1:2d}: Best={gen_best[1]:.3f} Avg={gen_avg:.3f} Worst={gen_worst:.3f}")
        print(f"  Best params: SL={gen_best[0]['stop_loss']:.4f} Hold={gen_best[0]['min_hold']} "
              f"Z={gen_best[0]['z_threshold']:.2f} BB={gen_best[0]['bb_margin']:.3f} "
              f"Vol={gen_best[0]['vol_mult']:.2f} ADR={gen_best[0]['adr_pct']:.2f} "
              f"ExitZ={gen_best[0]['exit_z']:.2f}")

        # Selection: top 50%
        survivors = [ind for ind, _ in fitnesses[:10]]

        # Create next generation
        next_pop = [best_ever.copy()]  # elitism
        while len(next_pop) < 20:
            p1 = random.choice(survivors)
            p2 = random.choice(survivors)
            child = crossover(p1, p2)
            child = mutate(child)
            next_pop.append(child)

        population = next_pop

    # === FINAL RESULTS ===
    print(f"\n{'='*80}")
    print("BEST EVOLVED PARAMETERS")
    print(f"{'='*80}")
    print(f"\nParams:")
    for k, v in best_ever.items():
        seed_val = seed_individual()[k]
        change = ""
        if isinstance(v, float):
            change = f" (was {seed_val}, {'+'if v>seed_val else ''}{v-seed_val:.4f})"
        else:
            change = f" (was {seed_val}, {'+'if v>seed_val else ''}{v-seed_val})"
        print(f"  {k}: {v}{change}")

    print(f"\nAvg PF: {best_ever_fitness:.3f} (seed: {seed_fitness:.3f}, change: {best_ever_fitness-seed_fitness:+.3f})")

    # Detailed per-coin comparison
    print(f"\n{'='*80}")
    print("PER-COIN COMPARISON: Seed vs Evolved")
    print(f"{'='*80}")

    seed_results = evaluate_detailed(seed_individual(), coin_data)
    evolved_results = evaluate_detailed(best_ever, coin_data)

    print(f"\n{'Coin':<8} {'Strategy':<16} {'Seed PF':<10} {'Evolved PF':<12} {'Change':<10} {'Seed WR':<10} {'Evol WR'}")
    print("-" * 80)

    for coin in COINS:
        if coin not in seed_results or coin not in evolved_results:
            continue
        sr = seed_results[coin]
        er = evolved_results[coin]
        change = er['pf'] - sr['pf']
        marker = "+" if change > 0 else ""
        print(f"{coin:<8} {OPTIMAL[coin]:<16} {sr['pf']:<10.2f} {er['pf']:<12.2f} {marker}{change:<+9.2f} {sr['wr']:<10.1f} {er['wr']:.1f}")

    # Summary
    seed_avg_pf = np.mean([r['pf'] for r in seed_results.values()])
    evolved_avg_pf = np.mean([r['pf'] for r in evolved_results.values()])
    seed_avg_pnl = np.mean([r['pnl'] for r in seed_results.values()])
    evolved_avg_pnl = np.mean([r['pnl'] for r in evolved_results.values()])

    print("-" * 80)
    print(f"{'AVG':<8} {'':<16} {seed_avg_pf:<10.2f} {evolved_avg_pf:<12.2f} {evolved_avg_pf-seed_avg_pf:+.2f}")
    print(f"\nAvg P&L: Seed={seed_avg_pnl:+.1f}% Evolved={evolved_avg_pnl:+.1f}%")

    # Save
    output = {
        'seed_params': seed_individual(),
        'evolved_params': best_ever,
        'seed_fitness': seed_fitness,
        'evolved_fitness': best_ever_fitness,
        'seed_results': seed_results,
        'evolved_results': evolved_results,
    }
    with open('/home/scamarena/ProjectCoin/genetic_results.json', 'w') as f:
        json.dump(output, f, indent=2)

    print(f"\nResults saved to genetic_results.json")

    # Print ready-to-use params
    print(f"\n{'='*80}")
    print("RECOMMENDED PARAMS FOR trader.py")
    print(f"{'='*80}")
    print(f"STOP_LOSS = {best_ever['stop_loss']}")
    print(f"MIN_HOLD_CANDLES = {best_ever['min_hold']}")
    print(f"# Entry: z_threshold={best_ever['z_threshold']}, bb_margin={best_ever['bb_margin']}, vol_mult={best_ever['vol_mult']}, adr_pct={best_ever['adr_pct']}")
    print(f"# Exit: exit_z={best_ever['exit_z']}")


if __name__ == "__main__":
    main()
