# RUN428 — Hull Suite Adaptive Trend with RSI Extreme Filter

## Hypothesis

**Mechanism**: The Hull Suite is an adaptive trend indicator that uses weighted moving averages with adaptive coefficients, providing smoother and more responsive trend signals than traditional MAs. It changes its smoothing based on market conditions. RSI Extreme Filter adds timing precision: when Hull Suite fires a trend change signal AND RSI is in extreme territory (oversold <30 or overbought >70), the trend change has both adaptive MA direction AND oscillator timing confirmation.

**Why not duplicate**: RUN362 uses Hull Suite Adaptive Trend standalone. This RUN adds RSI Extreme Filter — the distinct mechanism is using RSI extremes to confirm Hull Suite trend changes, filtering out signals when RSI is in neutral territory.

## Proposed Config Changes (config.rs)

```rust
// ── RUN428: Hull Suite Adaptive Trend with RSI Extreme Filter ─────────────────────────────────
// hull_suite: adaptive MA with variable smoothing based on trend strength
// hull_flip: hull changes direction (bullish to bearish or vice versa)
// rsi_extreme: rsi < RSI_OVERSOLD or rsi > RSI_OVERBOUGHT
// LONG: hull_flip bullish AND rsi < RSI_OVERSOLD
// SHORT: hull_flip bearish AND rsi > RSI_OVERBOUGHT

pub const HULL_RSI_ENABLED: bool = true;
pub const HULL_RSI_HULL_PERIOD: usize = 20;
pub const HULL_RSI_ADAPT_COEFF: f64 = 0.5;   // adaptation coefficient
pub const HULL_RSI_RSI_PERIOD: usize = 14;
pub const HULL_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const HULL_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const HULL_RSI_SL: f64 = 0.005;
pub const HULL_RSI_TP: f64 = 0.004;
pub const HULL_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run428_1_hull_rsi_backtest.py)
2. **Walk-forward** (run428_2_hull_rsi_wf.py)
3. **Combined** (run428_3_combined.py)

## Out-of-Sample Testing

- HULL_PERIOD sweep: 14 / 20 / 30
- ADAPT_COEFF sweep: 0.3 / 0.5 / 0.7
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
