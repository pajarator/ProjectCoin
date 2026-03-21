# RUN300 — Chaikin Volatility Oscillator: High-Energy Volatility Expansion

## Hypothesis

**Mechanism**: Chaikin Volatility = rate-of-change of ATR over N periods. High volatility expansion (> threshold) indicates explosive moves about to happen — trade in the direction of the volatility surge after a consolidation cooldown. Low volatility contraction followed by rapid expansion = strongest signals. Mean-reversion works because volatility expands then contracts in cycles.

**Why not duplicate**: No prior RUN uses Chaikin Volatility. ATR is used for stops and momentum (RUN26, RUN27, RUN245), but Chaikin measures the *rate of change* of ATR itself, not absolute ATR. This is a volatility-cycle strategy — enters after volatility compresses and explodes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN300: Chaikin Volatility Oscillator ────────────────────────────────────
// chaikin_vol = (ATR(N) - ATR(N-M)) / ATR(N-M)  — rate of change of ATR
// LOW vol state: chaikin_vol < contraction_threshold (consolidation)
// EXPANSION: chaikin_vol crosses above expansion_threshold (volatility breakout)
// LONG: expansion signal + price closes above SMA(20)
// SHORT: expansion signal + price closes below SMA(20)
// Require prior contraction (chaikin_vol < contraction_threshold for at least N bars)

pub const CHAIKIN_VOL_ENABLED: bool = true;
pub const CHAIKIN_VOL_ATR_PERIOD: usize = 10;     // ATR period for vol calc
pub const CHAIKIN_VOL_CHANGE_PERIOD: usize = 5;   // M in ATR(N) - ATR(N-M)
pub const CHAIKIN_VOL_CONTRACTION: f64 = 0.10;   // < 10% = compressed
pub const CHAIKIN_VOL_EXPANSION: f64 = 0.30;     // > 30% = expansion breakout
pub const CHAIKIN_VOL_CONTRACTION_BARS: u32 = 8; // must be contracted this long before entry
pub const CHAIKIN_VOL_SL: f64 = 0.005;
pub const CHAIKIN_VOL_TP: f64 = 0.004;
pub const CHAIKIN_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run300_1_chaikin_vol_backtest.py)
2. **Walk-forward** (run300_2_chaikin_vol_wf.py)
3. **Combined** (run300_3_combined.py)

## Out-of-Sample Testing

- CONTRACTION sweep: 0.05 / 0.10 / 0.15
- EXPANSION sweep: 0.20 / 0.30 / 0.50
- ATR_PERIOD sweep: 7 / 10 / 14
- CONTRACTION_BARS sweep: 4 / 8 / 16
