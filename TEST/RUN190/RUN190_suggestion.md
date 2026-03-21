# RUN190 — Time-of-Day Volatility Filter: Session-Aware Trade Calibration

## Hypothesis

**Mechanism**: Crypto markets have predictable intraday volatility cycles:
- **Asian session** (00:00–08:00 UTC): low volatility, choppy range-bound action → favor mean-reversion, tighter stops
- **London session** (08:00–12:00 UTC): rising volatility, directional moves begin → neutral
- **New York session** (12:00–20:00 UTC): peak volatility, strongest directional moves → favor momentum, wider stops
- **Overnight tail** (20:00–00:00 UTC): declining volatility → exit and reduce exposure

Apply different SL, TP, and strategy selection based on the UTC hour bucket.

**Why not duplicate**: No prior RUN adjusts parameters based on time-of-day. All prior RUNs use static parameters across all hours. Session-aware calibration is a new dimension.

## Proposed Config Changes (config.rs)

```rust
// ── RUN190: Time-of-Day Volatility Filter ────────────────────────────────
// Session windows (UTC):
// ASIAN:   00:00 – 08:00 UTC → low vol → SL=0.002, TP=0.003, favor_mean_rev
// LONDON:  08:00 – 12:00 UTC → rising vol → SL=0.003, TP=0.004, neutral
// NY_AM:  12:00 – 16:00 UTC → high vol → SL=0.004, TP=0.005, favor_momentum
// NY_PM:  16:00 – 20:00 UTC → peak vol → SL=0.005, TP=0.006, favor_momentum
// TAIL:   20:00 – 00:00 UTC → declining vol → SL=0.002, TP=0.003, exit_only

pub const TOD_FILTER_ENABLED: bool = true;
pub const TOD_ASIAN_SL: f64 = 0.002;
pub const TOD_ASIAN_TP: f64 = 0.003;
pub const TOD_LONDON_SL: f64 = 0.003;
pub const TOD_LONDON_TP: f64 = 0.004;
pub const TOD_NY_AM_SL: f64 = 0.004;
pub const TOD_NY_AM_TP: f64 = 0.005;
pub const TOD_NY_PM_SL: f64 = 0.005;
pub const TOD_NY_PM_TP: f64 = 0.006;
pub const TOD_TAIL_SL: f64 = 0.002;
pub const TOD_TAIL_TP: f64 = 0.003;
```

Add in `engine.rs` or create a helper:

```rust
pub fn get_session_params(hour_utc: u32) -> (f64, f64, &str) {
    match hour_utc {
        0..=7  => (config::TOD_ASIAN_SL, config::TOD_ASIAN_TP, "ASIAN"),
        8..=11 => (config::TOD_LONDON_SL, config::TOD_LONDON_TP, "LONDON"),
        12..=15 => (config::TOD_NY_AM_SL, config::TOD_NY_AM_TP, "NY_AM"),
        16..=19 => (config::TOD_NY_PM_SL, config::TOD_NY_PM_TP, "NY_PM"),
        _      => (config::TOD_TAIL_SL, config::TOD_TAIL_TP, "TAIL"),
    }
}
```

Modify `open_position` in `engine.rs` to use session-aware SL/TP.

---

## Validation Method

1. **Historical backtest** (run190_1_tod_backtest.py) — stratified by session
2. **Walk-forward** (run190_2_tod_wf.py)
3. **Combined** (run190_3_combined.py)

## Out-of-Sample Testing

- Session boundary sweep (shift ±1 hour)
- Volatility-adaptive thresholds based on realized vol within session
