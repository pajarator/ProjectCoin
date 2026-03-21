# RUN336 — Ease of Movement with EMA Slope Confirmation: Effort vs Result

## Hypothesis

**Mechanism**: Ease of Movement = (high - low) / volume adjusted by a scaling factor. High EOM = price moves easily on low volume (effortless moves = trending). Low EOM = price struggles to move even with volume (distribution/acculation). When EOM crosses above its SMA AND EMA is rising → easy upward movement → LONG. When EOM crosses below AND EMA falling → easy downward movement → SHORT.

**Why not duplicate**: RUN195 uses Ease of Movement but as a standalone signal. This RUN combines EOM crossover with EMA slope confirmation — the EMA slope filters out EOM signals that occur against the trend. The distinct mechanism is the EOM + trend slope confluence.

## Proposed Config Changes (config.rs)

```rust
// ── RUN336: Ease of Movement with EMA Slope Confirmation ─────────────────────────
// eom = ((high - low) / volume) * scaling_factor
// eom_ma = SMA(eom, period)
// ema_slope = EMA(close, ema_period) - EMA(close, ema_period)[N]
// LONG: eom crosses above eom_ma AND ema_slope > 0
// SHORT: eom crosses below eom_ma AND ema_slope < 0
// Volume gate: volume > avg_vol * VOL_GATE to confirm genuine EOM signals

pub const EOM_EMA_ENABLED: bool = true;
pub const EOM_EMA_PERIOD: usize = 20;
pub const EOM_EMA_VOL_GATE: f64 = 1.2;     // volume must exceed 1.2x avg
pub const EOM_EMA_EMA_PERIOD: usize = 20;
pub const EOM_EMA_SLOPE_BARS: u32 = 5;
pub const EOM_EMA_SL: f64 = 0.005;
pub const EOM_EMA_TP: f64 = 0.004;
pub const EOM_EMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run336_1_eom_ema_backtest.py)
2. **Walk-forward** (run336_2_eom_ema_wf.py)
3. **Combined** (run336_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- EMA_PERIOD sweep: 15 / 20 / 30
- SLOPE_BARS sweep: 3 / 5 / 8
- VOL_GATE sweep: 1.0 / 1.2 / 1.5
