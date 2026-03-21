# RUN396 — Fisher Transform with ADX Trend Strength Filter

## Hypothesis

**Mechanism**: The Fisher Transform converts price data into a Gaussian normal distribution, making turning points clearer and easier to identify. It oscillates above/below zero with sharp crossovers. However, like all oscillators, it can produce false signals in choppy markets. Pair it with ADX (Average Directional Index): ADX measures trend strength regardless of direction. Only take Fisher Transform signals when ADX is above a threshold (strong trend), ensuring the signal occurs in a trending environment where mean reversion strategies work best.

**Why not duplicate**: RUN313 uses Fisher Transform standalone. RUN328 uses ADX Momentum Angle. RUN361 uses MACD Histogram Slope with RSI Filter. This RUN specifically uses Fisher Transform (distinct normalization mechanism) with ADX as a trend strength gate — the distinct mechanism is using ADX to filter Fisher signals to only trending markets.

## Proposed Config Changes (config.rs)

```rust
// ── RUN396: Fisher Transform with ADX Trend Strength Filter ─────────────────────────────
// fisher_transform = 0.5 * ln((1 + x) / (1 - x)) where x = 2 * (price - lowest_low) / (highest_high - lowest_low) - 1
// fisher_cross: fisher crosses above/below signal line
// adx = ADX(close, period) measuring trend strength
// trend_confirmed: adx > ADX_THRESH (strong trend present)
// LONG: fisher bullish cross AND adx > ADX_THRESH
// SHORT: fisher bearish cross AND adx > ADX_THRESH

pub const FISHER_ADX_ENABLED: bool = true;
pub const FISHER_ADX_FISHER_PERIOD: usize = 10;
pub const FISHER_ADX_SIGNAL_PERIOD: usize = 5;
pub const FISHER_ADX_ADX_PERIOD: usize = 14;
pub const FISHER_ADX_ADX_THRESH: f64 = 25.0;   // above this = strong trend
pub const FISHER_ADX_SL: f64 = 0.005;
pub const FISHER_ADX_TP: f64 = 0.004;
pub const FISHER_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run396_1_fisher_adx_backtest.py)
2. **Walk-forward** (run396_2_fisher_adx_wf.py)
3. **Combined** (run396_3_combined.py)

## Out-of-Sample Testing

- FISHER_PERIOD sweep: 7 / 10 / 14
- SIGNAL_PERIOD sweep: 3 / 5 / 7
- ADX_PERIOD sweep: 10 / 14 / 21
- ADX_THRESH sweep: 20 / 25 / 30
