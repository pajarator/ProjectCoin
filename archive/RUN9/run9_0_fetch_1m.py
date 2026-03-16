#!/usr/bin/env python3
"""
RUN9.0 - Fetch 1-minute candle data for all 18 coins (5 months).

Fetches ~216,000 candles per coin via paginated Binance API calls (1000/request).
Saves to data_cache/{COIN}_USDT_1m_5months.csv.
Includes checkpoint/resume so it can be interrupted and restarted.
"""
import ccxt
import pandas as pd
import time
import os
import json
import signal
import sys
from datetime import datetime

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/run9_0_checkpoint.json'

COINS = ['DASH', 'UNI', 'NEAR', 'ADA', 'LTC', 'SHIB', 'LINK', 'ETH',
         'DOT', 'XRP', 'ATOM', 'SOL', 'DOGE', 'XLM', 'AVAX', 'ALGO', 'BNB', 'BTC']

# 5 months back from ~Mar 14 2026 = Oct 15 2025
START_DATE = '2025-10-15 00:00:00'
START_TS = int(pd.Timestamp(START_DATE).timestamp() * 1000)

BATCH_SIZE = 1000  # Binance max per request
TIMEFRAME = '1m'
CANDLE_MS = 60 * 1000  # 1 minute in ms

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, will stop after current batch...")


signal.signal(signal.SIGINT, _sigint_handler)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return {'completed_coins': [], 'partial': None}


def save_checkpoint(data):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(data, f)


def fetch_coin(exchange, symbol, coin_name):
    """Fetch all 1m candles for a coin from START_DATE to now."""
    output_file = f"{DATA_CACHE_DIR}/{coin_name}_USDT_1m_5months.csv"

    all_candles = []
    since = START_TS
    now_ts = int(time.time() * 1000)

    # Estimate total batches
    total_candles_est = (now_ts - START_TS) // CANDLE_MS
    total_batches_est = total_candles_est // BATCH_SIZE + 1

    batch_num = 0
    while since < now_ts:
        if _shutdown:
            return False

        try:
            ohlcv = exchange.fetch_ohlcv(symbol, TIMEFRAME, since=since, limit=BATCH_SIZE)
        except Exception as e:
            print(f"  Error fetching {symbol} batch {batch_num}: {e}")
            time.sleep(5)
            continue

        if not ohlcv:
            break

        all_candles.extend(ohlcv)

        # Move since to after the last candle
        last_ts = ohlcv[-1][0]
        since = last_ts + CANDLE_MS

        batch_num += 1
        if batch_num % 10 == 0:
            pct = min(100, batch_num / total_batches_est * 100)
            print(f"  {coin_name}: batch {batch_num}/{total_batches_est} ({pct:.0f}%) - {len(all_candles)} candles")

        # Small delay to be nice to the API
        time.sleep(0.1)

        # If we got fewer than BATCH_SIZE, we've reached the end
        if len(ohlcv) < BATCH_SIZE:
            break

    if not all_candles:
        print(f"  {coin_name}: No data fetched!")
        return True

    # Build DataFrame
    df = pd.DataFrame(all_candles, columns=['t', 'o', 'h', 'l', 'c', 'v'])
    df['t'] = pd.to_datetime(df['t'], unit='ms')
    df.set_index('t', inplace=True)

    # Remove duplicates
    df = df[~df.index.duplicated(keep='first')]
    df.sort_index(inplace=True)

    df.to_csv(output_file)
    print(f"  {coin_name}: Saved {len(df)} candles to {output_file}")
    print(f"    Range: {df.index[0]} to {df.index[-1]}")

    return True


def main():
    print("=" * 80)
    print("RUN9.0 - FETCH 1-MINUTE CANDLE DATA (5 MONTHS)")
    print("=" * 80)
    print(f"Coins: {len(COINS)}")
    print(f"Start: {START_DATE}")
    print(f"Estimated: ~216,000 candles/coin, ~216 requests/coin")
    print("=" * 80)

    exchange = ccxt.binance({'enableRateLimit': True})

    checkpoint = load_checkpoint()
    completed = set(checkpoint['completed_coins'])

    if completed:
        print(f"Resuming: {len(completed)}/{len(COINS)} coins already done")

    start_time = time.time()

    for i, coin in enumerate(COINS):
        if coin in completed:
            print(f"[{i+1}/{len(COINS)}] {coin}: already cached, skipping")
            continue

        if _shutdown:
            print("Shutdown requested. Saving checkpoint...")
            save_checkpoint({'completed_coins': list(completed), 'partial': None})
            sys.exit(0)

        print(f"\n[{i+1}/{len(COINS)}] Fetching {coin}/USDT 1m...")
        symbol = f"{coin}/USDT"

        success = fetch_coin(exchange, symbol, coin)
        if success:
            completed.add(coin)
            save_checkpoint({'completed_coins': list(completed), 'partial': None})

        elapsed = time.time() - start_time
        done = len(completed)
        remaining = len(COINS) - done
        if done > 0:
            rate = elapsed / done
            eta = rate * remaining
            print(f"  Progress: {done}/{len(COINS)} coins | Elapsed: {elapsed/60:.1f}m | ETA: {eta/60:.1f}m")

    print(f"\n{'='*80}")
    print("COMPLETE")
    print(f"{'='*80}")

    # Verify
    for coin in COINS:
        path = f"{DATA_CACHE_DIR}/{coin}_USDT_1m_5months.csv"
        if os.path.exists(path):
            df = pd.read_csv(path)
            print(f"  {coin}: {len(df)} candles")
        else:
            print(f"  {coin}: MISSING!")

    # Clean up checkpoint
    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("\nCheckpoint removed (clean finish)")


if __name__ == "__main__":
    main()
