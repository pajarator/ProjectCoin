# RUN364 — Volume-Weighted EMA Crossover with Trend Confirmation

## Hypothesis

**Mechanism**: Standard EMA crossover ignores volume. Volume-weighted EMA (VW-EMA) weights each price point by its volume — high-volume bars have more influence on the EMA. This creates an EMA that more accurately reflects where institutional money actually traded. Combine with trend confirmation: only take VW-EMA crossover signals when ADX confirms the market is trending in the same direction.

**Why not duplicate**: No prior RUN uses volume-weighted EMA crossover. This is distinct from standard EMA crossover (RUN283, RUN230) because volume weighting changes when the EMA actually crosses, making it more responsive to genuine institutional moves.

## Proposed Config Changes (config.rs)

```rust
// ── RUN364: Volume-Weighted EMA Crossover with Trend Confirmation ───────────────────────────
// vw_ema(n) = EMA(close * volume, n) / EMA(volume, n)
// vw_ema_fast = vw_ema(FAST_PERIOD)
// vw_ema_slow = vw_ema(SLOW_PERIOD)
// adx_strong = ADX(ADX_PERIOD) > ADX_MIN
// LONG: vw_ema_fast crosses above vw_ema_slow AND adx_strong AND +DI > -DI
// SHORT: vw_ema_fast crosses below vw_ema_slow AND adx_strong AND -DI > +DI

pub const VWEMA_X_ENABLED: bool = true;
pub const VWEMA_X_FAST: usize = 9;
pub const VWEMA_X_SLOW: usize = 21;
pub const VWEMA_X_ADX_PERIOD: usize = 14;
pub const VWEMA_X_ADX_MIN: f64 = 20.0;
pub const VWEMA_X_SL: f64 = 0.005;
pub const VWEMA_X_TP: f64 = 0.004;
pub const VWEMA_X_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run364_1_vwema_x_backtest.py)
2. **Walk-forward** (run364_2_vwema_x_wf.py)
3. **Combined** (run364_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 7 / 9 / 12
- SLOW sweep: 16 / 21 / 30
- ADX_MIN sweep: 15 / 20 / 25
