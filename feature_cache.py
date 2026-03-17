"""
Cache pre-computed feature matrices to disk.
Saves as CSV in data_cache/features/{COIN}_{TF}_features.csv.
"""
import os
import pandas as pd
from feature_engine import build_feature_matrix, load_cached_data, COINS
from tqdm import tqdm

CACHE_DIR = 'data_cache/features'


def ensure_cache_dir():
    os.makedirs(CACHE_DIR, exist_ok=True)


def cache_path(coin: str, timeframe: str = '15m') -> str:
    return os.path.join(CACHE_DIR, f'{coin}_USDT_{timeframe}_features.csv')


def build_and_cache(coin: str, timeframe: str = '15m', duration: str = '1year',
                    force: bool = False) -> pd.DataFrame:
    """Build feature matrix for a coin and save to cache."""
    path = cache_path(coin, timeframe)
    if not force and os.path.exists(path):
        return pd.read_csv(path, index_col=0, parse_dates=True)

    ensure_cache_dir()
    df = load_cached_data(coin, timeframe, duration)
    features = build_feature_matrix(df)
    features.to_csv(path)
    return features


def load_cached_features(coin: str, timeframe: str = '15m') -> pd.DataFrame:
    """Load cached feature matrix. Builds it if not cached."""
    path = cache_path(coin, timeframe)
    if os.path.exists(path):
        return pd.read_csv(path, index_col=0, parse_dates=True)
    return build_and_cache(coin, timeframe)


def build_all(timeframe: str = '15m', duration: str = '1year', force: bool = False):
    """Build and cache feature matrices for all coins."""
    ensure_cache_dir()
    for coin in tqdm(COINS, desc='Building features'):
        try:
            build_and_cache(coin, timeframe, duration, force=force)
            print(f'  {coin}: OK')
        except Exception as e:
            print(f'  {coin}: FAILED - {e}')


if __name__ == '__main__':
    build_all(force=True)
