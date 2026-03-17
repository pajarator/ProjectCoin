"""
Extended data fetcher for Binance Futures derivatives data.
Fetches funding rates, open interest, and liquidation events via CCXT.
"""
import ccxt
import pandas as pd
import time

# Binance Futures exchange
futures_exchange = ccxt.binance({
    'enableRateLimit': True,
    'options': {'defaultType': 'future'}
})


def fetch_funding_rate(symbol: str, since: int = None, limit: int = 1000) -> pd.DataFrame:
    """
    Fetch historical funding rate data.

    Args:
        symbol: Trading pair (e.g., 'BTC/USDT')
        since: Start timestamp in ms (None = most recent)
        limit: Max records per request

    Returns:
        DataFrame with columns: timestamp, funding_rate, funding_timestamp
    """
    try:
        rates = futures_exchange.fetch_funding_rate_history(symbol, since=since, limit=limit)
        if not rates:
            return pd.DataFrame()

        df = pd.DataFrame([{
            'timestamp': pd.to_datetime(r['timestamp'], unit='ms'),
            'funding_rate': r['fundingRate'],
        } for r in rates])
        df.set_index('timestamp', inplace=True)
        return df
    except Exception as e:
        print(f'Error fetching funding rate for {symbol}: {e}')
        return pd.DataFrame()


def fetch_funding_rate_history(symbol: str, days: int = 365) -> pd.DataFrame:
    """Fetch extended funding rate history by paginating."""
    all_data = []
    since = int((pd.Timestamp.now() - pd.Timedelta(days=days)).timestamp() * 1000)

    while True:
        df = fetch_funding_rate(symbol, since=since, limit=1000)
        if df.empty:
            break
        all_data.append(df)
        # Move since to after last record
        since = int(df.index[-1].timestamp() * 1000) + 1
        if len(df) < 1000:
            break
        time.sleep(futures_exchange.rateLimit / 1000)

    if all_data:
        return pd.concat(all_data).sort_index()
    return pd.DataFrame()


def fetch_open_interest(symbol: str, timeframe: str = '1d',
                        since: int = None, limit: int = 500) -> pd.DataFrame:
    """
    Fetch open interest history.

    Note: Binance provides OI via their API. CCXT support varies.
    Falls back to fetching via REST if needed.
    """
    try:
        # Try CCXT method first
        if hasattr(futures_exchange, 'fetch_open_interest_history'):
            oi_data = futures_exchange.fetch_open_interest_history(
                symbol, timeframe=timeframe, since=since, limit=limit
            )
            if oi_data:
                df = pd.DataFrame([{
                    'timestamp': pd.to_datetime(r['timestamp'], unit='ms'),
                    'open_interest': r['openInterestAmount'],
                    'open_interest_value': r.get('openInterestValue', None),
                } for r in oi_data])
                df.set_index('timestamp', inplace=True)
                return df
    except Exception as e:
        print(f'OI fetch error for {symbol}: {e}')

    return pd.DataFrame()


def fetch_open_interest_history(symbol: str, timeframe: str = '1d',
                                 days: int = 365) -> pd.DataFrame:
    """Fetch extended OI history by paginating."""
    all_data = []
    since = int((pd.Timestamp.now() - pd.Timedelta(days=days)).timestamp() * 1000)

    while True:
        df = fetch_open_interest(symbol, timeframe=timeframe, since=since, limit=500)
        if df.empty:
            break
        all_data.append(df)
        since = int(df.index[-1].timestamp() * 1000) + 1
        if len(df) < 500:
            break
        time.sleep(futures_exchange.rateLimit / 1000)

    if all_data:
        return pd.concat(all_data).sort_index()
    return pd.DataFrame()
