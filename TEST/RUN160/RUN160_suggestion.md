# RUN160 — CME Day-Range Gap Signal: Prior Day Close/Open Relationship as Intraday Direction

## Hypothesis

**Mechanism**: Crypto's 24/7 market means "overnight" gaps are actually day-gaps relative to the prior UTC day close. When BTC opens a new UTC day significantly above or below the previous day's close, it creates a "gap" relative to the prior day's trading range. These gaps tend to get filled within the same UTC day — a reliable intraday pattern. COINCLAW uses 15m candles but doesn't track day-range gaps.

**Implementation**: Use Binance 1D klines (fetched once per day) to compute the prior day's close and today's open. If `today_open / yesterday_close - 1.0 > 0.3%`, it's a gap up — expect mean-reversion downward. If `< -0.3%`, it's a gap down — expect mean-reversion upward.

**Why this is not a duplicate**: RUN58 (Post-Event Gap Fill) was about CME weekend gaps (Friday close → Sunday open). RUN141 (CME Gap Fill) was also about weekend gaps. This RUN is specifically about **UTC day-range gaps** — a different time frame and mechanism. No prior RUN addresses UTC day-boundary gaps on Binance.

## Proposed Config Changes (config.rs)

```rust
// ── RUN160: CME Day-Range Gap Signal ────────────────────────────────
// Day gap = (today_open / yesterday_close) - 1.0
// Gap up > 0.3% → expect fill DOWN → SHORT
// Gap down < -0.3% → expect fill UP → LONG
// Entry: gap detected on first 15m bar of new UTC day
// Exit: gap filled (price crosses yesterday_close) OR end of UTC day

pub const DAY_GAP_ENABLED: bool = true;
pub const DAY_GAP_THRESH: f64 = 0.003;     // 0.3% gap to qualify
pub const DAY_GAP_SL: f64 = 0.004;         // 0.4% stop
pub const DAY_GAP_TP: f64 = 0.003;        // 0.3% take profit
pub const DAY_GAP_MAX_HOLD: u32 = 96;       // max hold until end of UTC day (~24 hours at 15m)
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Kline1d {
    #[serde(alias = "0")]
    open_time: u64,
    #[serde(alias = "1")]
    open: String,
    #[serde(alias = "2")]
    high: String,
    #[serde(alias = "3")]
    low: String,
    #[serde(alias = "4")]
    close: String,
    #[serde(alias = "5")]
    volume: String,
}

/// Fetch yesterday's close and today's open for gap detection.
/// Uses Binance 1D kline endpoint.
pub async fn fetch_day_range(symbol: &str) -> Option<(f64, f64, f64)> {
    // Today's open: most recent 1D kline
    let today_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}USDT&interval=1d&limit=2",
        symbol
    );
    let today_resp = reqwest::get(&today_url).await.ok()?;
    let today_klines: Vec<Kline1d> = today_resp.json().await.ok()?;
    let today_open = today_klines.last()?.open.parse().ok()?;

    // Yesterday's close: second-to-last 1D kline
    let yesterday_close = today_klines.get(today_klines.len() - 2)?.close.parse().ok()?;

    Some((today_open, yesterday_close, today_open / yesterday_close - 1.0))
}
```

Add to `CoinState` in `state.rs`:

```rust
pub yesterday_close: f64,
pub today_open: f64,
pub day_gap: f64,              // (today_open / yesterday_close) - 1.0
pub day_gap_active: bool,       // gap trade is active for today
pub day_gap_filled: bool,      // gap has been filled today
pub day_gap_direction: Option<Direction>,
```

Add entry logic in `engine.rs`:

