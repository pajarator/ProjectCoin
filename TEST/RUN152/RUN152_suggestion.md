# RUN152 — Orderbook Imbalance Signal: Mid-Price Instability as Entry Catalyst

## Hypothesis

**Mechanism**: The orderbook's bid/ask density ratio reveals institutional order flow imbalance. When bids are much thinner than asks at the top levels, price is vulnerable to a rapid drop (supply overhang). When asks are thinner than bids, price is vulnerable to a rapid rise (demand overhang). COINCLAW has no orderbook data. Adding a lightweight orderbook snapshot via Binance public API (no key, ~100ms latency) provides a leading signal of short-term directional pressure.

**API to use**:
- Binance: `GET https://api.binance.com/api/v3/orderbook?symbol={SYMBOL}USDT&limit=5`
  - Response: `{"bids": [[price, qty], ...], "asks": [[price, qty], ...]}`
  - Update: real-time
  - No API key required
  - Latency: ~100ms

**Imbalance calculation**: `(bid_vol - ask_vol) / (bid_vol + ask_vol)` over top 5 levels. Values near 0 = balanced. Values near ±1 = extreme imbalance.

**Why this is not a duplicate**: No prior RUN uses orderbook data. All signals are OHLCV-based. Orderbook imbalance is a leading indicator (executions reveal order flow before price moves).

## Proposed Config Changes (config.rs)

```rust
// ── RUN152: Orderbook Imbalance Signal ────────────────────────────────
// Imbalance = (bid_vol - ask_vol) / (bid_vol + ask_vol) over top 5 levels
// LONG: imbalance < -OB_THRESHOLD (asks much thicker than bids = demand overhang)
// SHORT: imbalance > OB_THRESHOLD (bids much thicker than asks = supply overhang)
// Entry: imbalance crosses threshold AND price confirms direction
// Exit: imbalance normalization OR MAX_HOLD bars

pub const OB_ENABLED: bool = true;
pub const OB_THRESHOLD: f64 = 0.60;          // imbalance magnitude threshold (0-1)
pub const OB_LOOKUP: usize = 5;              // top N levels
pub const OB_SL: f64 = 0.004;              // 0.4% stop
pub const OB_TP: f64 = 0.003;              // 0.3% take profit
pub const OB_MAX_HOLD: u32 = 8;              // ~2 hours at 15m bars
pub const OB_FETCH_MS: u64 = 500;           // fetch every 500ms (within 15s loop)
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OrderbookEntry {
    #[serde(alias = "0")]
    pub price: String,
    #[serde(alias = "1")]
    pub qty: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderbookResponse {
    #[serde(alias = "lastUpdateId")]
    pub last_update_id: u64,
    #[serde(alias = "bids")]
    pub bids: Vec<OrderbookEntry>,
    #[serde(alias = "asks")]
    pub asks: Vec<OrderbookEntry>,
}

/// Compute orderbook imbalance: (bid_vol - ask_vol) / (bid_vol + ask_vol)
/// Returns value in [-1.0, 1.0]: negative = sell pressure, positive = buy pressure
pub fn compute_imbalance(book: &OrderbookResponse) -> f64 {
    let mut bid_vol = 0.0;
    let mut ask_vol = 0.0;
    for (i, entry) in book.bids.iter().enumerate() {
        if i >= 5 { break; }
        if let Ok(qty) = entry.qty.parse::<f64>() {
            bid_vol += qty;
        }
    }
    for (i, entry) in book.asks.iter().enumerate() {
        if i >= 5 { break; }
        if let Ok(qty) = entry.qty.parse::<f64>() {
            ask_vol += qty;
        }
    }
    let total = bid_vol + ask_vol;
    if total == 0.0 { return 0.0; }
    (bid_vol - ask_vol) / total
}

/// Fetch orderbook for a symbol from Binance public API.
/// No API key required.
pub async fn fetch_orderbook(symbol: &str) -> Option<(f64, OrderbookResponse)> {
    let url = format!(
        "https://api.binance.com/api/v3/orderbook?symbol={}USDT&limit=5",
        symbol
    );
    let resp = reqwest::get(&url).await.ok()?;
    let book: OrderbookResponse = resp.json().await.ok()?;
    let imbalance = compute_imbalance(&book);
    Some((imbalance, book))
}
```

Add to `CoinState` in `state.rs`:

```rust
pub ob_imbalance: f64,
pub ob_imbalance_prev: f64,
pub ob_cross_bars: u32,  // bars since imbalance crossed threshold
```

Add entry logic in `engine.rs`:

```rust
/// Fires when orderbook imbalance crosses threshold and persists
fn check_ob_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::OB_ENABLED { return None; }

    let imb = cs.ob_imbalance;
    let imb_prev = cs.ob_imbalance_prev;
    if imb.is_nan() || imb_prev.is_nan() { return None; }

    let thresh = config::OB_THRESHOLD;
    let cross_bars = cs.ob_cross_bars;
    if cross_bars < 1 { return None; }

    // Imbalance crossed above threshold → SHORT (bids overwhelming = supply overhang)
    if imb_prev <= thresh && imb > thresh {
        return Some((Direction::Short, "ob_imbalance"));
    }
    // Imbalance crossed below -threshold → LONG (asks overwhelming = demand overhang)
    if imb_prev >= -thresh && imb < -thresh {
        return Some((Direction::Long, "ob_imbalance"));
    }
    None
}
```

Note: Integrate into fetch cycle — call `fetch_orderbook` every `OB_FETCH_MS` (max once per loop cycle). Update `ob_cross_bars` when imbalance exceeds threshold.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data (requires historical orderbook data or simulated imbalance from trade tape).
- **Baseline**: COINCLAW v16 (no orderbook signal)
- **Comparison**: orderbook trades tracked separately

**Metrics to measure**:
- Orderbook imbalance WR (hypothesis: >55%)
- PF on orderbook trades
- Imbalance magnitude vs outcome correlation
- False signal rate (imbalance crosses but price doesn't follow)

**Hypothesis**: Orderbook imbalance > ±0.60 triggers mean-reversion with WR >55% because thin orders at the top of the book are vulnerable to rapid filling, causing price to move toward the thicker side.

---

## Validation Method

1. **Historical backtest** (run152_1_ob_backtest.py):
   - 18 coins, 1-year 15m data
   - Reconstruct orderbook imbalance from tick data or use synthetic simulation
   - Identify all imbalance crosses
   - Record: imbalance value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run152_2_ob_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep OB_THRESHOLD: 0.50 / 0.60 / 0.70 / 0.80

3. **Combined comparison** (run152_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + ob_imbalance
   - Portfolio stats, correlation with regime trades

---

## Out-of-Sample Testing

- Threshold sweep: 0.50 / 0.60 / 0.70 / 0.80
- LOOKUP sweep: 3 / 5 / 10 levels
- CROSS_BARS sweep: 1 / 2 / 3 bars
- OOS: final 4 months held out from all parameter selection
