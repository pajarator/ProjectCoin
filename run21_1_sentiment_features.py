"""
RUN21.1 — Sentiment Features Analysis

Merge Fear & Greed (daily, forward-fill) with 15m data.
Test:
  - Does performance differ at extreme fear (<25) vs extreme greed (>75)?
  - Test as long-only filter when F&G < 40 (buy the fear)

Output: run21_1_results.json
"""
import json
import os
import signal
import numpy as np
import pandas as pd
from tqdm import tqdm

from feature_engine import load_cached_data, COINS
from indicators import add_all_indicators
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
from backtester import Backtester
from data_fetcher_sentiment import fetch_fear_greed_index

RESULTS_FILE = 'run21_1_results.json'
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}
FG_CACHE = 'data_cache/fear_greed.csv'

shutdown_requested = False
def signal_handler(sig, frame):
    global shutdown_requested
    shutdown_requested = True
signal.signal(signal.SIGINT, signal_handler)


def get_fear_greed() -> pd.DataFrame:
    """Get Fear & Greed data, cached to disk."""
    if os.path.exists(FG_CACHE):
        fg = pd.read_csv(FG_CACHE, index_col=0, parse_dates=True)
        if len(fg) > 300:
            return fg

    fg = fetch_fear_greed_index(days=365)
    if not fg.empty:
        os.makedirs('data_cache', exist_ok=True)
        fg.to_csv(FG_CACHE)
    return fg


def merge_fg_with_ohlcv(df: pd.DataFrame, fg: pd.DataFrame) -> pd.DataFrame:
    """Merge Fear & Greed (daily) with 15m OHLCV data via forward-fill."""
    result = df.copy()
    # Resample F&G to daily, then forward-fill to 15m
    fg_daily = fg['fg_value'].resample('D').last()
    fg_15m = fg_daily.resample('15min').ffill()
    result['fg_value'] = fg_15m.reindex(result.index, method='ffill')
    return result


def analyze_coin(coin: str, fg: pd.DataFrame) -> dict:
    """Analyze sentiment-filtered performance for one coin."""
    df = load_cached_data(coin)
    df = add_all_indicators(df)
    df = merge_fg_with_ohlcv(df, fg)

    # Drop rows without F&G
    df = df.dropna(subset=['fg_value'])
    if len(df) < 500:
        return {'coin': coin, 'error': 'insufficient overlap with F&G data'}

    # Define regimes
    extreme_fear = df['fg_value'] < 25
    fear = df['fg_value'] < 40
    neutral = (df['fg_value'] >= 40) & (df['fg_value'] <= 60)
    greed = df['fg_value'] > 60
    extreme_greed = df['fg_value'] > 75

    results = {'coin': coin, 'strategies': {}}

    # Test top strategies in each sentiment regime
    for strat_name, strat_func in list(ALL_STRATEGIES.items())[:10]:  # top 10 to save time
        try:
            entry, exit_sig = strat_func(df)

            # Full period
            bt_full = Backtester(df, fee=0.001, slippage=0.0005)
            full_result = bt_full.run(entry, exit_sig, stop_loss=0.003)

            if full_result.total_trades < 10:
                continue

            strat_results = {
                'full': {
                    'trades': full_result.total_trades,
                    'win_rate': full_result.win_rate,
                    'profit_factor': full_result.profit_factor,
                }
            }

            # Fear-filtered (only enter during fear)
            fear_entry = entry & fear
            if fear_entry.sum() >= 3:
                bt_fear = Backtester(df, fee=0.001, slippage=0.0005)
                fear_result = bt_fear.run(fear_entry, exit_sig, stop_loss=0.003)
                strat_results['fear_only'] = {
                    'trades': fear_result.total_trades,
                    'win_rate': fear_result.win_rate,
                    'profit_factor': fear_result.profit_factor,
                }

            # Greed-filtered
            greed_entry = entry & greed
            if greed_entry.sum() >= 3:
                bt_greed = Backtester(df, fee=0.001, slippage=0.0005)
                greed_result = bt_greed.run(greed_entry, exit_sig, stop_loss=0.003)
                strat_results['greed_only'] = {
                    'trades': greed_result.total_trades,
                    'win_rate': greed_result.win_rate,
                    'profit_factor': greed_result.profit_factor,
                }

            # Fear filter helps?
            if 'fear_only' in strat_results:
                strat_results['fear_helps'] = (
                    strat_results['fear_only']['win_rate'] > strat_results['full']['win_rate']
                )

            results['strategies'][strat_name] = strat_results
        except Exception:
            continue

    # Count where fear filter helps
    fear_helps_count = sum(1 for s in results['strategies'].values() if s.get('fear_helps', False))
    results['fear_filter_helps'] = fear_helps_count
    results['strategies_tested'] = len(results['strategies'])

    return results


def main():
    print('=' * 60)
    print('RUN21.1 — Sentiment Features Analysis')
    print('=' * 60)

    fg = get_fear_greed()
    if fg.empty:
        print('ERROR: Could not fetch Fear & Greed data')
        return

    print(f'Fear & Greed data: {len(fg)} days')
    print(f'Range: {fg.index[0]} to {fg.index[-1]}')
    print(f'Current: {fg["fg_value"].iloc[-1]} ({fg["fg_class"].iloc[-1]})\n')

    all_results = {}

    for coin in tqdm(COINS, desc='Sentiment analysis'):
        if shutdown_requested:
            break

        try:
            result = analyze_coin(coin, fg)
            all_results[coin] = result
            if 'fear_filter_helps' in result:
                print(f'  {coin}: fear helps {result["fear_filter_helps"]}/{result["strategies_tested"]}')
        except Exception as e:
            print(f'  {coin}: FAILED - {e}')

    # Summary
    valid = {k: v for k, v in all_results.items() if 'fear_filter_helps' in v}
    total_fear_helps = sum(v['fear_filter_helps'] for v in valid.values())
    total_tested = sum(v['strategies_tested'] for v in valid.values())

    final = {
        'per_coin': all_results,
        'summary': {
            'coins_analyzed': len(valid),
            'total_fear_filter_helps': total_fear_helps,
            'total_strategy_tests': total_tested,
            'fear_help_pct': (total_fear_helps / total_tested * 100) if total_tested > 0 else 0,
        }
    }

    with open(RESULTS_FILE, 'w') as f:
        json.dump(final, f, indent=2, default=str)

    print(f'\n{"=" * 60}')
    print(f'RESULTS: {RESULTS_FILE}')
    print(f'Fear filter helps: {total_fear_helps}/{total_tested} '
          f'({final["summary"]["fear_help_pct"]:.0f}%)')


if __name__ == '__main__':
    main()
