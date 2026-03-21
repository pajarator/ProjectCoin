# RUN307 — Chande Momentum Oscillator: Raw Momentum Without Smoothing

## Hypothesis

**Mechanism**: CMO = ((Sum_up - Sum_down) / (Sum_up + Sum_down)) * 100, measured over N periods. Unlike RSI (which uses an EMA smoothing layer), CMO is raw momentum — it doesn't smooth out the measurement. This makes CMO more responsive but also noisier. CMO crossing above +50 = strong bullish momentum. CMO crossing below -50 = strong bearish momentum. Crosses near 0 = weak signals filtered out.

**Why not duplicate**: No prior RUN uses CMO. RSI variants are covered (RSI EMA crossover RUN276, RSI percentile RUN262, RSI divergence RUN215, RSI gap RUN287, RSI extreme RUN290, RSI volume divergence RUN247, RSI breadth RUN253). CMO is distinct because it's the only unsmoothed momentum oscillator — it captures raw directional energy without the EMA delay.

## Proposed Config Changes (config.rs)

```rust
// ── RUN307: Chande Momentum Oscillator ──────────────────────────────────────
// cmo(n) = 100 * (sum_up - sum_down) / (sum_up + sum_down)
// up = max(close - close[1], 0)
// down = max(close[1] - close, 0)
// LONG: cmo crosses above +50 (strong bullish momentum)
// SHORT: cmo crosses below -50 (strong bearish momentum)
// Hold until cmo crosses back through 0 (momentum exhaustion)

pub const CMO_ENABLED: bool = true;
pub const CMO_PERIOD: usize = 14;
pub const CMO_LONG_THRESH: f64 = 50.0;
pub const CMO_SHORT_THRESH: f64 = -50.0;
pub const CMO_EXIT_ZERO_CROSS: bool = true;   // exit on zero cross vs threshold reversal
pub const CMO_SL: f64 = 0.005;
pub const CMO_TP: f64 = 0.004;
pub const CMO_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run307_1_cmo_backtest.py)
2. **Walk-forward** (run307_2_cmo_wf.py)
3. **Combined** (run307_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- LONG_THRESH sweep: 40 / 50 / 60
- SHORT_THRESH sweep: -60 / -50 / -40
