# RUN153 — Liquidation Cluster Signal: Open Interest Spike Fade

## Hypothesis

**Mechanism**: When open interest (OI) spikes suddenly, it often precedes cascade liquidations — traders are over-leveraged and a sharp price move triggers mass liquidations, which then reverse. By monitoring OI changes via Binance public API, COINCLAW can detect these spike clusters and fade the resulting volatility spike. The reversal after a liquidation cascade is fast and predictable.

**API to use**:
- Binance: `GET https://api.binance.com/fapi/v1/openInterest?symbol={SYMBOL}USDT`
  - Response: `{"openInterest": "12345.1234", "symbol": "BTCUSDT"}`
  - Update: real-time
  - No API key required

**OI Spike calculation**: Compare current OI to 24-bar rolling average. If current OI > avg OI × OI_SPIK_MULT, it's a spike. Direction: price drops after spike → LONG (cascading long liquidations); price rises after spike → SHORT (cascading short liquidations).

**Why this is not a duplicate**: No prior RUN uses open interest or liquidation data. All signals are OHLCV-based. OI spike is a structural market signal that predicts cascade reversals — orthogonal to all price-action signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN153: Liquidation Cluster Signal ────────────────────────────────
// OI Spike = current OI > rolling_avg_OI * OI_SPIKE_MULT
// After spike: price drops → LONG (longs liquidated); price rises → SHORT (shorts liquidated)
// Exit: OI mean-reverts OR MAX_HOLD bars

pub const OI_ENABLED: bool = true;
pub const OI_SPIKE_MULT: f64 = 1.5;       // OI must be 1.5x the 24-bar rolling average
pub const OI_ROLLING_WINDOW: usize = 24;   // 24 bars (~6 hours) rolling window
pub const OI_PRICE_MOVE_THRESH: f64 = 0.003;  // 0.3% price move after spike to confirm direction
pub const OI_SL: f64 = 0.005;            // 0.5% stop (wider — liquidation cascades are volatile)
pub const OI_TP: f64 = 0.004;            // 0.4% take profit
pub const OI_MAX_HOLD: u32 = 12;           // ~3 hours at 15m bars
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OpenInterestResponse {
    #[serde(alias = "openInterest")]
    open_interest: String,
    #[serde(alias = "symbol")]
    symbol: String,
}

impl OpenInterestResponse {
    fn oi(&self) -> Option<f64> {
        self.open_interest.replace(".", "").replace("e+", "E").parse::<f64>().ok()
    }
}

/// Fetch open interest for a symbol from Binance futures public API.
/// No API key required.
pub async fn fetch_open_interest(symbol: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/fapi/v1/openInterest?symbol={}USDT",
        symbol
    );
    let resp = reqwest::get(&url).await.ok()?;
    let data: OpenInterestResponse = resp.json().await.ok()?;
    data.oi()
}
```

Add to `CoinState` in `state.rs`:

```rust
pub open_interest: f64,
pub oi_rolling_avg: f64,
pub oi_rolling_history: Vec<f64>,  // rolling window of past OI values
pub oi_spike_bars: u32,            // bars since OI spike detected
pub oi_spike_direction: Option<Direction>,  // direction of the spike
```

Add entry logic in `engine.rs`:

```rust
/// Fires when OI spike detected and price has moved > threshold in opposite direction
/// (cascading liquidations confirmed)
fn check_oi_spike_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::OI_ENABLED { return None; }

    if cs.oi_spike_bars == 0 { return None; }  // no active spike

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let price_move = match cs.oi_spike_direction {
        Some(Direction::Long) => {
            // Price dropped after spike → longs liquidated → LONG entry
            let drop = (cs.oi_spike_entry_price - ind.p) / cs.oi_spike_entry_price;
            if drop >= config::OI_PRICE_MOVE_THRESH { Direction::Long } else { return None; }
        }
        Some(Direction::Short) => {
            // Price rose after spike → shorts liquidated → SHORT entry
            let rise = (ind.p - cs.oi_spike_entry_price) / cs.oi_spike_entry_price;
            if rise >= config::OI_PRICE_MOVE_THRESH { Direction::Short } else { return None; }
        }
        None => return None,
    };

    Some((price_move, "liq_spike"))
}
```

Note: OI spike is detected during the fetch cycle. When `current_oi > oi_rolling_avg * OI_SPIKE_MULT`, set `oi_spike_bars = 1` and record `oi_spike_entry_price = ind.p` and direction from price movement at spike time.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data (requires OI history — may use simulated OI based on price volatility if historical OI unavailable).
- **Baseline**: COINCLAW v16 (no OI signal)
- **Comparison**: OI spike trades tracked separately

**Metrics to measure**:
- Liquidation spike WR (hypothesis: >55%)
- PF on OI spike trades
- OI spike frequency per coin
- Price move magnitude after spike vs outcome

**Hypothesis**: OI spikes precede cascade liquidations which reverse predictably. When OI > 1.5× rolling average AND price has moved >0.3% in the direction opposite to the crowded side, the reversal probability within 3-12 bars is >55%.

---

## Validation Method

1. **Historical backtest** (run153_1_oi_backtest.py):
   - 18 coins, 1-year 15m data
   - Fetch or simulate OI data (Binance has public OI history)
   - Identify all OI spikes
   - Record: OI magnitude, price move direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run153_2_oi_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep OI_SPIKE_MULT: 1.3 / 1.5 / 2.0
   - Sweep OI_PRICE_MOVE_THRESH: 0.002 / 0.003 / 0.005

3. **Combined comparison** (run153_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + oi_spike
   - Portfolio stats, correlation with regime trades

---

## Out-of-Sample Testing

- SPIKE_MULT sweep: 1.3 / 1.5 / 2.0 / 2.5
- PRICE_MOVE_THRESH sweep: 0.002 / 0.003 / 0.005 / 0.008
- OOS: final 4 months held out from all parameter selection
