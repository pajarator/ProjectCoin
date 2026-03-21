"""
Lightweight sentiment data fetcher using free APIs.
  - Fear & Greed Index (alternative.me)
  - Google Trends (via pytrends, optional)
  - CoinGecko social stats
"""
import pandas as pd
import json
from urllib.request import urlopen
from urllib.error import URLError


def fetch_fear_greed_index(days: int = 365) -> pd.DataFrame:
    """
    Fetch Crypto Fear & Greed Index from alternative.me.
    Free API, no key needed.

    Returns:
        DataFrame with columns: value, classification
        Index: date
    """
    url = f'https://api.alternative.me/fng/?limit={days}&format=json'
    try:
        with urlopen(url, timeout=30) as response:
            data = json.loads(response.read().decode())

        records = []
        for item in data.get('data', []):
            records.append({
                'date': pd.to_datetime(int(item['timestamp']), unit='s'),
                'fg_value': int(item['value']),
                'fg_class': item['value_classification'],
            })

        df = pd.DataFrame(records)
        df.set_index('date', inplace=True)
        df.sort_index(inplace=True)
        return df
    except (URLError, Exception) as e:
        print(f'Fear & Greed fetch error: {e}')
        return pd.DataFrame()


def fetch_coingecko_market(coin_id: str = 'bitcoin', days: int = 365) -> pd.DataFrame:
    """
    Fetch market data from CoinGecko (free API).
    Includes price, market cap, and total volume.

    Args:
        coin_id: CoinGecko coin ID (e.g., 'bitcoin', 'ethereum')
        days: Number of days of history
    """
    url = f'https://api.coingecko.com/api/v3/coins/{coin_id}/market_chart?vs_currency=usd&days={days}'
    try:
        with urlopen(url, timeout=30) as response:
            data = json.loads(response.read().decode())

        prices = pd.DataFrame(data['prices'], columns=['timestamp', 'price'])
        prices['timestamp'] = pd.to_datetime(prices['timestamp'], unit='ms')
        prices.set_index('timestamp', inplace=True)

        volumes = pd.DataFrame(data['total_volumes'], columns=['timestamp', 'total_volume'])
        volumes['timestamp'] = pd.to_datetime(volumes['timestamp'], unit='ms')
        volumes.set_index('timestamp', inplace=True)

        return prices.join(volumes['total_volume'])
    except (URLError, Exception) as e:
        print(f'CoinGecko fetch error for {coin_id}: {e}')
        return pd.DataFrame()


# Map our coin symbols to CoinGecko IDs
COINGECKO_IDS = {
    'BTC': 'bitcoin', 'ETH': 'ethereum', 'BNB': 'binancecoin',
    'SOL': 'solana', 'XRP': 'ripple', 'ADA': 'cardano',
    'DOGE': 'dogecoin', 'AVAX': 'avalanche-2', 'DOT': 'polkadot',
    'LINK': 'chainlink', 'UNI': 'uniswap', 'ATOM': 'cosmos',
    'LTC': 'litecoin', 'NEAR': 'near', 'ALGO': 'algorand',
    'XLM': 'stellar', 'TRX': 'tron', 'DASH': 'dash',
    'SHIB': 'shiba-inu',
}
