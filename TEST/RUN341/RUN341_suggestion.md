# RUN341 — Price Ladder with RSI Band Oscillator: Multi-Dimensional Entry System

## Hypothesis

**Mechanism**: Combine price ladder acceptance (RUN310) with RSI band zones. The ladder gives directional bias (which levels are being accepted), RSI gives the timing (when RSI leaves extreme zone). LONG when: price has accepted level N (momentum up) AND RSI crosses above 40 from oversold. SHORT when: price has accepted level below (momentum down) AND RSI crosses below 60 from overbought. The combination filters out weak ladder signals when RSI is neutral.

**Why not duplicate**: RUN310 uses price ladder acceptance alone. This RUN adds RSI timing — the ladder gives direction, RSI band zones give entry timing. The distinct mechanism is the 2-dimensional signal: ladder level × RSI zone = entry.

## Proposed Config Changes (config.rs)

```rust
// ── RUN341: Price Ladder with RSI Band Oscillator ────────────────────────────────
// ladder_accept(levels) = N consecutive closes above/below level
// rsi_band_cross: RSI crosses above 40 (from oversold) or below 60 (from overbought)
// LONG: ladder_accept(level[N]) AND RSI crosses above RSI_ENTER_LONG
// SHORT: ladder_reject(level[N]) AND RSI crosses below RSI_ENTER_SHORT

pub const LADDER_RSI_ENABLED: bool = true;
pub const LADDER_RSI_LADDER_STEP: f64 = 0.005;
pub const LADDER_RSI_ACCEPTANCE: u32 = 2;
pub const LADDER_RSI_RSI_ENTER_LONG: f64 = 40.0;
pub const LADDER_RSI_RSI_ENTER_SHORT: f64 = 60.0;
pub const LADDER_RSI_RSI_PERIOD: usize = 14;
pub const LADDER_RSI_SL: f64 = 0.005;
pub const LADDER_RSI_TP: f64 = 0.004;
pub const LADDER_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run341_1_ladder_rsi_backtest.py)
2. **Walk-forward** (run341_2_ladder_rsi_wf.py)
3. **Combined** (run341_3_combined.py)

## Out-of-Sample Testing

- LADDER_STEP sweep: 0.003 / 0.005 / 0.008
- RSI_ENTER_LONG sweep: 35 / 40 / 45
- RSI_ENTER_SHORT sweep: 55 / 60 / 65
- ACCEPTANCE sweep: 1 / 2 / 3
