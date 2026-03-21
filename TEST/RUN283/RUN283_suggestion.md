# RUN283 — EMA-SMA Divergence: Fast vs Slow Average Discord

## Hypothesis

**Mechanism**: When EMA(10) is above SMA(50) → short-term momentum bullish. When EMA(10) crosses above SMA(50) → golden cross bullish signal. When EMA(10) crosses below SMA(50) → death cross bearish signal. The crossover of fast EMA and slow SMA is a momentum shift signal.

**Why not duplicate**: No prior RUN uses EMA-SMA crossover. All prior EMA cross RUNs use two EMAs. EMA-SMA crossover is distinct because SMA has different smoothing properties than EMA.

## Proposed Config Changes (config.rs)

```rust
// ── RUN283: EMA-SMA Crossover ────────────────────────────────────────────
// ema_fast = EMA(close, fast_period)
// sma_slow = SMA(close, slow_period)
// LONG: ema_fast crosses above sma_slow
// SHORT: ema_fast crosses below sma_slow

pub const EMA_SMA_X_ENABLED: bool = true;
pub const EMA_SMA_X_FAST: usize = 10;        // EMA period
pub const EMA_SMA_X_SLOW: usize = 50;        // SMA period
pub const EMA_SMA_X_SL: f64 = 0.005;
pub const EMA_SMA_X_TP: f64 = 0.004;
pub const EMA_SMA_X_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run283_1_ema_sma_x_backtest.py)
2. **Walk-forward** (run283_2_ema_sma_x_wf.py)
3. **Combined** (run283_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 5 / 10 / 15
- SLOW sweep: 30 / 50 / 75
