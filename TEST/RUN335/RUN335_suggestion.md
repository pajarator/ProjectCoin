# RUN335 — Choppiness Index Trend Mode: Regime-Switching by Market Texture

## Hypothesis

**Mechanism**: Choppiness Index (CI) measures market "choppiness" — values near 100 = choppy (no directional bias), values near 0 = trending. CI > 61.8 = market is choppy → use mean-reversion. CI < 38.2 = market is trending → use trend-following. CI between 38.2 and 61.8 = transition zone → no trades. The strategy switches its entry logic based on the CI regime.

**Why not duplicate**: RUN202 uses Choppiness Index as a filter for entries. This RUN uses CI specifically as a regime switch — it changes the ENTRY logic between mean-reversion and trend-following based on CI. The dual-mode regime switching is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN335: Choppiness Index Trend Mode ───────────────────────────────────────
// ci = 100 * LOG(sum(ATR(1), period) / max(HIGH, prior_close) - min(LOW, prior_close)) / LOG(10)
// ci < CI_TRENDING → trending mode (use trend-following: SMA cross)
// ci > CI_CHOPPY → choppy mode (use mean-reversion: RSI extremes)
// ci between CI_TRENDING and CI_CHOPPY → no trades (transition zone)
//
// TRENDING MODE: price crosses above SMA(20) = LONG, below = SHORT
// CHOPPY MODE: RSI < 30 = LONG, RSI > 70 = SHORT

pub const CI_MODE_ENABLED: bool = true;
pub const CI_MODE_PERIOD: usize = 14;
pub const CI_MODE_TRENDING: f64 = 38.2;     // below this = trending
pub const CI_MODE_CHOPPY: f64 = 61.8;       // above this = choppy
pub const CI_MODE_RSI_LONG: f64 = 30.0;
pub const CI_MODE_RSI_SHORT: f64 = 70.0;
pub const CI_MODE_SMA_PERIOD: usize = 20;
pub const CI_MODE_SL: f64 = 0.005;
pub const CI_MODE_TP: f64 = 0.004;
pub const CI_MODE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run335_1_ci_mode_backtest.py)
2. **Walk-forward** (run335_2_ci_mode_wf.py)
3. **Combined** (run335_3_combined.py)

## Out-of-Sample Testing

- TRENDING sweep: 30 / 38.2 / 45
- CHOPPY sweep: 55 / 61.8 / 70
- RSI_LONG sweep: 25 / 30 / 35
- RSI_SHORT sweep: 65 / 70 / 75