```rust
/// Detect day gap on first bar of new UTC day and set up gap-fill trade
fn check_day_gap(state: &mut SharedState, ci: usize) {
    let cs = &mut state.coins[ci];
    if cs.pos.is_some() { return; }
    if !config::DAY_GAP_ENABLED { return; }
    if cs.day_gap_active { return; }  // already set up for today

    let (today_open, yesterday_close, gap) = match fetch_day_range(cs.binance_symbol).await {
        Some(g) => g,
        None => return,
    };

    cs.yesterday_close = yesterday_close;
    cs.today_open = today_open;
    cs.day_gap = gap;
    cs.day_gap_filled = false;

    let thresh = config::DAY_GAP_THRESH;
    if gap > thresh {
        cs.day_gap_direction = Some(Direction::Short);
        cs.day_gap_active = true;
    } else if gap < -thresh {
        cs.day_gap_direction = Some(Direction::Long);
        cs.day_gap_active = true;
    }
}

/// Entry fires when day gap is active and price has moved toward filling
fn check_day_gap_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if !cs.day_gap_active { return None; }

    let ind = cs.ind_15m.as_ref()?;
    let dir = cs.day_gap_direction?;

    // For gap up: price must pull back (not continue up) to enter SHORT
    // For gap down: price must rally back (not continue down) to enter LONG
    let entry_ready = match dir {
        Direction::Short => ind.p < cs.today_open,  // price came back down
        Direction::Long => ind.p > cs.today_open,    // price came back up
    };

    if entry_ready { Some((dir, "day_gap")) } else { None }
}

/// Exit fires when gap is filled (price crosses yesterday_close)
fn check_day_gap_exit(state: &mut SharedState, ci: usize) -> bool {
    let cs = &state.coins[ci];
    let pos = cs.pos.as_ref()?;
    if pos.trade_type != Some(TradeType::Regime) { return false; }
    if !cs.day_gap_active { return false; }

    let ind = cs.ind_15m.as_ref()?;

    // Gap filled: price crossed yesterday_close
    match cs.day_gap_direction {
        Some(Direction::Short) => {
            if ind.p <= cs.yesterday_close {
                close_position(state, ci, ind.p, "GAP_FILLED", TradeType::Regime);
                cs.day_gap_active = false;
                cs.day_gap_filled = true;
                return true;
            }
        }
        Some(Direction::Long) => {
            if ind.p >= cs.yesterday_close {
                close_position(state, ci, ind.p, "GAP_FILLED", TradeType::Regime);
                cs.day_gap_active = false;
                cs.day_gap_filled = true;
                return true;
            }
        }
        None => {}
    }
    false
}
```

Note: `check_day_gap` runs once per day (on the first fetch of a new UTC day). `check_day_gap_entry` runs every 15m bar. Only one gap trade per coin per UTC day maximum.

---

## Expected Outcome

**Validation**: Backtest on BTC and 18 coins, 1-year 15m data with daily kline data.
- **Baseline**: COINCLAW v16 (no day-gap strategy)
- **Comparison**: day-gap trades tracked separately

**Metrics to measure**:
- Day-gap WR (hypothesis: >55%)
- PF on day-gap trades
- Gap fill completion rate (what % of gaps fully fill?)
- Timing: avg time to gap fill

**Hypothesis**: UTC day-range gaps >0.3% fill within the same UTC day with WR >55% because market makers arbitrage the gap toward yesterday's close.

---

## Validation Method

1. **Historical backtest** (run160_1_daygap_backtest.py):
   - BTC + 18 coins, 1-year 15m data with daily klines
   - Identify all day-range gaps >0.3%
   - Simulate gap-fill entry when price reverts toward yesterday's close
   - Record: gap magnitude, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time, fill %

2. **Walk-forward** (run160_2_daygap_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep DAY_GAP_THRESH: 0.002 / 0.003 / 0.005 / 0.008

3. **Combined comparison** (run160_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + day_gap
   - Portfolio stats, gap trade contribution

---

## Out-of-Sample Testing

- THRESH sweep: 0.002 / 0.003 / 0.005 / 0.008
- OOS: final 4 months held out from all parameter selection
