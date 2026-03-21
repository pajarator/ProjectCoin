# RUN106 — Hourly Scalp Cooldown: Scalp Cooldown Periods Scaled by UTC Session

## Hypothesis

**Named:** `hourly_scalp_cooldown`

**Mechanism:** COINCLAW currently uses a fixed 2-bar cooldown after scalp exits. But scalp opportunities vary by time of day — during high-volatility US market hours (UTC 13:00-20:00), scalp signals fire more frequently and the 2-bar cooldown is too short, allowing rapid re-entry into volatile conditions. During the quiet Asia session (UTC 00:00-08:00), signals are sparse and the same 2-bar cooldown may be unnecessarily long, blocking valid opportunities. The Hourly Scalp Cooldown scales the cooldown by UTC session: shorter during quiet periods, longer during volatile ones.

**Hourly Scalp Cooldown:**
- Divide UTC day into sessions:
  - Asia (UTC 00:00-08:00): quiet → `COOLDOWN_ASIA = 3` bars
  - Europe/US overlap (UTC 08:00-13:00): moderate → `COOLDOWN_EU = 2` bars (default)
  - US market (UTC 13:00-20:00): volatile → `COOLDOWN_US = 1` bar (shorter to capture more scalp opportunities)
  - Late US (UTC 20:00-24:00): moderate → `COOLDOWN_LATE = 2` bars
- Cooldown applies after all scalp exits (TP, SL, MAX_HOLD)

**Why this is not a duplicate:**
- RUN39 (asymmetric win/loss cooldown) differentiated by win vs loss, not by time-of-day session
- RUN41 (session filter) blocked entries entirely during certain sessions — this adjusts cooldown duration
- RUN83 (cooldown by market mode) scaled cooldowns by market mode — this scales scalp cooldowns by UTC session
- No prior RUN has adjusted scalp cooldown periods based on intraday session

**Mechanistic rationale:** Scalp trades fire on 1m timeframe volatility. The frequency of scalp opportunities varies by session — US market hours have more volatility and more scalp signals. A session-adaptive cooldown ensures the cooldown period is proportional to the opportunity frequency: longer in quiet sessions (avoid overtrading), shorter in volatile sessions (capture more opportunities).

---

## Proposed Config Changes

```rust
// RUN106: Hourly Scalp Cooldown
pub const HOURLY_SCALP_COOLDOWN_ENABLE: bool = true;
pub const SCALP_COOLDOWN_ASIA: u32 = 3;    // UTC 00:00-08:00
pub const SCALP_COOLDOWN_EU: u32 = 2;      // UTC 08:00-13:00
pub const SCALP_COOLDOWN_US: u32 = 1;      // UTC 13:00-20:00
pub const SCALP_COOLDOWN_LATE: u32 = 2;     // UTC 20:00-24:00
```

**`engine.rs` — get_session_scalp_cooldown:**
```rust
/// Get the scalp cooldown period based on current UTC session.
fn get_session_scalp_cooldown() -> u32 {
    use chrono::Timelike;
    let utc_hour = chrono::Utc::now().hour();

    if !config::HOURLY_SCALP_COOLDOWN_ENABLE {
        return 2;  // default
    }

    if utc_hour >= 0 && utc_hour < 8 {
        config::SCALP_COOLDOWN_ASIA
    } else if utc_hour >= 8 && utc_hour < 13 {
        config::SCALP_COOLDOWN_EU
    } else if utc_hour >= 13 && utc_hour < 20 {
        config::SCALP_COOLDOWN_US
    } else {
        config::SCALP_COOLDOWN_LATE
    }
}

// In close_position — for scalp trades, use session cooldown:
if trade_type == TradeType::Scalp {
    cs.cooldown = get_session_scalp_cooldown();
} else {
    // Regime trades use existing cooldown logic
}
```

---

## Validation Method

### RUN106.1 — Hourly Scalp Cooldown Grid Search (Rust, parallel across 18 coins)

**Data:** 1m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed 2-bar scalp cooldown

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SCALP_COOLDOWN_ASIA` | [1, 2, 3, 4] |
| `SCALP_COOLDOWN_US` | [1, 2] |
| `SCALP_COOLDOWN_LATE` | [1, 2, 3] |

**Per coin:** 4 × 2 × 3 = 24 configs × 18 coins = 432 backtests

**Key metrics:**
- `avg_scalp_cooldown`: average cooldown by session
- `session_entry_rate_delta`: change in entry rate by session
- `scalp_PF_delta`: scalp profit factor change vs baseline
- `scalp_PnL_delta`: scalp P&L change vs baseline

### RUN106.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best cooldown per session per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS scalp P&L delta vs baseline
- Session entry rates shift appropriately (higher during volatile sessions)

### RUN106.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 2-bar) | Hourly Scalp Cooldown | Delta |
|--------|--------------------------|---------------------|-------|
| Scalp P&L | $X | $X | +$X |
| Scalp Win Rate | X% | X% | +Ypp |
| Scalp Profit Factor | X.XX | X.XX | +0.XX |
| Asia Session Cooldown | 2 | X | +N |
| US Session Cooldown | 2 | X | -N |
| Asia Entry Rate | X | X | -Y% |
| US Entry Rate | X | X | +Y% |

---

## Why This Could Fail

1. **Crypto has no opening bell:** Unlike equities, crypto trades 24/7. The session effect may be weak compared to equities, and the boundaries (UTC 8, 13, 20) are somewhat arbitrary.
2. **Scalp cooldown affects scalp frequency:** Adjusting cooldown may not move the needle if scalp entry signals are already the limiting factor.

---

## Why It Could Succeed

1. **Session volatility patterns are real:** US market hours (UTC 13-20) do have higher crypto volatility. A shorter cooldown during these periods allows capturing more of the available scalp opportunities.
2. **Aligns with market microstructure:** The cooldown period should match the opportunity frequency. Quiet periods should have longer cooldowns to prevent overtrading; volatile periods should have shorter cooldowns.
3. **Simple and additive:** Just a UTC hour lookup and a cooldown assignment. No new indicators needed.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN106 Hourly Scalp Cooldown |
|--|--|--|
| Scalp cooldown (Asia) | 2 bars | 3 bars |
| Scalp cooldown (US) | 2 bars | 1 bar |
| Session awareness | None | UTC hour-based |
| Entry rate during US session | X | X + Y% |
| Entry rate during Asia session | X | X - Y% |
| Overtrading prevention | Fixed 2-bar | Adaptive |
