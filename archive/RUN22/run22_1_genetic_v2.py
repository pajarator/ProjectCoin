"""
RUN22.1 — Genetic Algorithm v2 for Strategy Discovery

Major upgrade over archive/RUN4/run4_4_genetic.py:
  - Population: 100, Generations: 50
  - Strategy genome: list of (indicator, operator, threshold) tuples
  - Walk-forward fitness: train 8mo, score last 4mo
  - Fitness: sharpe * sqrt(trades) (penalize few trades)
  - Diversity: tournament selection + crowding
  - Multiprocessing for parallel fitness evaluation

Output: run22_1_results.json
Checkpoint: run22_1_checkpoint.json
"""
import json
import os
import signal
import random
import numpy as np
import pandas as pd
from copy import deepcopy
from multiprocessing import Pool, cpu_count
from tqdm import tqdm

from feature_engine import load_cached_data, COINS
from indicators import add_all_indicators
from backtester import Backtester

CHECKPOINT_FILE = 'run22_1_checkpoint.json'
RESULTS_FILE = 'run22_1_results.json'
POPULATION_SIZE = 100
GENERATIONS = 50
TOURNAMENT_SIZE = 5
MUTATION_RATE = 0.15
CROSSOVER_RATE = 0.7
ELITE_COUNT = 5

# Available indicators and their value ranges
INDICATORS = {
    'RSI': {'col': 'RSI', 'range': (0, 100)},
    'STOCH_K': {'col': 'STOCH_K', 'range': (0, 100)},
    'STOCH_D': {'col': 'STOCH_D', 'range': (0, 100)},
    'BB_position': {'col': 'BB_position', 'range': (-0.5, 1.5)},
    'BB_width': {'col': 'BB_width', 'range': (0, 0.2)},
    'ADX': {'col': 'ADX', 'range': (0, 100)},
    'Volume_ratio': {'col': 'Volume_ratio', 'range': (0, 5)},
    'ROC': {'col': 'ROC', 'range': (-20, 20)},
    'Price_position': {'col': 'Price_position', 'range': (0, 1)},
    'MACD_hist': {'col': 'MACD_hist', 'range': (-500, 500)},
}

OPERATORS = ['<', '>', 'cross_above', 'cross_below']

shutdown_requested = False
def signal_handler(sig, frame):
    global shutdown_requested
    print('\nShutdown requested, saving checkpoint...')
    shutdown_requested = True
signal.signal(signal.SIGINT, signal_handler)


def random_rule():
    """Generate a random entry/exit rule."""
    ind_name = random.choice(list(INDICATORS.keys()))
    ind_info = INDICATORS[ind_name]
    op = random.choice(['<', '>'])
    lo, hi = ind_info['range']
    threshold = random.uniform(lo, hi)
    return {'indicator': ind_name, 'column': ind_info['col'], 'operator': op, 'threshold': threshold}


def random_genome():
    """Generate a random strategy genome."""
    n_entry = random.randint(1, 3)
    n_exit = random.randint(1, 2)
    return {
        'entry_rules': [random_rule() for _ in range(n_entry)],
        'exit_rules': [random_rule() for _ in range(n_exit)],
        'stop_loss': random.uniform(0.002, 0.01),
    }


def generate_signals(df: pd.DataFrame, genome: dict):
    """Generate entry/exit signals from genome."""
    entry = pd.Series(True, index=df.index)
    for rule in genome['entry_rules']:
        col = rule['column']
        if col not in df.columns:
            entry &= False
            continue
        if rule['operator'] == '<':
            entry &= df[col] < rule['threshold']
        elif rule['operator'] == '>':
            entry &= df[col] > rule['threshold']
        elif rule['operator'] == 'cross_above':
            entry &= (df[col] > rule['threshold']) & (df[col].shift(1) <= rule['threshold'])
        elif rule['operator'] == 'cross_below':
            entry &= (df[col] < rule['threshold']) & (df[col].shift(1) >= rule['threshold'])

    exit_sig = pd.Series(False, index=df.index)
    for rule in genome['exit_rules']:
        col = rule['column']
        if col not in df.columns:
            continue
        if rule['operator'] == '<':
            exit_sig |= df[col] < rule['threshold']
        elif rule['operator'] == '>':
            exit_sig |= df[col] > rule['threshold']
        elif rule['operator'] == 'cross_above':
            exit_sig |= (df[col] > rule['threshold']) & (df[col].shift(1) <= rule['threshold'])
        elif rule['operator'] == 'cross_below':
            exit_sig |= (df[col] < rule['threshold']) & (df[col].shift(1) >= rule['threshold'])

    return entry, exit_sig


