# External API Research Cache
# Cached on 2026-03-20 — do not re-fetch unless stale

## CoinQuanta/awesome-crypto-api findings:

### Free, Fast, Simple HTTP/JSON APIs:

| API | Data Types | Update Freq | Notes |
|-----|-----------|-------------|-------|
| **CryptoCompare** | Prices, orderbooks, on-chain, sentiment | Sub-second | Most comprehensive, free tier available |
| **CoinGecko** | Prices, market data, orderbooks, on-chain | Near real-time | Good free tier |
| **CoinCap** | Prices, market activity | Real-time streaming | Simple, lightweight |
| **Nomics** | Prices, market data | Real-time | Clean API |
| **Santiment** | Sentiment, on-chain (1200+ assets) | Daily/trending | Not real-time enough |

### Fast Crypto-Specific Data (priority for COINCLAW):

1. **Binance public API** (no auth needed for price/orderbook):
   - `https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT` — ticker price
   - `https://api.binance.com/api/v3/orderbook?symbol=BTCUSDT&limit=5` — orderbook depth
   - Update: real-time
   - **No API key required for public endpoints**

2. **Bybit public API** (no auth for market data):
   - `https://api.bybit.com/v2/public/tickers` — all tickers with funding rate
   - Update: real-time
   - **Funding rate data available without auth**

3. **CoinGecko** (free tier):
   - `https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd` — price
   - `https://api.coingecko.com/api/v3/coins/bitcoin/tickers` — orderbook tickers
   - Update: ~30s for free tier

### Priority APIs for COINCLAW enhancement:

**HIGHEST PRIORITY:**
- **Binance funding rate**: `https://api.binance.com/fapi/v1/premiumIndex?symbol=BTCUSDT`
  - Fields: `lastFundingRate`, `nextFundingTime`
  - Update: 8h funding cycle (but premium index updates real-time)
  - Use: detect extreme funding rate anomalies

- **Bybit funding rate**: `https://api.bybit.com/v2/public/tickers`
  - Fields: `funding_rate`, `mark_price`
  - Update: real-time
  - Use: same as Binance

**MEDIUM PRIORITY:**
- **CryptoCompare orderbook**: `https://min-api.cryptocompare.com/data/orderbook?fsym=BTC&tsym=USDT&limit=10`
  - Fields: `bids`, `asks`
  - Update: real-time
  - Use: mid-price stability signal

- **CoinCap orderbook**: `https://api.coincap.io/v2/orderbook?baseSymbol=BTC&quoteSymbol=USDT`
  - Fields: `bids`, `asks`
  - Update: real-time

**LOWER PRIORITY (not real-time enough):**
- Santiment (daily data, too slow for 15s-1m loop)
- On-chain data (Covalent, Bitquery) — too complex for simple integration

## public-apis findings:

Most blockchain APIs require API keys and are on-chain focused (not suitable for fast market data).

**Walltime** (no auth): `https://walltime.info/api.html` — market data, but not crypto-specific enough.

**Chainlink** — oracle data, not market data.

**Conclusion**: Use Binance/Bybit public APIs for funding rate + orderbook data without any API key.
