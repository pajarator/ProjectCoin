# RUN362 — Hull Suite Adaptive Trend: Low-Lag Moving Average System

## Hypothesis

**Mechanism**: Hull Suite (Hull MA, Hull Variable MA) uses weighted moving averages with a square-root-of-period smoothing adjustment that significantly reduces lag compared to standard EMAs. HMA(n) = WMA(2*WMA(n/2) - WMA(n), sqrt(n)). The Hull MA crossover with a shorter and longer period gives trend signals with minimal lag. Hull Adaptive adjusts the smoothing based on market speed.

**Why not duplicate**: No prior RUN uses the Hull Suite. Hull MA provides the lowest-lag trend signal among all moving averages while maintaining smoothness. This is distinct from EMA, SMA, or TEMA because the sqrt(n) adjustment creates a fundamentally different smoothing characteristic.

## Proposed Config Changes (config.rs)

```rust
// ── RUN362: Hull Suite Adaptive Trend ────────────────────────────────────────
// hull_ma(n) = WMA(2*WMA(n/2) - WMA(n), sqrt(n))
// hull_fast = hull_ma(FAST_PERIOD)
// hull_slow = hull_ma(SLOW_PERIOD)
// LONG: hull_fast crosses above hull_slow
// SHORT: hull_fast crosses below hull_slow

pub const HULL_ENABLED: bool = true;
pub const HULL_FAST: usize = 9;              // fast Hull MA period
pub const HULL_SLOW: usize = 21;            // slow Hull MA period
pub const HULL_SL: f64 = 0.005;
pub const HULL_TP: f64 = 0.004;
pub const HULL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run362_1_hull_backtest.py)
2. **Walk-forward** (run362_2_hull_wf.py)
3. **Combined** (run362_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 7 / 9 / 12
- SLOW sweep: 16 / 21 / 30
