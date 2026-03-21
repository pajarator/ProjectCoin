# RUN453 — Elder Ray Index with Volume Confirmation

## Hypothesis

**Mechanism**: Elder Ray Index measures bull power and bear power by comparing the high/low to an EMA. Bull power measures how far the high exceeds the EMA; bear power measures how far the low is below. Strong bull/bear power readings indicate trending markets. Volume Confirmation adds institutional backing: when Elder Ray shows strong power readings AND volume is above average, the trend has conviction.

**Why not duplicate**: RUN319 uses Elder Ray with EMA. RUN357 uses Elder Ray with ADX Filter. RUN391 uses Elder Ray with CMO. This RUN specifically uses Volume Confirmation — the distinct mechanism is requiring volume above average to confirm Elder Ray power signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN453: Elder Ray Index with Volume Confirmation ─────────────────────────────────
// elder_ray_bull = high - EMA(close, period)
// elder_ray_bear = low - EMA(close, period)
// strong_power: |elder_ray| > POWER_THRESH (strong trending)
// volume_confirmation: volume > SMA(volume, period) * VOL_MULT
// LONG: elder_ray_bull > POWER_THRESH AND volume confirmation
// SHORT: elder_ray_bear < -POWER_THRESH AND volume confirmation

pub const ELDER_VOL_ENABLED: bool = true;
pub const ELDER_VOL_ELDER_PERIOD: usize = 13;
pub const ELDER_VOL_POWER_THRESH: f64 = 0.001;
pub const ELDER_VOL_VOL_PERIOD: usize = 20;
pub const ELDER_VOL_VOL_MULT: f64 = 1.2;
pub const ELDER_VOL_SL: f64 = 0.005;
pub const ELDER_VOL_TP: f64 = 0.004;
pub const ELDER_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run453_1_elder_vol_backtest.py)
2. **Walk-forward** (run453_2_elder_vol_wf.py)
3. **Combined** (run453_3_combined.py)

## Out-of-Sample Testing

- ELDER_PERIOD sweep: 10 / 13 / 20
- POWER_THRESH sweep: 0.0005 / 0.001 / 0.002
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
