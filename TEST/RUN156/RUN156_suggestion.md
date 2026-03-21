# RUN156 — Whale Exchange Flow Signal: On-Chain Accumulation/Distribution Leading Price

## Hypothesis

**Mechanism**: When large BTC holdings flow into exchanges en masse, it's a bearish signal — whales are preparing to sell. When BTC flows out of exchanges into cold storage, it's bullish — accumulation. COINCLAW has zero on-chain data. Adding exchange flow data via Glassnode's free tier API (or CryptoQuant's public endpoint) provides a structural directional signal that precedes price moves by 2-12 hours.

**APIs to use**:
- CryptoQuant (free, no key for some endpoints):
  - `GET https://api.cryptquant.com/v1/indicators/ETH/exchange-flows` — exchange inflow/outflow
  - Or use Glassnode: `GET https://api.glassnode.com/v1/metrics/addresses/count` (requires key)
- CoinGecko (free, no key): `GET https://api.coingecko.com/api/v3/coins/bitcoin` — has market cap data but not flow data
- Alternative: Use Binance's Bitcoin reserves proxy via `https://api.binance.com/api/v3/exchangeInfo` — not direct flow but BTC持仓量 changes correlate with exchange flows

**Flow calculation**: Compare current exchange flow (inflow - outflow) to 7-day rolling average. Sudden increase in net inflows = bearish (selling pressure). Sudden outflows = bullish (accumulation).

**Why this is not a duplicate**: No prior RUN uses on-chain or exchange flow data. This is a fundamentally different data class — direct blockchain behavior rather than price-action inference.

## Proposed Config Changes (config.rs)

```rust
// ── RUN156: Whale Exchange Flow Signal ─────────────────────────────────
// Flow = net exchange inflow - outflow (positive = more flowing in = sell pressure)
// Flow spike = current flow > rolling_avg * FLOW_SPIKE_MULT
// Entry: flow spike + price moved in same direction as flow → fade the move
// Exit: flow normalization OR MAX_HOLD bars

pub const FLOW_ENABLED: bool = true;
pub const FLOW_SPIKE_MULT: f64 = 2.0;     // flow must be 2x the 7-day rolling average
pub const FLOW_WINDOW_DAYS: usize = 7;      // rolling window for flow avg
pub const FLOW_PRICE_CONFIRM: f64 = 0.005; // 0.5% price move required to confirm direction
pub const FLOW_SL: f64 = 0.005;           // 0.5% stop (whale moves are large)
pub const FLOW_TP: f64 = 0.004;           // 0.4% take profit
pub const FLOW_MAX_HOLD: u32 = 48;           // ~12 hours at 15m bars (whale signals are slow)
pub const FLOW_FETCH_MINUTES: u64 = 60;      // on-chain data updates hourly/daily
```

Note: Exchange flow data typically has hourly granularity, making it suitable for the 15m timeframe as a contextual filter rather than a per-bar signal.

## Proposed Implementation

Since free on-chain APIs are limited, implement as a **daily contextual filter**:

```rust
// In state.rs — updated once per day
pub daily_exchange_flow: f64,      // net flow direction: positive=inflow, negative=outflow
pub daily_flow_bullish: bool,      // true if flow is outflow (accumulation)
pub daily_flow_bearish: bool,      // true if flow is inflow (distribution)
```

```rust
// In fetcher.rs — fetch once per day
pub async fn fetch_exchange_flow(coin: &str) -> Option<f64> {
    // CryptoQuant free endpoint (no key required for some endpoints)
    let url = format!(
        "https://api.cryptquant.com/v1/indicators/{}/exchange-flows",
        coin.to_lowercase()
    );
    let resp = reqwest::get(&url).await.ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;
    // Extract net flow from response
    let flow = data["result"]["net_flow"]?.as_f64()?;
    Some(flow)
}
```

```rust
// In engine.rs — filter regime entries based on daily flow signal
fn flow_filter_regime_entry(cs: &CoinState, proposed_dir: Direction) -> bool {
    if !config::FLOW_ENABLED { return true; }
    if cs.daily_flow_bearish && proposed_dir == Direction::Long {
        return false;  // block long during exchange inflow (distribution)
    }
    if cs.daily_flow_bullish && proposed_dir == Direction::Short {
        return false;  // block short during exchange outflow (accumulation)
    }
    true
}
```

---

## Expected Outcome

**Validation**: Backtest on BTC and 18 coins, 1-year daily+15m data.
- **Baseline**: COINCLAW v16 (no on-chain signal)
- **Comparison**: filtered vs unfiltered entries

**Metrics to measure**:
- Flow-filter improvement in WR (hypothesis: +3-5pp)
- Flow signal accuracy as directional predictor
- % of regime entries blocked by flow filter

**Hypothesis**: When exchange outflows spike (accumulation), blocking SHORT entries improves WR by >3pp because the directional bias is bullish. When exchange inflows spike (distribution), blocking LONG entries improves WR by >3pp.

---

## Validation Method

1. **Historical backtest** (run156_1_flow_backtest.py):
   - BTC + 18 coins, 1-year daily + 15m data
   - Fetch or reconstruct exchange flow data
   - Compare: regime entries with flow filter vs without
   - Record: filter hit rate, blocked trade outcomes, net improvement
   - Output: per-coin WR change, PF change

2. **Walk-forward** (run156_2_flow_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep FLOW_SPIKE_MULT: 1.5 / 2.0 / 3.0

3. **Combined comparison** (run156_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + flow_filter
   - Portfolio stats, blocked trade analysis

---

## Out-of-Sample Testing

- SPIKE_MULT sweep: 1.5 / 2.0 / 3.0 / 4.0
- OOS: final 4 months held out from all parameter selection
