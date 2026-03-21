# RUN420 — Aroon Oscillator with Volume Confirmation

## Hypothesis

**Mechanism**: The Aroon Oscillator measures the time since the last high or low within a lookback period. It oscillates between -100 and +100, with positive values indicating bullish趋势 (more recent highs) and negative values indicating看跌趋势 (more recent lows). Volume Confirmation adds conviction check: when Aroon flips direction (crosses above/below zero) AND volume is above its moving average, the trend change has institutional participation backing it.

**Why not duplicate**: RUN317 uses Aroon Oscillator Range Filter. This RUN uses Aroon Oscillator with Volume Confirmation — the distinct mechanism is using volume above its MA as the confirmation filter for Aroon directional flips, rather than Aroon range filtering or any other confirmation method.

## Proposed Config Changes (config.rs)

```rust
// ── RUN420: Aroon Oscillator with Volume Confirmation ─────────────────────────────────
// aroon_oscillator = aroon_up - aroon_down
// aroon_flip: oscillator crosses above/below 0
// volume_confirmation: volume > SMA(volume, period)
// LONG: aroon_oscillator crosses above 0 AND volume > vol_sma
// SHORT: aroon_oscillator crosses below 0 AND volume > vol_sma

pub const AROON_VOL_ENABLED: bool = true;
pub const AROON_VOL_AROON_PERIOD: usize = 25;
pub const AROON_VOL_VOL_PERIOD: usize = 20;
pub const AROON_VOL_VOL_MULT: f64 = 1.2;   // volume must be above this * sma
pub const AROON_VOL_SL: f64 = 0.005;
pub const AROON_VOL_TP: f64 = 0.004;
pub const AROON_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run420_1_aroon_vol_backtest.py)
2. **Walk-forward** (run420_2_aroon_vol_wf.py)
3. **Combined** (run420_3_combined.py)

## Out-of-Sample Testing

- AROON_PERIOD sweep: 20 / 25 / 30
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
