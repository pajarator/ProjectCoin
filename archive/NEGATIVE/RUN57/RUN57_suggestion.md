# RUN57 — Day-of-Week Trade Filter: Suppressing Low-Edge Days

## Hypothesis

**Named:** `day_of_week_filter`

**Mechanism:** Even though crypto trades 24/7, institutional participation (from TradFi crossover products, custodians, and larger market participants) follows a traditional market week structure:
- **Monday:** Weekend accumulation/liquidation unwinds — higher volatility, potentially different mean-reversion dynamics
- **Friday:** Pre-weekend positioning — traders reduce exposure before weekend
- **Weekdays (Tue-Thu):** "Normal" trading conditions where mean-reversion strategies are calibrated

The hypothesis is that COINCLAW's mean-reversion strategies perform differently by day of week, and selectively engaging or suppressing trades on specific days improves overall win rate.

**Additional angle — weekend effect:** Saturdays and Sundays may have distinct dynamics (lower volume, different volatility profile). The system currently trades all days equally.

**Why this is not a duplicate:**
- No prior RUN has tested day-of-week as a trade filter
- RUN21 mentioned `day_of_week` as a feature but never tested it as a conditional filter
- Calendar effects (day-of-week, hour-of-day) are fundamentally different from indicator-based signals
- This is a market microstructure filter, not a price/volume indicator filter

---

## Proposed Config Changes

```rust
// RUN57: Day-of-Week Trade Filter
// day_mask: bitmask of allowed days (bit 0=Monday, bit 6=Sunday)
pub const DOW_FILTER_ENABLE: bool = false;
pub const DOW_ALLOWED_DAYS: u8 = 0b1111100;  // Mon-Fri allowed (0x3C = 60), weekend suppressed
pub const DOW_SUPPRESS_MODE: u8 = 1;  // 0=disabled, 1=suppress_bad_days, 2=allow_good_days_only
```

**`engine.rs` — check entry day-of-week gate:**
```rust
fn is_allowed_day() -> bool {
    if !config::DOW_FILTER_ENABLE { return true; }
    let dow = chrono::Local::now().weekday().num_days_from_monday() as u8;  // 0=Mon, 6=Sun
    match config::DOW_SUPPRESS_MODE {
        1 => (config::DOW_ALLOWED_DAYS >> dow) & 1 == 1,  // allowed days
        2 => (config::DOW_ALLOWED_DAYS >> dow) & 1 == 0,  // suppress allowed, block others
        _ => true,
    }
}

pub fn check_entry(...) {
    // ... existing checks ...
    if !is_allowed_day() { return; }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN57.1 — Day-of-Week Performance Profiling (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset. Extract day-of-week from each candle timestamp.

**Step 1 — Profile current COINCLAW strategies by day-of-week:**

For each coin:
1. Run COINCLAW v13 strategy on full year
2. Tag each trade with the day-of-week (Mon-Sun) of its entry bar
3. Compute per-day: WR%, PF, total_PnL, trade_count, avg_win%, avg_loss%

**Step 2 — Grid search:**

| Parameter | Values |
|-----------|--------|
| `DOW_SUPPRESS_MODE` | [1=suppress_bad_days, 2=allow_good_days_only] |
| `MONDAY_ALLOWED` | [true, false] |
| `FRIDAY_ALLOWED` | [true, false] |
| `WEEKEND_ALLOWED` | [true, false] |

8 configs × 18 coins = 144 backtests

**Also measure:** Is there a day-of-week effect for scalp trades specifically vs regime trades? Scalp (1m) may have different day-of-week dynamics than regime (15m).

**Key metrics:**
- `best_day`: day with highest WR% and PF
- `worst_day`: day with lowest WR% and PF
- `dow_WR_spread`: best_day_WR% − worst_day_WR%
- `suppression_rate`: % of trades blocked by day filter

### RUN57.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: identify best/worst days from train half, suppress worst
2. Test: evaluate with day filter on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Best/worst days are consistent across ≥ 2/3 windows (not random noise)
- Portfolio OOS P&L ≥ baseline

### RUN57.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Day-of-Week Filter | Delta |
|--------|---------------|-------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K (−X%) |
| Monday WR% | X% | X% | +Ypp |
| Friday WR% | X% | X% | +Ypp |
| Weekend WR% | X% | X% | +Ypp |
| Suppression Rate | 0% | X% | — |
