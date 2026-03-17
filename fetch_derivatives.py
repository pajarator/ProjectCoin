"""
Batch fetch 1 year of funding rates + daily OI for all 19 coins.
Saves to data_cache/derivatives/.

Checkpoint: fetch_derivatives_checkpoint.json
"""
import json
import os
import signal
import pandas as pd
from tqdm import tqdm

from data_fetcher_extended import fetch_funding_rate_history, fetch_open_interest_history
from feature_engine import COINS

CACHE_DIR = 'data_cache/derivatives'
CHECKPOINT_FILE = 'fetch_derivatives_checkpoint.json'

shutdown_requested = False
def signal_handler(sig, frame):
    global shutdown_requested
    print('\nShutdown requested, saving checkpoint...')
    shutdown_requested = True
signal.signal(signal.SIGINT, signal_handler)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return {'completed': []}


def save_checkpoint(state):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(state, f)


def main():
    os.makedirs(CACHE_DIR, exist_ok=True)

    print('=' * 60)
    print('Fetching Derivatives Data (Funding Rates + OI)')
    print('=' * 60)

    state = load_checkpoint()
    completed = set(state['completed'])
    remaining = [c for c in COINS if c not in completed]
    print(f'{len(completed)} done, {len(remaining)} remaining\n')

    for coin in tqdm(remaining, desc='Fetching'):
        if shutdown_requested:
            break

        symbol = f'{coin}/USDT'
        print(f'\n--- {symbol} ---')

        # Funding rates
        fr_path = os.path.join(CACHE_DIR, f'{coin}_USDT_funding.csv')
        if not os.path.exists(fr_path):
            try:
                fr = fetch_funding_rate_history(symbol, days=365)
                if not fr.empty:
                    fr.to_csv(fr_path)
                    print(f'  Funding: {len(fr)} records')
                else:
                    print(f'  Funding: no data')
            except Exception as e:
                print(f'  Funding error: {e}')

        # Open interest
        oi_path = os.path.join(CACHE_DIR, f'{coin}_USDT_oi.csv')
        if not os.path.exists(oi_path):
            try:
                oi = fetch_open_interest_history(symbol, timeframe='1d', days=365)
                if not oi.empty:
                    oi.to_csv(oi_path)
                    print(f'  OI: {len(oi)} records')
                else:
                    print(f'  OI: no data')
            except Exception as e:
                print(f'  OI error: {e}')

        completed.add(coin)
        state['completed'] = sorted(list(completed))
        save_checkpoint(state)

    if len(completed) == len(COINS):
        print(f'\nAll {len(COINS)} coins fetched.')
        if os.path.exists(CHECKPOINT_FILE):
            os.remove(CHECKPOINT_FILE)
    else:
        print(f'\n{len(completed)}/{len(COINS)} done. Run again to resume.')


if __name__ == '__main__':
    main()
