# RUN155 — Cross-Exchange Rate Divergence: Arbitrage Signal as Mean-Reversion Trigger

## Hypothesis

**Mechanism**: When BTC's price on Binance diverges significantly from BTC's price on Kraken (or another major exchange), the discrepancy is temporary — arbitrageurs close the gap, causing price to mean-revert. A divergence >0.1% between Binance and Kraken creates a high-probability re-entry opportunity in the direction of the correction. COINCLAW uses Binance-only price data. Adding cross-exchange price comparison provides a structurally different signal based on market microstructure.

**APIs to use**:
- Binance: `GET https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT` — `{"price": "50000.00"}`
- Kraken: `GET https://api.kraken.com/0/public/Ticker?pair=XBTUSD` — `{"result": {"XXBTZUSD": {"c": ["50001.0", "0.5"]}}}`
- CoinCap: `GET https://api.coincap.io/v2/assets/bitcoin` — `{"data": {"priceUsd": "50001.23"}}`
- Both: no API key required

**Divergence calculation**: `(binance_price / kraken_price - 1.0)`. Values > 0.1% = Binance premium (arbitrageurs sell Binance, buy Kraken → Binance price drops). Values < -0.1% = Binance discount (arbitrageurs buy Binance, sell Kraken → Binance price rises).

**Why this is not a duplicate**: No prior RUN uses cross-exchange price discrepancies. All signals are single-exchange. Cross-exchange arbitrage is a well-known microstructure phenomenon — the gap is mechanically closed by arbitrageurs, making this a self-fulfilling signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN155: Cross-Exchange Rate Divergence ────────────────────────────
// Divergence = (binance_price / other_exchange_price) - 1.0
// > 0.1% = Binance premium → expect correction DOWN → SHORT
// < -0.1% = Binance discount → expect correction UP → LONG
// Exit: divergence normalizes OR MAX_HOLD bars

pub const CER_ENABLED: bool = true;
pub const CER_THRESHOLD: f64 = 0.001;     // 0.1% divergence threshold
pub const CER_EXCHANGE: &'static str = "kraken";  // secondary exchange
pub const CER_SL: f64 = 0.003;           // 0.3% stop
pub const CER_TP: f64 = 0.002;          // 0.2% take profit
pub const CER_MAX_HOLD: u32 = 8;         // ~2 hours at 15m bars
pub const CER_FETCH_SECONDS: u64 = 30;   // fetch every 30s (fast-moving signal)
```

Add to `coinclaw/src/fetcher.rs`:

```rust
/// Fetch BTC price from multiple exchanges for cross-rate divergence detection.
pub async fn fetch_btc_prices() -> Option<(f64, f64)> {
    // Binance
    let binance_url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";
    let binance_resp: serde_json::Value = reqwest::get(binance_url).await.ok()?;
    let binance_price: f64 = binance_resp["price"]
        .as_str()?
        .parse()
        .ok()?;

    // Kraken
    let kraken_url = "https://api.kraken.com/0/public/Ticker?pair=XBTUSD";
    let kraken_resp: serde_json::Value = reqwest::get(kraken_url).await.ok()?;
    let kraken_price: f64 = kraken_resp["result"]["XXBTZUSD"]["c"][0]
        .as_str()?
        .parse()
        .ok()?;

    Some((binance_price, kraken_price))
}

/// Compute cross-exchange divergence.
/// Positive = Binance premium, Negative = Binance discount.
pub fn compute_divergence(binance_price: f64, other_price: f64) -> Option<f64> {
    if other_price <= 0.0 { return None; }
    Some(binance_price / other_price - 1.0)
}
```

Add to `SharedState` in `state.rs`:

```rust
pub binance_btc_price: f64,
pub kraken_btc_price: f64,
pub cer_divergence: f64,
pub cer_divergence_prev: f64,
pub cer_cross_bars: u32,
```

Add entry logic in engine.rs (applied to BTC only — signal used as broader market context for altcoins):

```rust
/// Fires on BTC when cross-exchange divergence exceeds threshold
/// Also sets global divergence signal used by altcoins
fn check_cer_entry(state: &mut SharedState) -> Option<(Direction, &'static str)> {
    if !config::CER_ENABLED { return None; }

    let div = state.cer_divergence;
    let div_prev = state.cer_divergence_prev;
    if div.is_nan() || div_prev.is_nan() { return None; }

    let thresh = config::CER_THRESHOLD;
    let cross_bars = state.cer_cross_bars;
    if cross_bars < 1 { return None; }

    // Binance premium (> threshold) → SHORT (arbitrage will close the gap = Binance drops)
    if div_prev <= thresh && div > thresh {
        state.cer_direction = Some(Direction::Short);
        return Some((Direction::Short, "cer_divergence"));
    }
    // Binance discount (< -threshold) → LONG (arbitrage will close the gap = Binance rises)
    if div_prev >= -thresh && div < -thresh {
        state.cer_direction = Some(Direction::Long);
        return Some((Direction::Long, "cer_divergence"));
    }
    None
}

/// Altcoin filter: only allow LONG entries when cer_direction is LONG or None.
/// Only allow SHORT entries when cer_direction is SHORT or None.
/// Block altcoin entries that oppose the CER signal.
fn cer_filter_altcoin(state: &SharedState, proposed_dir: Direction) -> bool {
    match (&state.cer_direction, proposed_dir) {
        (Some(Direction::Short), Direction::Long) => false,  // block long against cer short
        (Some(Direction::Long), Direction::Short) => false,  // block short against cer long
        _ => true,
    }
}
```

Note: Apply `cer_filter_altcoin` to regime and scalp entries for all non-BTC coins. BTC uses the CER signal directly for entries.

---

## Expected Outcome

**Validation**: Backtest on BTC and 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no cross-exchange signal)
- **Comparison**: CER trades tracked separately; altcoin filtered vs unfiltered comparison

**Metrics to measure**:
- CER divergence WR (hypothesis: >55%)
- PF on CER trades
- Altcoin entry quality improvement when filtered by CER
- Divergence magnitude vs outcome correlation

**Hypothesis**: Cross-exchange divergences >0.1% resolve within 2-8 bars with WR >55% because arbitrageurs mechanically close the gap.

---

## Validation Method

1. **Historical backtest** (run155_1_cer_backtest.py):
   - BTC + 18 coins, 1-year 15m data
   - Fetch historical price data from Binance and Kraken APIs
   - Identify all divergences > 0.1%
   - Record: divergence value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time; altcoin filtering effect

2. **Walk-forward** (run155_2_cer_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep CER_THRESHOLD: 0.0005 / 0.0010 / 0.0015 / 0.0020

3. **Combined comparison** (run155_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + cer_signal
   - Portfolio stats, BTC CER contribution, altcoin filtered vs unfiltered

---

## Out-of-Sample Testing

- Threshold sweep: 0.0005 / 0.0010 / 0.0015 / 0.0020
- OOS: final 4 months held out from all parameter selection
