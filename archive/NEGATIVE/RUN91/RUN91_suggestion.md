# RUN91 — Hourly Z-Threshold Scaling: Time-of-Day Adaptive Entry Thresholds

## Hypothesis

**Named:** `hourly_z_threshold`

**Mechanism:** COINCLAW's regime entry z-score thresholds (e.g., z < -1.5 for longs) are fixed throughout the day. But crypto volatility follows a predictable intraday pattern: UTC 13:00-20:00 (overlapping with US equity market open) sees higher average volatility; UTC 00:00-08:00 (Asia session) sees lower volatility. A fixed z-threshold of -1.5 means different things in different volatility regimes: in high-vol periods, z = -1.5 is less extreme than in low-vol periods.

**Hourly Z-Threshold Scaling:**
- Divide UTC into high-vol and low-vol windows:
  - High-vol hours (UTC 13:00-20:00): scale z-threshold by `HIGH_VOL_MULT` (e.g., 1.2×) — require more extreme z to enter (z < -1.5 × 1.2 = -1.8)
  - Low-vol hours (UTC 00:00-08:00): scale by `LOW_VOL_MULT` (e.g., 0.8×) — allow less extreme z to enter (z < -1.5 × 0.8 = -1.2)
- The scaling adjusts the threshold, not the entry signal directly
- This is applied per coin, not portfolio-wide

**Why this is not a duplicate:**
- RUN41 (session filter) blocked entries entirely during certain hours — this scales the z-threshold within sessions
- RUN84 (session partial scaling) scaled partial exit tiers by session — this scales entry z-thresholds by session
- No prior RUN has adapted the entry z-score threshold itself based on time-of-day volatility patterns

**Mechanistic rationale:** In high-volatility periods, coins swing further from their means — z = -1.5 is a less extreme reading because the typical swing is larger. Requiring a more extreme z-threshold during high-vol hours ensures entries are still high-conviction despite larger noise swings. In low-vol periods, z = -1.5 is genuinely extreme — tightening the threshold captures opportunities that would otherwise be missed.

---

## Proposed Config Changes

```rust
// RUN91: Hourly Z-Threshold Scaling
pub const HOURLY_Z_ENABLE: bool = true;

// Hour windows (UTC)
pub const HIGH_VOL_START: u8 = 13;   // UTC 13:00 (US market open)
pub const HIGH_VOL_END: u8 = 20;     // UTC 20:00
pub const LOW_VOL_START: u8 = 0;     // UTC 00:00 (Asia session)
pub const LOW_VOL_END: u8 = 8;        // UTC 08:00

// Threshold multipliers
pub const HIGH_VOL_Z_MULT: f64 = 1.20;   // tighten threshold during high-vol (×1.2)
pub const LOW_VOL_Z_MULT: f64 = 0.80;    // relax threshold during low-vol (×0.8)
```

**`strategies.rs` — long_entry / short_entry with hourly scaling:**
```rust
/// Get the hourly-adjusted z-entry threshold for LONG entries.
fn hourly_long_z_threshold() -> f64 {
    if !config::HOURLY_Z_ENABLE { return -1.5; }  // default

    use chrono::Timelike;
    let utc_hour = chrono::Utc::now().hour();

    let base = -1.5;  // base z threshold
    if utc_hour >= config::HIGH_VOL_START && utc_hour < config::HIGH_VOL_END {
        base * config::HIGH_VOL_Z_MULT  // more negative: require more extreme
    } else if utc_hour >= config::LOW_VOL_START && utc_hour < config::LOW_VOL_END {
        base * config::LOW_VOL_Z_MULT   // less negative: allow less extreme
    } else {
        base
    }
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    let z_thresh = hourly_long_z_threshold();
    if ind.z < z_thresh { return false; }  // scaled z threshold

    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN91.1 — Hourly Z-Threshold Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed z-threshold -1.5 for longs

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `HIGH_VOL_Z_MULT` | [1.1, 1.2, 1.3] |
| `LOW_VOL_Z_MULT` | [0.7, 0.8, 0.9] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `avg_z_thresh_used`: average z-threshold applied by hour
- `entry_rate_delta`: change in entry rate by session
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_z_at_entry_delta`: change in average |z| at entry (should increase if filtering is working)

### RUN91.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best HIGH_VOL_MULT × LOW_VOL_MULT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- High-vol session entry rate decreases (more selective)
- Low-vol session entry rate increases (more opportunities captured)

### RUN91.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed -1.5) | Hourly Z-Threshold | Delta |
|--------|--------------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| High-Vol Entry Rate | X | X | -Y% |
| Low-Vol Entry Rate | X | X | +Y% |
| Avg Z at Entry (high-vol) | X | X | +N |
| Avg Z at Entry (low-vol) | X | X | -N |

---

## Why This Could Fail

1. **UTC hour is arbitrary for crypto:** Unlike equities, crypto trades 24/7. The "US market open" volatility effect may not be strong enough to justify separate thresholds.
2. **Volatility patterns shift over time:** The typical high-vol and low-vol hours may change as the crypto market evolves. Thresholds learned on historical data may not generalize.
3. **Interactions with breadth:** The market mode (LONG/ISO_SHORT/SHORT) already captures some volatility information. Adding hourly scaling may be redundant.

---

## Why It Could Succeed

1. **Intraday volatility is real:** Historical analysis shows that crypto volatility does follow an intraday pattern — US market hours see more volume and larger moves. Scaling thresholds accordingly is principled.
2. **Higher selectivity during noisy periods:** During high-vol periods, coins make larger swings that can trigger false z-entry signals. A tighter threshold during these periods reduces false entries.
3. **Captures more opportunities during quiet periods:** During Asia session, a smaller z-deviation is genuinely extreme. A relaxed threshold captures these setups that would be missed by a fixed threshold.
4. **Simple and additive:** Just a lookup table by UTC hour. Easy to compute, no new data.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN91 Hourly Z-Threshold |
|--|--|--|
| Z-threshold (high-vol) | -1.5 (fixed) | -1.8 (×1.2) |
| Z-threshold (low-vol) | -1.5 (fixed) | -1.2 (×0.8) |
| Entry selectivity | Same all day | Adaptive by hour |
| High-vol handling | No adjustment | More extreme z required |
| Low-vol handling | No adjustment | Less extreme z allowed |
| Implementation | None | UTC hour lookup + multiplier |
