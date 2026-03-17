#!/usr/bin/env python3
"""
Fetch 1 year of candle data for all 19 coins × 3 timeframes (1m, 5m, 15m).

- Main data: 2025-03-14 to 2026-03-14 → data_cache/{COIN}_USDT_{tf}_1year.csv
- Walk-forward holdout: 2026-03-14 to now → data_cache/walkforward/{COIN}_USDT_{tf}_wf.csv

For coins/timeframes that already have 5-month cache (Oct 15 → Mar 14),
we only fetch the missing earlier period and merge.
"""
import ccxt
import pandas as pd
import time
import os
import json
import signal
import sys

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'
WF_DIR = os.path.join(DATA_CACHE_DIR, 'walkforward')
CHECKPOINT_FILE = '/home/scamarena/ProjectCoin/fetch_1year_checkpoint.json'

COINS = ['ADA', 'ALGO', 'ATOM', 'AVAX', 'BNB', 'BTC', 'DASH', 'DOGE',
         'DOT', 'ETH', 'LINK', 'LTC', 'NEAR', 'SHIB', 'SOL', 'TRX',
         'UNI', 'XLM', 'XRP']

TIMEFRAMES = ['15m', '5m', '1m']

# Main data range: 1 year ending at existing cache end
MAIN_START = '2025-03-14 00:00:00'
MAIN_END   = '2026-03-14 17:15:00'  # existing cache end
MAIN_START_TS = int(pd.Timestamp(MAIN_START).timestamp() * 1000)
MAIN_END_TS   = int(pd.Timestamp(MAIN_END).timestamp() * 1000)

# Existing cache start (we already have data from here onward)
EXISTING_START = '2025-10-15 00:00:00'
EXISTING_START_TS = int(pd.Timestamp(EXISTING_START).timestamp() * 1000)

BATCH_SIZE = 1000

CANDLE_MS = {
    '1m':  60 * 1000,
    '5m':  5 * 60 * 1000,
    '15m': 15 * 60 * 1000,
}

_shutdown = False


def _sigint_handler(sig, frame):
    global _shutdown
    _shutdown = True
    print("\nSIGINT received, stopping after current batch...")


signal.signal(signal.SIGINT, _sigint_handler)


def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return {'completed': [], 'wf_completed': []}


def save_checkpoint(data):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump(data, f)


