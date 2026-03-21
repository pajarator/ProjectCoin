# RUN323 — Vortex Indicator Trend Extraction: VI+ and VI- Bifurcation

## Hypothesis

**Mechanism**: Vortex Indicator separates upward price movement (VI+) from downward movement (VI-). VI+ rising above VI- → bullish trend. VI- rising above VI+ → bearish trend. The crossover of VI+ and VI- is the signal. The distance between VI+ and VI- indicates trend strength. When VI+ and VI- are both rising but converging → trend losing strength.

**Why not duplicate**: RUN198 uses Vortex Indicator as a standalone indicator. RUN119 uses VI confirmation for entries. This RUN uses VI specifically for trend extraction: the distance between VI+ and VI- as a strength measure, and convergence/divergence patterns for early reversal detection. The key distinction is using VI as a trend-vs-exhaustion detector rather than a crossover signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN323: Vortex Indicator Trend Extraction ──────────────────────────────────
// vi_plus = |high - close[1]| / |high - low| (averaged over period)
// vi_minus = |low - close[1]| / |high - low| (averaged over period)
// trend_up = vi_plus > vi_minus AND vi_plus rising
// trend_down = vi_minus > vi_plus AND vi_minus rising
// LONG: vi_plus crosses above vi_minus AND distance > DIST_THRESH
// SHORT: vi_minus crosses above vi_plus AND distance > DIST_THRESH
// Exit: distance collapses below DIST_EXIT

pub const VI_TREND_ENABLED: bool = true;
pub const VI_TREND_PERIOD: usize = 14;
pub const VI_TREND_DIST_THRESH: f64 = 0.1;   // min VI+ - VI- for valid signal
pub const VI_TREND_DIST_EXIT: f64 = 0.02;    // collapse threshold for exit
pub const VI_TREND_SL: f64 = 0.005;
pub const VI_TREND_TP: f64 = 0.004;
pub const VI_TREND_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run323_1_vi_trend_backtest.py)
2. **Walk-forward** (run323_2_vi_trend_wf.py)
3. **Combined** (run323_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- DIST_THRESH sweep: 0.05 / 0.1 / 0.15
- DIST_EXIT sweep: 0.0 / 0.02 / 0.05
