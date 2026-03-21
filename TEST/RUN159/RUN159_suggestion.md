# RUN159 — Taker Buy/Sell Ratio Signal: Aggressive Order Flow as Short-Term Direction Indicator

## Hypothesis

**Mechanism**: Taker buy orders (aggressive buyers crossing the spread) vs taker sell orders reveal real institutional/smart money direction. When the taker buy ratio spikes above 55% or below 45%, it signals directional conviction that typically exhausts within 4-8 bars. COINCLAW has no order flow data. Adding taker buy/sell ratio via Binance futures API provides a short-term directional signal orthogonal to all price-action indicators.

**API to use**:
- Binance: `GET https://api.binance.com/futures/data/takerlongshortRatio?symbol=BTCUSDT&period=5m`
  - Response: `{"buySellRatio": "0.52", "buyVol": "123.45", "sellVol": "115.32"}`
  - buySellRatio = buyVol / (buyVol + sellVol)
  - Update: 5min
  - No API key required

**Taker ratio calculation**: `buyVol / (buyVol + sellVol)`. Above 0.55 = aggressive buying (short-term top risk). Below 0.45 = aggressive selling (short-term bottom risk). Mean-revert when ratio returns toward 0.50.

**Why this is not a duplicate**: No prior RUN uses taker buy/sell data. This is a direct measure of aggressive order flow — distinct from funding rates (positioning cost), orderbook (passive liquidity), and price-action (outcome of order flow).

## Proposed Config Changes (config.rs)

```rust
// ── RUN159: Taker Buy/Sell Ratio Signal ────────────────────────────────
// Taker ratio = buyVol / (buyVol + sellVol)
// > 0.55 = aggressive buying → SHORT (exhaustion risk)
// < 0.45 = aggressive selling → LONG (oversold reversal)
// Entry: ratio crosses threshold AND price confirms
// Exit: ratio returns toward 0.50 OR MAX_HOLD bars

pub const TAKER_ENABLED: bool = true;
pub const TAKER_BUY_THRESH: f64 = 0.55;   // aggressive buying threshold
pub const TAKER_SELL_THRESH: f64 = 0.45;  // aggressive selling threshold
pub const TAKER_PERSIST_BARS: u32 = 2;      // ratio must persist for 2 bars
pub const TAKER_SL: f64 = 0.004;          // 0.4% stop
pub const TAKER_TP: f64 = 0.003;          // 0.3% take profit
pub const TAKER_MAX_HOLD: u32 = 12;         // ~3 hours at 15m bars
pub const TAKER_FETCH_MINUTES: u64 = 5;      // 5min data
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TakerLongShortRatio {
    #[serde(alias = "buySellRatio")]
    buy_sell_ratio: Option<String>,
    #[serde(alias = "buyVol")]
    buy_vol: Option<String>,
    #[serde(alias = "sellVol")]
    sell_vol: Option<String>,
    #[serde(alias = "symbol")]
    symbol: String,
}

impl TakerLongShortRatio {
    fn ratio(&self) -> Option<f64> {
        self.buy_sell_ratio.as_ref()?.parse().ok()
    }
}

/// Fetch taker buy/sell ratio from Binance futures public API.
/// No API key required.
pub async fn fetch_taker_ratio(symbol: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/futures/data/takerlongshortRatio?symbol={}USDT&period=5m",
        symbol
    );
    let resp = reqwest::get(&url).await.ok()?;
    let data: Vec<TakerLongShortRatio> = resp.json().await.ok()?;
    let latest = data.last()?;
    latest.ratio()
}
```

Add to `CoinState` in `state.rs`:

```rust
pub taker_ratio: f64,
pub taker_ratio_prev: f64,
pub taker_extreme_bars: u32,       // consecutive bars at extreme
pub taker_extreme_direction: Option<Direction>,
```

Add entry logic in `engine.rs`:

```rust
/// Fires when taker ratio crosses extreme threshold and persists
fn check_taker_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::TAKER_ENABLED { return None; }

    let ratio = cs.taker_ratio;
    let ratio_prev = cs.taker_ratio_prev;
    if ratio.is_nan() || ratio_prev.is_nan() { return None; }

    let bars = cs.taker_extreme_bars;
    if bars < config::TAKER_PERSIST_BARS { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    // Aggressive buying extreme → SHORT
    if ratio_prev < config::TAKER_BUY_THRESH && ratio >= config::TAKER_BUY_THRESH {
        return Some((Direction::Short, "taker_ratio"));
    }
    // Aggressive selling extreme → LONG
    if ratio_prev > config::TAKER_SELL_THRESH && ratio <= config::TAKER_SELL_THRESH {
        return Some((Direction::Long, "taker_ratio"));
    }
    None
}
```

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 5min/15m data.
- **Baseline**: COINCLAW v16 (no taker ratio signal)
- **Comparison**: taker ratio trades tracked separately

**Metrics to measure**:
- Taker ratio WR (hypothesis: >55%)
- PF on taker ratio trades
- Ratio magnitude vs outcome correlation
- Timing: how quickly does mean-reversion occur after ratio extreme?

**Hypothesis**: Taker buy ratio > 0.55 or < 0.45 persisting 2+ bars precedes mean-reversion within 3-12 bars with WR >55%.

---

## Validation Method

1. **Historical backtest** (run159_1_taker_backtest.py):
   - 18 coins, 1-year 5min data (aggregated to 15m bars)
   - Fetch or simulate Binance taker ratio data
   - Identify all ratio extremes
   - Record: ratio value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run159_2_taker_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep TAKER_BUY_THRESH: 0.52 / 0.55 / 0.58
   - Sweep TAKER_SELL_THRESH: 0.42 / 0.45 / 0.48

3. **Combined comparison** (run159_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + taker_ratio
   - Portfolio stats, correlation with regime trades

---

## Out-of-Sample Testing

- BUY_THRESH sweep: 0.52 / 0.55 / 0.58 / 0.60
- SELL_THRESH sweep: 0.40 / 0.45 / 0.48
- PERSIST_BARS sweep: 1 / 2 / 3
- OOS: final 4 months held out from all parameter selection
