# RUN340 — Parabolic SAR with Volume Acceleration Filter

## Hypothesis

**Mechanism**: Parabolic SAR (Stop and Reverse) is a time/price reversal system — the SAR acts as a trailing stop that flips from below to above price when a trend reverses. Combine with volume acceleration: only trust SAR flips when volume is expanding (confirming the move has institutional support). SAR flip without volume = false move. SAR flip with volume surge = high-probability reversal.

**Why not duplicate**: RUN192 uses Parabolic SAR but as a standalone entry signal. This RUN specifically combines SAR flip with volume acceleration — volume is the confirmation filter for the SAR signal. The distinct mechanism is volume-confirmed SAR reversals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN340: Parabolic SAR with Volume Acceleration Filter ─────────────────────────
// sar = parabolic_sar(acceleration, maximum)
// LONG: sar flips below price AND volume > avg_vol * VOL_MULT (volume confirms reversal)
// SHORT: sar flips above price AND volume > avg_vol * VOL_MULT (volume confirms reversal)
// Acceleration step: SAR tracking speed (default 0.02)
// No volume surge = filter out the signal (no trade)

pub const SAR_VOL_ENABLED: bool = true;
pub const SAR_VOL_ACCEL: f64 = 0.02;
pub const SAR_VOL_MAX: f64 = 0.2;
pub const SAR_VOL_VOL_MULT: f64 = 1.5;     // volume must exceed 1.5x avg
pub const SAR_VOL_SL: f64 = 0.005;
pub const SAR_VOL_TP: f64 = 0.004;
pub const SAR_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run340_1_sar_vol_backtest.py)
2. **Walk-forward** (run340_2_sar_vol_wf.py)
3. **Combined** (run340_3_combined.py)

## Out-of-Sample Testing

- ACCEL sweep: 0.01 / 0.02 / 0.03
- MAX sweep: 0.15 / 0.2 / 0.25
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
