# RUN276 — RSI Double EMA Crossover: Smoothed Oscillator Trend Lines

## Hypothesis

**Mechanism**: Apply two EMAs to RSI values (fast EMA + slow EMA). When the fast EMA crosses above the slow EMA → bullish momentum building. When fast crosses below slow → bearish momentum. This is equivalent to applying an EMA crossover system to the oscillator itself, reducing noise and providing earlier signals than RSI threshold crossings.

**Why not duplicate**: No prior RUN applies EMA crossover to RSI. All prior RSI RUNs use absolute thresholds or RSI divergence. EMA crossover on RSI is distinct because it treats the oscillator like price and applies trend-following logic to it.

## Proposed Config Changes (config.rs)

```rust
// ── RUN276: RSI Double EMA Crossover ─────────────────────────────────────
// rsi_fast = EMA(RSI(close, 14), fast_period)
// rsi_slow = EMA(RSI(close, 14), slow_period)
// LONG: rsi_fast crosses above rsi_slow
// SHORT: rsi_fast crosses below rsi_slow

pub const RSI_EMA_X_ENABLED: bool = true;
pub const RSI_EMA_X_RSI_PERIOD: usize = 14;
pub const RSI_EMA_X_FAST: usize = 5;         // fast EMA period
pub const RSI_EMA_X_SLOW: usize = 14;         // slow EMA period
pub const RSI_EMA_X_SL: f64 = 0.005;
pub const RSI_EMA_X_TP: f64 = 0.004;
pub const RSI_EMA_X_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run276_1_rsi_ema_x_backtest.py)
2. **Walk-forward** (run276_2_rsi_ema_x_wf.py)
3. **Combined** (run276_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- FAST sweep: 3 / 5 / 7
- SLOW sweep: 10 / 14 / 21
