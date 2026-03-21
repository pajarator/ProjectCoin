# RUN330 — Session-Volume Adaptive Volatility Bands: Asia/London/NY Regime

## Hypothesis

**Mechanism**: Different trading sessions have distinct volatility profiles. Asia session (00:00-08:00 UTC) is typically lower volatility, London (08:00-16:00 UTC) has medium volatility, NY (13:00-21:00 UTC) has highest volatility. Adapt the Bollinger Band standard deviation multiplier based on the current session's typical volatility range. When actual BB width exceeds the session's expected range → volatility expansion breakout.

**Why not duplicate**: No prior RUN uses session-adaptive volatility bands. RUN41 uses session-based filtering. RUN91 uses hourly Z-threshold scaling. This RUN specifically adapts band width per session, not just filtering by session. The distinct mechanism is session-specific BB width calibration.

## Proposed Config Changes (config.rs)

```rust
// ── RUN330: Session-Volume Adaptive Volatility Bands ───────────────────────────
// For each session (ASIA/LONDON/NY), compute typical BB width as ratio of price
// ASIA_bb_mult = 1.5  (narrower bands, lower vol)
// LONDON_bb_mult = 2.0  (medium bands)
// NY_bb_mult = 2.5  (wider bands, higher vol)
// LONG: price crosses above upper_band AND BB_width > session_expected * VOL_EXPAND_THRESH
// SHORT: price crosses below lower_band AND BB_width > session_expected * VOL_EXPAND_THRESH

pub const SESSION_VOL_ENABLED: bool = true;
pub const SESSION_VOL_ASIA_MULT: f64 = 1.5;
pub const SESSION_VOL_LONDON_MULT: f64 = 2.0;
pub const SESSION_VOL_NY_MULT: f64 = 2.5;
pub const SESSION_VOL_EXPAND: f64 = 1.5;    // BB width must exceed expected × this
pub const SESSION_VOL_SL: f64 = 0.005;
pub const SESSION_VOL_TP: f64 = 0.004;
pub const SESSION_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run330_1_session_vol_backtest.py)
2. **Walk-forward** (run330_2_session_vol_wf.py)
3. **Combined** (run330_3_combined.py)

## Out-of-Sample Testing

- ASIA_MULT sweep: 1.0 / 1.5 / 2.0
- LONDON_MULT sweep: 1.5 / 2.0 / 2.5
- NY_MULT sweep: 2.0 / 2.5 / 3.0
- EXPAND sweep: 1.2 / 1.5 / 2.0
