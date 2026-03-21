# RUN161 — Binance ETF NAV Premium Signal: Institutional Premium/Discount as Directional Indicator

## Hypothesis

**Mechanism**: BTC ETF instruments (Grayscale GBTC, BlackRock IBIT, etc.) trade at a premium or discount to their NAV based on institutional demand. When BTC trades at a significant premium to its "fair value" (via ETF proxies), it signals overvalued conditions. When at a discount, undervalued. The Binance BTCDOM index or BTCO compound tracking provides a similar signal. COINCLAW has no ETF or NAV data. This RUN uses Binance's composite index as a NAV proxy.

**API to use**:
- Binance: `GET https://api.binance.com/api/v3/ticker/price?symbol=BTCOUSDT` — BTCO perpetual price
  - Compare to BTCUSDT: `BTCOUSDT / BTCUSDT - 1.0` = premium/discount ratio
  - Or use Bybit: `GET https://api.bybit.com/v5/market/tickers?category=perpetual&symbol=BTCUSDT` — has index price field
- Bybit index price: `https://api.bybit.com/v5/market/tickers?category=perpetual&symbol=BTCUSDT` → `indexPrice`

**Premium calculation**: `btco_perpetual_price / btc_spot_price - 1.0`. When premium > 0.1% = BTC overvalued vs composite. When premium < -0.1% = BTC undervalued.

**Why this is not a duplicate**: No prior RUN uses ETF premium or NAV data. This is a distinct signal class — institutional pricing efficiency vs spot.

## Proposed Config Changes (config.rs)

```rust
// ── RUN161: ETF NAV Premium Signal ─────────────────────────────────────
// Premium = (BTCO_perpetual / BTC_spot) - 1.0
// Premium > 0.1% = BTC overvalued → SHORT BTC, block LONG alts
// Premium < -0.1% = BTC undervalued → LONG BTC, block SHORT alts
// Entry: premium crosses threshold (for BTC direct trades)
// Filter: block opposing altcoin entries when premium is extreme
// Exit: premium normalization

pub const ETF_ENABLED: bool = true;
pub const ETF_PREMIUM_THRESH: f64 = 0.001;    // 0.1% premium/discount threshold
pub const ETF_DISCOUNT_THRESH: f64 = -0.001;
pub const ETF_SL: f64 = 0.004;              // 0.4% stop
pub const ETF_TP: f64 = 0.003;             // 0.3% take profit
pub const ETF_MAX_HOLD: u32 = 16;             // ~4 hours at 15m bars
```

Add to `coinclaw/src/fetcher.rs`:

```rust
/// Fetch BTC index price from Bybit (more accurate NAV proxy)
pub async fn fetch_btc_index_price() -> Option<f64> {
    let url = "https://api.bybit.com/v5/market/tickers?category=perpetual&symbol=BTCUSDT";
    let resp = reqwest::get(url).await.ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;
    let index_price = data["result"]["list"][0]["indexPrice"].as_str()?.parse().ok()?;
    Some(index_price)
}

/// Fetch BTCO perpetual price from Binance
pub async fn fetch_btco_perpetual_price() -> Option<f64> {
    let url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCOUSDT";
    let resp = reqwest::get(url).await.ok()?;
    let price = resp["price"].as_str()?.parse().ok()?;
    Some(price)
}

/// Compute ETF premium: (BTCO_perpetual / BTC_index) - 1.0
pub async fn compute_etf_premium() -> Option<f64> {
    let btco = fetch_btco_perpetual_price().await?;
    let btc_index = fetch_btc_index_price().await?;
    if btc_index == 0.0 { return None; }
    Some(btco / btc_index - 1.0)
}
```

Add to `SharedState` in `state.rs`:

```rust
pub etf_premium: f64,
pub etf_premium_prev: f64,
pub etf_premium_bars: u32,  // consecutive bars at extreme
```

Add entry logic in `engine.rs`:

```rust
/// ETF premium as a blocking filter for all altcoin entries
fn etf_filter_entries(state: &SharedState, proposed_dir: Direction) -> bool {
    if !config::ETF_ENABLED { return true; }
    if state.etf_premium.abs() < config::ETF_PREMIUM_THRESH { return true; }  // no filter

    match proposed_dir {
        Direction::Long if state.etf_premium < config::ETF_DISCOUNT_THRESH => {
            // BTC deeply discounted → alts overvalued in BTCO terms → block LONG
            return false;
        }
        Direction::Short if state.etf_premium > config::ETF_PREMIUM_THRESH => {
            // BTC deeply premium → alts undervalued in BTCO terms → block SHORT
            return false;
        }
        _ => {}
    }
    true
}

/// BTC direct entry when ETF premium crosses threshold
fn check_etf_entry(state: &mut SharedState, btc_ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[btc_ci];
    if cs.pos.is_some() { return None; }
    if !config::ETF_ENABLED { return None; }

    let prem = state.etf_premium;
    let prem_prev = state.etf_premium_prev;
    if prem.abs() < config::ETF_PREMIUM_THRESH { return None; }
    if prem.abs() < state.etf_premium_prev.abs() { return None; }  // not widening

    if prem > config::ETF_PREMIUM_THRESH {
        return Some((Direction::Short, "etf_premium"));  // BTC premium = overvalued
    }
    if prem < config::ETF_DISCOUNT_THRESH {
        return Some((Direction::Long, "etf_premium"));   // BTC discount = undervalued
    }
    None
}
```

Note: ETF premium filter applies to all coins — when premium is extreme, block the opposing direction for all positions. Use BTC's ETF signal as a market-wide directional filter.

---

## Expected Outcome

**Validation**: Backtest on BTC and 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no ETF premium signal)
- **Comparison**: ETF-filtered vs unfiltered entries

**Metrics to measure**:
- ETF signal directional accuracy (does premium/discount predict BTC direction?)
- Altcoin entry quality improvement when filtered
- Blocking rate (% entries blocked during extreme premium periods)
- Improvement in altcoin WR when filtered

**Hypothesis**: When ETF premium > 0.1% or < -0.1%, blocking opposing altcoin entries improves WR by >3pp because institutional pricing efficiency signals misaligned positions.

---

## Validation Method

1. **Historical backtest** (run161_1_etf_backtest.py):
   - BTC + 18 coins, 1-year 15m data
   - Reconstruct ETF premium from BTCO and BTC index prices
   - Compare: filtered vs unfiltered regime entries
   - Record: premium value, blocked trade outcomes, directional accuracy
   - Output: per-coin WR change, PF change

2. **Walk-forward** (run161_2_etf_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep ETF_PREMIUM_THRESH: 0.0005 / 0.0010 / 0.0015 / 0.0020

3. **Combined comparison** (run161_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + etf_filter
   - Portfolio stats, blocking analysis

---

## Out-of-Sample Testing

- THRESH sweep: 0.0005 / 0.0010 / 0.0015 / 0.0020
- OOS: final 4 months held out from all parameter selection
