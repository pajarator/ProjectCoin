# RUN455 — SuperTrend with Volume Confirmation

## Hypothesis

**Mechanism**: SuperTrend is an ATR-based trend indicator that provides clear entry points and trailing stops. It's simple and effective but can whipsaw in choppy markets. Volume Confirmation adds institutional backing: when SuperTrend flips direction AND volume is above its moving average, the trend change has volume-backed conviction and is less likely to be a false signal.

**Why not duplicate**: RUN344 uses SuperTrend with MACD Histogram Divergence. This RUN uses Volume Confirmation instead of MACD Divergence — the distinct mechanism is using volume above MA to confirm SuperTrend flips, filtering out flips that lack institutional participation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN455: SuperTrend with Volume Confirmation ─────────────────────────────────
// supertrend: ATR-based trend direction and trailing stop
// supertrend_flip: trend changes from bullish to bearish or vice versa
// volume_confirmation: volume > SMA(volume, period)
// LONG: supertrend_flip bullish AND volume > vol_sma
// SHORT: supertrend_flip bearish AND volume > vol_sma

pub const ST_VOL_ENABLED: bool = true;
pub const ST_VOL_ST_PERIOD: usize = 10;
pub const ST_VOL_ST_MULT: f64 = 3.0;
pub const ST_VOL_VOL_PERIOD: usize = 20;
pub const ST_VOL_VOL_MULT: f64 = 1.2;
pub const ST_VOL_SL: f64 = 0.005;
pub const ST_VOL_TP: f64 = 0.004;
pub const ST_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run455_1_st_vol_backtest.py)
2. **Walk-forward** (run455_2_st_vol_wf.py)
3. **Combined** (run455_3_combined.py)

## Out-of-Sample Testing

- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
