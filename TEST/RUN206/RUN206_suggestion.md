# RUN206 — Price Channel Breakout with Volume Confirmation

## Hypothesis

**Mechanism**: Price Channel = highest high and lowest low over N periods (similar to Donchian but with volume confirmation). When price closes above the upper channel AND volume > 20-period volume MA → confirmed bullish breakout → LONG. When price closes below the lower channel AND volume > 20-period volume MA → confirmed bearish breakout → SHORT. Volume filter eliminates false breakouts.

**Why not duplicate**: No prior RUN combines price channel breakout with volume confirmation. RUN189 uses Donchian without volume filter. Adding volume confirmation fundamentally changes the signal quality.

## Proposed Config Changes (config.rs)

```rust
// ── RUN206: Price Channel Breakout + Volume Confirmation ─────────────────
// channel_upper = highest high over period
// channel_lower = lowest low over period
// vol_ma = SMA(volume, vol_ma_period)
// LONG: close > channel_upper AND volume > vol_ma × vol_mult
// SHORT: close < channel_lower AND volume > vol_ma × vol_mult
// Channel middle = retest exit point

pub const PC_ENABLED: bool = true;
pub const PC_PERIOD: usize = 20;             // channel lookback
pub const PC_VOL_MA: usize = 20;            // volume MA period
pub const PC_VOL_MULT: f64 = 1.5;           // volume must exceed MA × this
pub const PC_SL: f64 = 0.005;
pub const PC_TP: f64 = 0.004;
pub const PC_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run206_1_pc_backtest.py)
2. **Walk-forward** (run206_2_pc_wf.py)
3. **Combined** (run206_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 40
- VOL_MA sweep: 10 / 20 / 30
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
