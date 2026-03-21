# RUN157 — Bid-Ask Spread Widening Signal: Market Uncertainty as Mean-Reversion Entry

## Hypothesis

**Mechanism**: When the bid-ask spread on Binance orderbook widens significantly beyond its normal level, it signals market uncertainty and defensive positioning by market makers. Wide spreads often precede either sudden directional moves (volatility spike) or mean-reversion as market makers narrow spreads. COINCLAW has no spread data. Adding spread monitoring via the Binance orderbook API provides a signal of market stress that can be used as a regime filter or entry trigger.

**API to use** (already available from RUN152 orderbook setup):
- Binance: `GET https://api.binance.com/api/v3/orderbook?symbol={SYMBOL}USDT&limit=5`
  - Response: `{"bids": [[price, qty], ...], "asks": [[price, qty], ...]}`
  - Spread = `asks[0].price - bids[0].price`
  - Normalize: `spread / mid_price`
  - No API key required

**Spread calculation**: `(ask_price - bid_price) / mid_price`. Normal spread is ~0.01-0.02% for BTC. When spread exceeds 3-5x the 20-bar rolling average, it signals market maker stress.

**Why this is not a duplicate**: No prior RUN uses bid-ask spread. RUN152 uses orderbook imbalance but not the spread width itself. Spread is a market microstructure signal orthogonal to all price-action indicators.

## Proposed Config Changes (config.rs)

```rust
// ── RUN157: Bid-Ask Spread Widening Signal ──────────────────────────────
// Spread = (ask - bid) / mid_price
// Spread spike = current spread > rolling_avg * SPREAD_SPIKE_MULT
// Entry: spread spike + price has moved > threshold in one direction → mean-revert
// Alternative use: BLOCK all new entries when spread is elevated (market maker stress)
// Exit: spread normalization OR MAX_HOLD bars

pub const SPREAD_ENABLED: bool = true;
pub const SPREAD_SPIKE_MULT: f64 = 3.0;    // spread must be 3x the 20-bar rolling average
pub const SPREAD_ROLLING_WINDOW: usize = 20;  // bars for rolling average
pub const SPREAD_PRICE_MOVE: f64 = 0.005;  // 0.5% price move after spread spike to confirm
pub const SPREAD_SL: f64 = 0.005;         // 0.5% stop
pub const SPREAD_TP: f64 = 0.003;         // 0.3% take profit
pub const SPREAD_MAX_HOLD: u32 = 16;        // ~4 hours at 15m bars
```

Add to `coinclaw/src/fetcher.rs`:

```rust
/// Compute normalized spread from orderbook
/// Returns spread as fraction of mid-price (e.g., 0.0005 = 0.05%)
pub fn compute_spread(book: &OrderbookResponse) -> Option<f64> {
    let bid_price: f64 = book.bids.first()?.price.parse().ok()?;
    let ask_price: f64 = book.asks.first()?.price.parse().ok()?;
    let mid_price = (bid_price + ask_price) / 2.0;
    if mid_price == 0.0 { return None; }
    Some((ask_price - bid_price) / mid_price)
}
```

Add to `CoinState` in `state.rs`:

```rust
pub spread: f64,
pub spread_avg: f64,         // 20-bar rolling average
pub spread_history: Vec<f64>,
pub spread_spike_bars: u32,
pub spread_spike_direction: Option<Direction>,
```

Add entry logic in `engine.rs`:

```rust
/// Option A: Spread as entry trigger (fade the spread spike move)
fn check_spread_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::SPREAD_ENABLED { return None; }
    if cs.spread_spike_bars == 0 { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let price_move = match cs.spread_spike_direction {
        Some(Direction::Long) => {
            let drop = (cs.spread_entry_price - ind.p) / cs.spread_entry_price;
            if drop >= config::SPREAD_PRICE_MOVE { Direction::Long } else { return None; }
        }
        Some(Direction::Short) => {
            let rise = (ind.p - cs.spread_entry_price) / cs.spread_entry_price;
            if rise >= config::SPREAD_PRICE_MOVE { Direction::Short } else { return None; }
        }
        None => return None,
    };

    Some((price_move, "spread_widening"))
}

/// Option B: Spread as a blocking filter (block all entries when spread is elevated)
fn spread_block_entries(cs: &CoinState) -> bool {
    if !config::SPREAD_ENABLED { return false; }
    if cs.spread_avg <= 0.0 { return false; }
    cs.spread > cs.spread_avg * config::SPREAD_SPIKE_MULT
}
```

Note: Both Option A (spread as entry trigger) and Option B (spread as blocking filter) should be tested independently.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no spread signal)
- **Comparison**: spread entries tracked separately; spread filter vs no-filter comparison

**Metrics to measure**:
- Spread-triggered entry WR (hypothesis: >55%)
- Spread-filter blocking rate (% entries blocked vs outcomes on blocked)
- Spread spike frequency vs volatility events correlation

**Hypothesis**: Spread widening spikes (>3x normal) accompanied by >0.5% price move → mean-reversion within 4-16 bars with WR >55%. Or: blocking entries during spread spikes reduces loss rate on blocked trades.

---

## Validation Method

1. **Historical backtest** (run157_1_spread_backtest.py):
   - 18 coins, 1-year 15m data
   - Compute spread from orderbook data (available from Binance API historically via klines)
   - Identify all spread spikes with price confirmation
   - Record: spread magnitude, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run157_2_spread_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep SPREAD_SPIKE_MULT: 2.0 / 3.0 / 4.0 / 5.0

3. **Combined comparison** (run157_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + spread_trigger
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + spread_filter
   - Portfolio stats, blocking rate analysis

---

## Out-of-Sample Testing

- SPIKE_MULT sweep: 2.0 / 3.0 / 4.0 / 5.0
- PRICE_MOVE sweep: 0.003 / 0.005 / 0.008
- OOS: final 4 months held out from all parameter selection
