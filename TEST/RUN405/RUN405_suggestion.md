# RUN405 — Relative Vigor Index with Choppiness Index Trend Filter

## Hypothesis

**Mechanism**: The Relative Vigor Index (RVI) measures how bullish or bearish the current price action is by comparing the closing price to the trading range. It oscillates around a centerline. Like other oscillators, it can produce false signals in choppy markets. Add Choppiness Index (CI) as a regime filter: CI > 60 means the market is choppy (range-bound); CI < 40 means it's trending. Only take RVI crossover signals when CI < 50 (trending market), ensuring signals only fire when the market has directional conviction.

**Why not duplicate**: RUN312 uses Relative Vigor Index standalone. RUN335 uses Choppiness Index Trend Mode. This RUN specifically combines RVI crossovers with CI as a market regime filter — the distinct mechanism is using CI to gate RVI entries, preventing signals in choppy markets where RVI fails.

## Proposed Config Changes (config.rs)

```rust
// ── RUN405: Relative Vigor Index with Choppiness Index Trend Filter ───────────────────────────
// rvi = (close - open) / (high - low) smoothed by SMA
// rvi_signal = SMA(rvi, signal_period)
// rvi_cross: rvi crosses above/below signal line
// choppiness_index = 100 * log10(sum(ATR, period) / (max(high) - min(low))) / log10(period)
// trending: ci < CI_THRESH (below = trending, above = choppy)
// LONG: rvi crosses bullish AND ci < CI_THRESH
// SHORT: rvi crosses bearish AND ci < CI_THRESH

pub const RVI_CI_ENABLED: bool = true;
pub const RVI_CI_RVI_PERIOD: usize = 10;
pub const RVI_CI_SIGNAL_PERIOD: usize = 4;
pub const RVI_CI_CI_PERIOD: usize = 14;
pub const RVI_CI_CI_THRESH: f64 = 50.0;   // below = trending
pub const RVI_CI_SL: f64 = 0.005;
pub const RVI_CI_TP: f64 = 0.004;
pub const RVI_CI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run405_1_rvi_ci_backtest.py)
2. **Walk-forward** (run405_2_rvi_ci_wf.py)
3. **Combined** (run405_3_combined.py)

## Out-of-Sample Testing

- RVI_PERIOD sweep: 7 / 10 / 14
- SIGNAL_PERIOD sweep: 3 / 4 / 5
- CI_PERIOD sweep: 10 / 14 / 21
- CI_THRESH sweep: 40 / 50 / 60
