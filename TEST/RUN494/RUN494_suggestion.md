# RUN494 — Volume Profile with Stochastic Extreme

## Hypothesis

**Mechanism**: Volume Profile organizes volume by price level, identifying the Point of Control (POC) and high-volume nodes. Price often finds support/resistance at these levels. Stochastic Extreme detects when Stochastic is at overbought/oversold levels. When price is at a Volume Profile node AND Stochastic confirms extreme momentum, the level has both structural and momentum confirmation.

**Why not duplicate**: RUN395 uses Volume Profile POC with VWAP Trend Alignment. This RUN uses Stochastic Extreme instead — distinct mechanism is using Stochastic as the confirming oscillator versus VWAP as trend alignment. This catches reversal setups at structural volume levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN494: Volume Profile with Stochastic Extreme ─────────────────────────────────
// volume_profile: organizes volume by price level, identifies POC
// price_at_poc: price is at or near the point of control
// stochastic_extreme: stochastic > 80 (overbought) or < 20 (oversold)
// LONG: price near POC support AND stochastic < 20 AND stochastic rising
// SHORT: price near POC resistance AND stochastic > 80 AND stochastic falling

pub const VOLPROF_STOCH_ENABLED: bool = true;
pub const VOLPROF_STOCH_VP_PERIOD: usize = 20;
pub const VOLPROF_STOCH_POC_TOLERANCE: f64 = 0.001;
pub const VOLPROF_STOCH_STOCH_PERIOD: usize = 14;
pub const VOLPROF_STOCH_STOCH_K: usize = 3;
pub const VOLPROF_STOCH_STOCH_D: usize = 3;
pub const VOLPROF_STOCH_STOCH_LOW: f64 = 20.0;
pub const VOLPROF_STOCH_STOCH_HIGH: f64 = 80.0;
pub const VOLPROF_STOCH_SL: f64 = 0.005;
pub const VOLPROF_STOCH_TP: f64 = 0.004;
pub const VOLPROF_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run494_1_volprof_stoch_backtest.py)
2. **Walk-forward** (run494_2_volprof_stoch_wf.py)
3. **Combined** (run494_3_combined.py)

## Out-of-Sample Testing

- VP_PERIOD sweep: 14 / 20 / 30
- POC_TOLERANCE sweep: 0.0005 / 0.001 / 0.002
- STOCH_PERIOD sweep: 10 / 14 / 20
- STOCH_K sweep: 2 / 3 / 5
- STOCH_LOW sweep: 15 / 20 / 25
- STOCH_HIGH sweep: 75 / 80 / 85