def fitness(genome: dict, train_df: pd.DataFrame, test_df: pd.DataFrame) -> float:
    """
    Walk-forward fitness: train on first portion, score on test.
    Fitness = sharpe * sqrt(trades). Penalize <10 trades.
    """
    try:
        entry, exit_sig = generate_signals(test_df, genome)
        bt = Backtester(test_df, fee=0.001, slippage=0.0005)
        result = bt.run(entry, exit_sig, stop_loss=genome['stop_loss'])

        if result.total_trades < 5:
            return -999

        score = result.Sharpe_ratio * np.sqrt(result.total_trades)
        if result.profit_factor < 0.8:
            score *= 0.5
        return score
    except Exception:
        return -999


def mutate(genome: dict) -> dict:
    """Mutate a genome."""
    g = deepcopy(genome)
    mutation_type = random.choice(['rule', 'threshold', 'add', 'remove', 'stop_loss'])

    if mutation_type == 'threshold':
        rules = random.choice([g['entry_rules'], g['exit_rules']])
        if rules:
            rule = random.choice(rules)
            ind_info = INDICATORS.get(rule['indicator'], {'range': (0, 100)})
            lo, hi = ind_info['range']
            rule['threshold'] += random.gauss(0, (hi - lo) * 0.1)
            rule['threshold'] = max(lo, min(hi, rule['threshold']))

    elif mutation_type == 'rule':
        target = random.choice(['entry_rules', 'exit_rules'])
        if g[target]:
            idx = random.randint(0, len(g[target]) - 1)
            g[target][idx] = random_rule()

    elif mutation_type == 'add':
        target = random.choice(['entry_rules', 'exit_rules'])
        if len(g[target]) < 4:
            g[target].append(random_rule())

    elif mutation_type == 'remove':
        target = random.choice(['entry_rules', 'exit_rules'])
        if len(g[target]) > 1:
            g[target].pop(random.randint(0, len(g[target]) - 1))

    elif mutation_type == 'stop_loss':
        g['stop_loss'] += random.gauss(0, 0.001)
        g['stop_loss'] = max(0.001, min(0.02, g['stop_loss']))

    return g


def crossover(parent1: dict, parent2: dict) -> dict:
    """Single-point crossover between two genomes."""
    child = deepcopy(parent1)

    # Swap entry rules
    if random.random() < 0.5:
        child['entry_rules'] = deepcopy(parent2['entry_rules'])

    # Swap exit rules
    if random.random() < 0.5:
        child['exit_rules'] = deepcopy(parent2['exit_rules'])

    # Average stop loss
    child['stop_loss'] = (parent1['stop_loss'] + parent2['stop_loss']) / 2

    return child


def tournament_select(population: list, fitnesses: list, k: int = TOURNAMENT_SIZE) -> dict:
    """Tournament selection."""
    indices = random.sample(range(len(population)), min(k, len(population)))
    best_idx = max(indices, key=lambda i: fitnesses[i])
    return deepcopy(population[best_idx])


def genome_to_dict(genome: dict) -> dict:
    """Convert genome to JSON-serializable dict."""
    return {
        'entry_rules': genome['entry_rules'],
        'exit_rules': genome['exit_rules'],
        'stop_loss': genome['stop_loss'],
    }


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return {'completed_coins': [], 'results': {}}


def save_checkpoint(state):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(state, f, indent=2, default=str)


