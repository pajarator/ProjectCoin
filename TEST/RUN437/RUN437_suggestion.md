# RUN437 — MACD Double Signal Line Crossover with Volume Confirmation

## Hypothesis

**Mechanism**: MACD Double Signal Line Crossover uses two different signal line periods (e.g., 9 and 17) to create a dual confirmation system. The shorter signal line is more responsive; the longer is more accurate. A trade only fires when both signal lines agree on direction (both bullish or both bearish). Volume Confirmation adds institutional backing: when both signal lines cross AND volume is above its moving average, the signal has dual smoothing AND volume-backed conviction.

**Why not duplicate**: RUN373 uses MACD Double Signal Line Crossover standalone. This RUN adds Volume Confirmation — the distinct mechanism is using volume above MA to filter out MACD double signal crossovers that lack institutional backing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN437: MACD Double Signal Line Crossover with Volume Confirmation ─────────────────────────────────────
// macd_line = EMA(close, fast) - EMA(close, slow)
// signal_fast = EMA(macd_line, signal_fast_period)
// signal_slow = EMA(macd_line, signal_slow_period)
// double_cross: both signals cross macd_line in same direction
// volume_confirmation: volume > SMA(volume, period)
// LONG: double_cross bullish AND volume > vol_sma
// SHORT: double_cross bearish AND volume > vol_sma

pub const MACD_DBL_VOL_ENABLED: bool = true;
pub const MACD_DBL_VOL_FAST_PERIOD: usize = 12;
pub const MACD_DBL_VOL_SLOW_PERIOD: usize = 26;
pub const MACD_DBL_VOL_SIGNAL_FAST: usize = 9;
pub const MACD_DBL_VOL_SIGNAL_SLOW: usize = 17;
pub const MACD_DBL_VOL_VOL_PERIOD: usize = 20;
pub const MACD_DBL_VOL_VOL_MULT: f64 = 1.2;
pub const MACD_DBL_VOL_SL: f64 = 0.005;
pub const MACD_DBL_VOL_TP: f64 = 0.004;
pub const MACD_DBL_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run437_1_macd_dbl_vol_backtest.py)
2. **Walk-forward** (run437_2_macd_dbl_vol_wf.py)
3. **Combined** (run437_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 8 / 12 / 16
- SLOW_PERIOD sweep: 21 / 26 / 30
- SIGNAL_FAST sweep: 7 / 9 / 12
- SIGNAL_SLOW sweep: 14 / 17 / 21
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
