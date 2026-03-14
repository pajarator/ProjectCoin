"""
Data fetcher for crypto historical data using CCXT.
"""
import ccxt
import pandas as pd
from datetime import datetime, timedelta
import time

# Initialize exchange (Binance as default - most liquid)
exchange = ccxt.binance({
    'enableRateLimit': True,
    'options': {'defaultType': 'spot'}
})

def fetch_ohlcv(symbol: str, timeframe: str = '1h', limit: int = 1000) -> pd.DataFrame:
    """
    Fetch OHLCV data for a symbol.
    
    Args:
        symbol: Trading pair (e.g., 'BTC/USDT')
        timeframe: Candle timeframe (1m, 5m, 15m, 1h, 4h, 1d)
        limit: Number of candles to fetch (max 1000 for free endpoints)
    
    Returns:
        DataFrame with columns: timestamp, open, high, low, close, volume
    """
    print(f"Fetching {symbol} {timeframe} data...")
    
    try:
        ohlcv = exchange.fetch_ohlcv(symbol, timeframe, limit=limit)
        
        df = pd.DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
        df.set_index('timestamp', inplace=True)
        
        print(f"✓ Got {len(df)} candles from {df.index[0]} to {df.index[-1]}")
        return df
        
    except Exception as e:
        print(f"Error fetching {symbol}: {e}")
        return pd.DataFrame()

def fetch_multiple_symbols(symbols: list, timeframe: str = '1h', limit: int = 1000) -> dict:
    """
    Fetch data for multiple symbols.
    """
    data = {}
    
    for symbol in symbols:
        df = fetch_ohlcv(symbol, timeframe, limit)
        if not df.empty:
            data[symbol] = df
        time.sleep(exchange.rateLimit / 1000)  # Rate limiting
    
    return data

def get_available_symbols() -> list:
    """
    Get list of available USDT pairs on Binance.
    """
    markets = exchange.load_markets()
    usdt_pairs = [symbol for symbol in markets.keys() if symbol.endswith('/USDT')]
    return sorted(usdt_pairs)

# Popular pairs to start with
DEFAULT_PAIRS = [
    'BTC/USDT', 'ETH/USDT', 'BNB/USDT', 'SOL/USDT', 'XRP/USDT',
    'ADA/USDT', 'DOGE/USDT', 'AVAX/USDT', 'DOT/USDT', 'MATIC/USDT',
    'LINK/USDT', 'UNI/USDT', 'ATOM/USDT', 'LTC/USDT', 'ETC/USDT'
]

if __name__ == "__main__":
    # Test fetch
    df = fetch_ohlcv('BTC/USDT', '1h', 500)
    print(df.tail())
