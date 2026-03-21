# RUN214 — ATR Channel Breakout with Average True Range Trailing Stop

## Hypothesis

**Mechanism**: ATR Channel = EMA ± ATR × multiplier (similar to Keltner but using the EMA as the center instead of HL2). When price closes above the upper ATR channel → bullish momentum → LONG with ATR-based trailing stop that follows the lower band. The ATR multiplier controls sensitivity to volatility.

**Why not duplicate**: No prior RUN uses ATR Channel specifically as an entry mechanism. RUN26 tested ATR-based stops but not the ATR channel as an entry. RUN188 uses Keltner (HL2-based). ATR Channel is distinct because ATR-based channels adapt faster to volatility spikes than stddev-based bands.

## Proposed Config Changes (config.rs)

```rust
// ── RUN214: ATR Channel Breakout ──────────────────────────────────────────
// atr_channel_upper = EMA(close, period) + ATR(atr_period) × upper_mult
// atr_channel_lower = EMA(close, period) - ATR(atr_period) × lower_mult
// LONG: close crosses above upper channel → momentum breakout
// Trailing stop: follows atr_channel_lower as price rises

pub const ATR_CHAN_ENABLED: bool = true;
pub const ATR_CHAN_EMA_PERIOD: usize = 20;  // EMA center period
pub const ATR_CHAN_ATR_PERIOD: usize = 14;   // ATR period
pub const ATR_CHAN_UPPER_MULT: f64 = 2.0;   // upper band multiplier
pub const ATR_CHAN_LOWER_MULT: f64 = 2.0;    // lower band multiplier (also trailing stop offset)
pub const ATR_CHAN_CONFIRM_BARS: u32 = 1;    // bars to confirm breakout
pub const ATR_CHAN_SL_ATR: f64 = 1.5;        // stop = ATR × this below entry
pub const ATR_CHAN_TP: f64 = 0.004;
pub const ATR_CHAN_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run214_1_atr_chan_backtest.py)
2. **Walk-forward** (run214_2_atr_chan_wf.py)
3. **Combined** (run214_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 30
- ATR_PERIOD sweep: 10 / 14 / 20
- UPPER_MULT sweep: 1.5 / 2.0 / 2.5
- LOWER_MULT sweep: 1.5 / 2.0 / 2.5