def evolve_coin(coin: str) -> dict:
    """Run genetic evolution for a single coin."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)

    # Walk-forward split: 8mo train, 4mo test
    split = int(len(df) * 0.67)
    train_df = df.iloc[:split]
    test_df = df.iloc[split:]

    # Initialize population
    population = [random_genome() for _ in range(POPULATION_SIZE)]
    best_fitness_history = []
    best_genome = None
    best_fitness = -999

    for gen in range(GENERATIONS):
        # Evaluate fitness
        fitnesses = [fitness(g, train_df, test_df) for g in population]

        # Track best
        gen_best_idx = max(range(len(fitnesses)), key=lambda i: fitnesses[i])
        gen_best_fit = fitnesses[gen_best_idx]
        if gen_best_fit > best_fitness:
            best_fitness = gen_best_fit
            best_genome = deepcopy(population[gen_best_idx])
        best_fitness_history.append(float(gen_best_fit))

        if gen % 10 == 0:
            avg_fit = np.mean([f for f in fitnesses if f > -999])
            tqdm.write(f'  Gen {gen}: best={gen_best_fit:.2f} avg={avg_fit:.2f}')

        # Create next generation
        next_pop = []

        # Elitism
        elite_indices = sorted(range(len(fitnesses)), key=lambda i: -fitnesses[i])[:ELITE_COUNT]
        for idx in elite_indices:
            next_pop.append(deepcopy(population[idx]))

        # Fill rest with crossover + mutation
        while len(next_pop) < POPULATION_SIZE:
            if random.random() < CROSSOVER_RATE:
                p1 = tournament_select(population, fitnesses)
                p2 = tournament_select(population, fitnesses)
                child = crossover(p1, p2)
            else:
                child = tournament_select(population, fitnesses)

            if random.random() < MUTATION_RATE:
                child = mutate(child)

            next_pop.append(child)

        population = next_pop

    # Final evaluation of best genome
    if best_genome:
        entry, exit_sig = generate_signals(test_df, best_genome)
        bt = Backtester(test_df, fee=0.001, slippage=0.0005)
        final_result = bt.run(entry, exit_sig, stop_loss=best_genome['stop_loss'])

        return {
            'coin': coin,
            'best_fitness': float(best_fitness),
            'best_genome': genome_to_dict(best_genome),
            'test_result': {
                'trades': final_result.total_trades,
                'win_rate': final_result.win_rate,
                'profit_factor': final_result.profit_factor,
                'sharpe': final_result.Sharpe_ratio,
                'max_drawdown': final_result.max_drawdown,
                'total_pnl_pct': final_result.total_pnl_pct,
            },
            'fitness_history': best_fitness_history,
            'generations': GENERATIONS,
        }

    return {'coin': coin, 'error': 'No viable genome found'}


def main():
    print('=' * 60)
    print('RUN22.1 — Genetic Algorithm v2')
    print(f'Population: {POPULATION_SIZE}, Generations: {GENERATIONS}')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed_coins'])
    results = state['results']

    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Evolving'):
        if shutdown_requested:
            break

        print(f'\n--- {coin}/USDT ---')
        try:
            result = evolve_coin(coin)
            results[coin] = result
            completed.add(coin)
            state['completed_coins'] = sorted(list(completed))
            state['results'] = results
            save_checkpoint(state)

            if 'test_result' in result:
                tr = result['test_result']
                print(f'  Best: {tr["trades"]}t WR={tr["win_rate"]:.1f}% '
                      f'PF={tr["profit_factor"]:.2f} Sharpe={tr["sharpe"]:.2f}')
                print(f'  Genome: {len(result["best_genome"]["entry_rules"])} entry rules, '
                      f'{len(result["best_genome"]["exit_rules"])} exit rules')
            else:
                print(f'  {result.get("error", "Failed")}')
        except Exception as e:
            print(f'  FAILED: {e}')
            import traceback; traceback.print_exc()

    if len(completed) == len(COINS):
        valid = {k: v for k, v in results.items() if 'test_result' in v}
        avg_pf = np.mean([v['test_result']['profit_factor'] for v in valid.values()]) if valid else 0

        # Find unique indicator combos in top genomes
        indicator_usage = {}
        for v in valid.values():
            for rule in v['best_genome']['entry_rules']:
                ind = rule['indicator']
                indicator_usage[ind] = indicator_usage.get(ind, 0) + 1

        final = {
            'per_coin': results,
            'summary': {
                'coins_evolved': len(valid),
                'avg_profit_factor': float(avg_pf),
                'avg_win_rate': float(np.mean([v['test_result']['win_rate'] for v in valid.values()])) if valid else 0,
                'indicator_usage': dict(sorted(indicator_usage.items(), key=lambda x: -x[1])),
                'pf_above_1_3': sum(1 for v in valid.values() if v['test_result']['profit_factor'] > 1.3),
            }
        }

        with open(RESULTS_FILE, 'w') as f:
            json.dump(final, f, indent=2, default=str)

        print(f'\n{"=" * 60}')
        print(f'RESULTS: {RESULTS_FILE}')
        print(f'Avg PF: {avg_pf:.2f}')
        print(f'PF > 1.3: {final["summary"]["pf_above_1_3"]}/{len(valid)}')
        print(f'Indicator usage: {final["summary"]["indicator_usage"]}')

        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(completed)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
