# RUN151 — Funding Rate Anomaly Signal: Perpetual Futures Extreme as Mean-Reversion Catalyst

## Hypothesis

**Mechanism**: When perpetual futures funding rates spike to extreme levels (>0.05% or <-0.05% per 8h cycle), it signals an overcrowded trade. Excessive long funding means most traders are long — a reversal risk. COINCLAW currently has zero funding rate data. Adding a funding rate layer via Binance/Bybit's public API (no key needed) would detect these extremes and trigger counter-positions.

**API to use**:
- Binance: `GET https://api.binance.com/fapi/v1/premiumIndex?symbol={SYMBOL}USDT`
  - Response: `{"lastFundingRate": "0.00010000", "nextFundingTime": 1234567890, ...}`
  - Update: real-time (funding settles every 8h)
  - No API key required

**Why this is not a duplicate**: No prior RUN uses external API data. All existing COINCLAW signals are price-action only (z-score, RSI, Bollinger). Funding rate is a structural market signal unavailable from price data alone.

## Proposed Config Changes (config.rs)

```rust
// ── RUN151: Funding Rate Anomaly Signal ───────────────────────────────
// Fires when funding rate exceeds threshold → counter-position
// Entry: funding_rate > FR_LONG_THRESH → SHORT (overcrowded long)
// Entry: funding_rate < FR_SHORT_THRESH → LONG (overcrowded short)
// Exit: funding rate normalization OR MAX_HOLD bars

pub const FR_ENABLED: bool = true;
pub const FR_LONG_THRESH: f64 = 0.0005;     // 0.05% per 8h = extreme long funding
pub const FR_SHORT_THRESH: f64 = -0.0005;    // -0.05% per 8h = extreme short funding
pub const FR_SL: f64 = 0.004;              // 0.4% stop
pub const FR_TP: f64 = 0.003;              // 0.3% take profit
pub const FR_MAX_HOLD: u32 = 16;            // ~4 hours at 15m bars
pub const FR_MIN_AGE_BARS: u32 = 2;         // funding rate must persist for 2 bars
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BinancePremiumIndex {
    #[serde(alias = "lastFundingRate")]
    last_funding_rate: Option<String>,
    #[serde(alias = "symbol")]
    symbol: String,
}

impl BinancePremiumIndex {
    fn funding_rate(&self) -> Option<f64> {
        self.last_funding_rate.as_ref()?
            .parse::<f64>()
            .ok()
    }
}

/// Fetch funding rate for a symbol from Binance futures public API.
/// No API key required.
pub async fn fetch_funding_rate(symbol: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/fapi/v1/premiumIndex?symbol={}USDT",
        symbol
    );
    let resp = reqwest::get(&url).await.ok()?;
    let data: BinancePremiumIndex = resp.json().await.ok()?;
    data.funding_rate()
}
```

Add to `CoinState` in `state.rs`:

```rust
pub funding_rate: f64,
pub funding_rate_prev: f64,
pub funding_rate_bars: u32,  // consecutive bars exceeding threshold
```

Add entry logic in `engine.rs`:

```rust
/// Fires when funding rate exceeds threshold for MIN_AGE_BARS consecutive bars
fn check_funding_rate_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::FR_ENABLED { return None; }

    let fr = cs.funding_rate;
    if fr.is_nan() { return None; }

    let fr_bars = cs.funding_rate_bars;
    if fr_bars < config::FR_MIN_AGE_BARS { return None; }

    if fr > config::FR_LONG_THRESH {
        return Some((Direction::Short, "funding_rate"));
    }
    if fr < config::FR_SHORT_THRESH {
        return Some((Direction::Long, "funding_rate"));
    }
    None
}
```

Integration: Call `fetch_funding_rate` for each coin in the fetch cycle (every 15s-1m). Update `funding_rate_bars` counter when |fr| > threshold.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no funding rate signal)
- **Comparison**: funding rate trades tracked separately

**Metrics to measure**:
- Funding rate anomaly WR (hypothesis: >55%)
- PF on funding rate trades
- How often does funding rate extreme lead to reversal?
- Correlation with regime trades (should be orthogonal)

**Hypothesis**: When funding rate exceeds ±0.05%, the probability of mean-reversion within 4-16 bars is >55% because overcrowded derivative positions are mechanically unwound.

---

## Validation Method

1. **Historical backtest** (run151_1_fr_backtest.py):
   - 18 coins, 1-year 15m data
   - Fetch archived Binance funding rate data from futures historical API
   - Identify all funding rate extremes
   - Record: funding rate value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run151_2_fr_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep FR_LONG_THRESH: 0.0003 / 0.0005 / 0.0010
   - Sweep FR_SHORT_THRESH: symmetric negatives

3. **Combined comparison** (run151_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + funding_rate
   - Portfolio stats, correlation with regime trades

---

## Out-of-Sample Testing

- Threshold sweep: 0.0003 / 0.0005 / 0.0010 / 0.0020
- MIN_AGE sweep: 1 / 2 / 4 bars
- OOS: final 4 months held out from all parameter selection
