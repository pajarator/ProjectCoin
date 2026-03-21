# RUN210 — Gator Oscillator: Alligator Trend Phase Filter

## Hypothesis

**Mechanism**: The Gator Oscillator visualizes the Alligator indicator's three SMAs (jaw, teeth, lips) in histogram form. It shows when the Alligator is "sleeping" (bars are red/both sides near zero = no trend) vs "awakening" (bars are green on one side = trend forming). Only take momentum signals when the Gator is "awake" (bars transitioning from red to green).

**Why not duplicate**: No prior RUN uses the Gator Oscillator or Alligator system. All prior market phase detection RUNs use ADX or Choppiness Index. The Gator is unique because it specifically identifies the transition from sleep to wake phases of the market.

## Proposed Config Changes (config.rs)

```rust
// ── RUN210: Gator Oscillator Trend Filter ────────────────────────────────
// jaw = SMA(close, 13), teeth = SMA(close, 8), lips = SMA(close, 5)
// gator_upper = |jaw - teeth|
// gator_lower = |teeth - lips|
// Histogram bars above/below zero
// GREEN bars = trending (awake), RED bars = sleeping
// Wait for GREEN bars after RED → trend starting → enter in trend direction

pub const GATOR_ENABLED: bool = true;
pub const GATOR_JAW_PERIOD: usize = 13;     // jaw (slow) period
pub const GATOR_TEETH_PERIOD: usize = 8;     // teeth (medium) period
pub const GATOR_LIPS_PERIOD: usize = 5;      // lips (fast) period
pub const GATOR_CONFIRM_BARS: u32 = 2;       // consecutive green bars needed
pub const GATOR_SL: f64 = 0.005;
pub const GATOR_TP: f64 = 0.004;
pub const GATOR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run210_1_gator_backtest.py)
2. **Walk-forward** (run210_2_gator_wf.py)
3. **Combined** (run210_3_combined.py)

## Out-of-Sample Testing

- JAW_PERIOD sweep: 10 / 13 / 20
- TEETH_PERIOD sweep: 5 / 8 / 12
- LIPS_PERIOD sweep: 3 / 5 / 8
- CONFIRM_BARS sweep: 1 / 2 / 3
