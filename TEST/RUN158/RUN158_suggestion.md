# RUN158 — Market-Wide Positioning Index: Exchange Long/Short Ratio as Crowding Signal

## Hypothesis

**Mechanism**: When the majority of traders on Binance (or across major exchanges) are long, the market is crowded on the long side — a reversal risk. Conversely for shorts. COINCLAW uses no positioning data. By combining long/short ratios from Binance and Bybit public APIs, a "Positioning Index" can be computed. Extreme positioning (ratios >2:1 or <1:2) precedes reversals.

**APIs to use**:
- Binance: `GET https://api.binance.com/futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=5m`
  - Response: `{"longAccount": "0.52", "shortAccount": "0.48"}`
  - Update: 5min
  - No API key required
- Bybit: `GET https://api.bybit.com/v2/public/tickers` has `longShort` ratio field for some symbols

**Positioning Index calculation**: `(long_ratio - short_ratio)` normalized to [-1, +1]. Values near +0.8 = extreme long crowding; near -0.8 = extreme short crowding.

**Why this is not a duplicate**: No prior RUN uses positioning or long/short ratio data. All signals are price-action derivatives of positioning. Direct positioning data is a fundamentally different signal class.

## Proposed Config Changes (config.rs)

```rust
// ── RUN158: Market-Wide Positioning Index ────────────────────────────────
// Positioning Index = (long_ratio - short_ratio) normalized to [-1, +1]
// > 0.6 = extreme long crowding → SHORT (crowded longs = reversal risk)
// < -0.6 = extreme short crowding → LONG (crowded shorts = reversal risk)
// Entry: positioning extreme + price confirms direction
// Exit: positioning normalization OR MAX_HOLD bars

pub const POS_ENABLED: bool = true;
pub const POS_THRESHOLD: f64 = 0.60;        // extreme positioning threshold (0-1 scale)
pub const POS_MIN_AGE_BARS: u32 = 2;       // positioning must persist for 2 bars
pub const POS_SL: f64 = 0.005;            // 0.5% stop
pub const POS_TP: f64 = 0.003;            // 0.3% take profit
pub const POS_MAX_HOLD: u32 = 16;           // ~4 hours at 15m bars
pub const POS_FETCH_MINUTES: u64 = 5;       // 5min data
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BinanceLongShortRatio {
    #[serde(alias = "longAccount")]
    long_account: String,
    #[serde(alias = "shortAccount")]
    short_account: String,
    #[serde(alias = "symbol")]
    symbol: String,
}

impl BinanceLongShortRatio {
    fn long_ratio(&self) -> Option<f64> {
        self.long_account.parse().ok()
    }
    fn short_ratio(&self) -> Option<f64> {
        self.short_account.parse().ok()
    }
}

/// Fetch long/short ratio for a symbol from Binance futures public API.
/// No API key required.
pub async fn fetch_long_short_ratio(symbol: &str) -> Option<(f64, f64)> {
    let url = format!(
        "https://api.binance.com/futures/data/globalLongShortAccountRatio?symbol={}USDT&period=5m",
        symbol
    );
    let resp = reqwest::get(&url).await.ok()?;
    let data: Vec<BinanceLongShortRatio> = resp.json().await.ok()?;
    let latest = data.last()?;
    Some((latest.long_ratio()?, latest.short_ratio()?))
}

/// Compute positioning index from long/short ratios.
/// Returns value in [-1, +1]: positive = net long, negative = net short.
pub fn compute_positioning_index(long_ratio: f64, short_ratio: f64) -> f64 {
    long_ratio - short_ratio
}
```

Add to `CoinState` in `state.rs`:

```rust
pub long_ratio: f64,
pub short_ratio: f64,
pub positioning_index: f64,       // -1 to +1
pub positioning_index_prev: f64,
pub positioning_bars: u32,         // consecutive bars at extreme
```

Add entry logic in `engine.rs`:

```rust
/// Fires when positioning extreme persists for MIN_AGE_BARS bars
fn check_positioning_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::POS_ENABLED { return None; }

    let pi = state.positioning_index;
    let pi_prev = state.positioning_index_prev;
    if pi.is_nan() || pi_prev.is_nan() { return None; }

    let thresh = config::POS_THRESHOLD;
    let bars = state.positioning_bars;
    if bars < config::POS_MIN_AGE_BARS { return None; }

    // Extreme long crowding → SHORT
    if pi_prev < thresh && pi >= thresh {
        return Some((Direction::Short, "positioning"));
    }
    // Extreme short crowding → LONG
    if pi_prev > -thresh && pi <= -thresh {
        return Some((Direction::Long, "positioning"));
    }
    None
}
```

Note: Use BTC's positioning index as a market-wide signal — apply as a filter for altcoin entries (block longs when BTC positioning is extremely short, and vice versa).

---

## Expected Outcome

**Validation**: Backtest on BTC and 18 coins, 1-year 5min/15m data.
- **Baseline**: COINCLAW v16 (no positioning signal)
- **Comparison**: positioning trades tracked separately

**Metrics to measure**:
- Positioning index WR (hypothesis: >55%)
- PF on positioning trades
- BTC positioning as altcoin filter effectiveness
- Timing: how far ahead does positioning predict reversals?

**Hypothesis**: Extreme positioning ratios (>0.6 or <-0.6) persisting 2+ bars precede mean-reversion with WR >55% within 4-16 bars.

---

## Validation Method

1. **Historical backtest** (run158_1_pos_backtest.py):
   - BTC + 18 coins, 1-year 5min data (aggregated to 15m)
   - Fetch or simulate Binance long/short ratio data
   - Identify all positioning extremes
   - Record: positioning value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run158_2_pos_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep POS_THRESHOLD: 0.50 / 0.60 / 0.70 / 0.80

3. **Combined comparison** (run158_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + positioning
   - BTC positioning as filter vs no-filter for altcoins
   - Portfolio stats

---

## Out-of-Sample Testing

- Threshold sweep: 0.50 / 0.60 / 0.70 / 0.80
- MIN_AGE sweep: 1 / 2 / 3 bars
- OOS: final 4 months held out from all parameter selection