def fetch_range(exchange, symbol, tf, since_ts, until_ts):
    """Fetch candles in [since_ts, until_ts] via pagination."""
    candle_ms = CANDLE_MS[tf]
    all_candles = []
    since = since_ts
    total_est = (until_ts - since_ts) // candle_ms
    batches_est = max(1, total_est // BATCH_SIZE)
    batch = 0

    while since < until_ts:
        if _shutdown:
            return None

        try:
            ohlcv = exchange.fetch_ohlcv(symbol, tf, since=since, limit=BATCH_SIZE)
        except Exception as e:
            print(f"    Error batch {batch}: {e}, retrying in 5s...")
            time.sleep(5)
            continue

        if not ohlcv:
            break

        # Filter to only candles within our range
        ohlcv = [c for c in ohlcv if c[0] <= until_ts]
        all_candles.extend(ohlcv)

        last_ts = ohlcv[-1][0] if ohlcv else since + candle_ms
        since = last_ts + candle_ms
        batch += 1

        if batch % 20 == 0:
            pct = min(100, batch / batches_est * 100)
            print(f"    batch {batch}/{batches_est} ({pct:.0f}%) — {len(all_candles)} candles")

        time.sleep(0.12)

        if len(ohlcv) < BATCH_SIZE:
            break

    return all_candles


def build_df(candles):
    if not candles:
        return pd.DataFrame()
    df = pd.DataFrame(candles, columns=['t', 'o', 'h', 'l', 'c', 'v'])
    df['t'] = pd.to_datetime(df['t'], unit='ms')
    df.set_index('t', inplace=True)
    df = df[~df.index.duplicated(keep='first')]
    df.sort_index(inplace=True)
    return df


def existing_cache_path(coin, tf):
    """Return path to existing 5-month cache if it exists."""
    path = f"{DATA_CACHE_DIR}/{coin}_USDT_{tf}_5months.csv"
    if os.path.exists(path):
        return path
    return None


def main():
    os.makedirs(WF_DIR, exist_ok=True)

    print("=" * 80)
    print("FETCH 1-YEAR DATA: 19 coins × 3 timeframes")
    print(f"Main:    {MAIN_START} → {MAIN_END}")
    print(f"WF:      {MAIN_END} → now")
    print("=" * 80)

    exchange = ccxt.binance({'enableRateLimit': True})
    cp = load_checkpoint()
    completed = set(cp['completed'])
    wf_completed = set(cp['wf_completed'])

    total_jobs = len(COINS) * len(TIMEFRAMES)
    done_count = len(completed)

    if done_count:
        print(f"Resuming: {done_count}/{total_jobs} main jobs done, {len(wf_completed)} WF done\n")

    start_time = time.time()

    # ── Phase 1: Main data (1 year) ──
    print("\n═══ PHASE 1: MAIN DATA (1 year) ═══\n")
    for coin in COINS:
        for tf in TIMEFRAMES:
            job_key = f"{coin}_{tf}"
            if job_key in completed:
                continue
            if _shutdown:
                save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})
                print("Saved checkpoint, exiting.")
                sys.exit(0)

            symbol = f"{coin}/USDT"
            out_file = f"{DATA_CACHE_DIR}/{coin}_USDT_{tf}_1year.csv"

            print(f"[{len(completed)+1}/{total_jobs}] {coin} {tf} ...")

            existing_path = existing_cache_path(coin, tf)
            if existing_path:
                # We have Oct 15 → Mar 14; just fetch Mar 14 → Oct 15
                print(f"  Existing cache found, fetching backfill (Mar 14 → Oct 15)...")
                new_candles = fetch_range(exchange, symbol, tf, MAIN_START_TS, EXISTING_START_TS - CANDLE_MS[tf])
                if new_candles is None:  # shutdown
                    save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})
                    sys.exit(0)

                new_df = build_df(new_candles)
                old_df = pd.read_csv(existing_path, index_col='t', parse_dates=True)
                # Filter old_df to main range
                old_df = old_df[old_df.index <= pd.Timestamp(MAIN_END)]

                if not new_df.empty:
                    merged = pd.concat([new_df, old_df])
                    merged = merged[~merged.index.duplicated(keep='first')]
                    merged.sort_index(inplace=True)
                else:
                    merged = old_df

                merged.to_csv(out_file)
                print(f"  Saved {len(merged)} candles ({merged.index[0]} → {merged.index[-1]})")
            else:
                # Fetch full year
                print(f"  No existing cache, fetching full year...")
                candles = fetch_range(exchange, symbol, tf, MAIN_START_TS, MAIN_END_TS)
                if candles is None:
                    save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})
                    sys.exit(0)

                df = build_df(candles)
                if df.empty:
                    print(f"  WARNING: No data for {coin} {tf}!")
                else:
                    df.to_csv(out_file)
                    print(f"  Saved {len(df)} candles ({df.index[0]} → {df.index[-1]})")

            completed.add(job_key)
            done_count = len(completed)
            save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})

            elapsed = time.time() - start_time
            remaining = total_jobs - done_count
            if done_count > 0:
                rate = elapsed / done_count
                eta = rate * remaining
                print(f"  Progress: {done_count}/{total_jobs} | Elapsed: {elapsed/60:.1f}m | ETA: {eta/60:.1f}m")

    # ── Phase 2: Walk-forward holdout ──
    print("\n═══ PHASE 2: WALK-FORWARD HOLDOUT ═══\n")
    wf_start_ts = MAIN_END_TS + CANDLE_MS['1m']  # 1 candle after main end
    now_ts = int(time.time() * 1000)

    wf_total = len(COINS) * len(TIMEFRAMES)
    for coin in COINS:
        for tf in TIMEFRAMES:
            job_key = f"{coin}_{tf}"
            if job_key in wf_completed:
                continue
            if _shutdown:
                save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})
                sys.exit(0)

            symbol = f"{coin}/USDT"
            out_file = f"{WF_DIR}/{coin}_USDT_{tf}_wf.csv"

            print(f"[WF {len(wf_completed)+1}/{wf_total}] {coin} {tf} ...")
            candles = fetch_range(exchange, symbol, tf, wf_start_ts, now_ts)
            if candles is None:
                save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})
                sys.exit(0)

            df = build_df(candles)
            if df.empty:
                print(f"  No WF data for {coin} {tf}")
            else:
                df.to_csv(out_file)
                print(f"  Saved {len(df)} candles ({df.index[0]} → {df.index[-1]})")

            wf_completed.add(job_key)
            save_checkpoint({'completed': list(completed), 'wf_completed': list(wf_completed)})

    # ── Done ──
    print(f"\n{'='*80}")
    print("COMPLETE")
    print(f"{'='*80}")

    # Verify
    for tf in TIMEFRAMES:
        print(f"\n  {tf}:")
        for coin in COINS:
            main_f = f"{DATA_CACHE_DIR}/{coin}_USDT_{tf}_1year.csv"
            wf_f = f"{WF_DIR}/{coin}_USDT_{tf}_wf.csv"
            m_rows = len(pd.read_csv(main_f)) if os.path.exists(main_f) else 0
            w_rows = len(pd.read_csv(wf_f)) if os.path.exists(wf_f) else 0
            print(f"    {coin:5s}: main={m_rows:>7,}  wf={w_rows:>5,}")

    if os.path.exists(CHECKPOINT_FILE):
        os.remove(CHECKPOINT_FILE)
        print("\nCheckpoint cleaned up.")


if __name__ == "__main__":
    main()
