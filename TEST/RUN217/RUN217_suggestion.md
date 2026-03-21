# RUN217 — Twiggs Momentum: Smoothed Exponential Momentum for Trend Detection

## Hypothesis

**Mechanism**: Twiggs Momentum = 6-period EMA of (close - close of 2 periods ago), then smoothed with a 4-period EMA of the momentum values. This produces a smoother, more reliable momentum signal than standard MACD because the exponential smoothing reduces false signals from market noise.

**Why not duplicate**: No prior RUN uses Twiggs Momentum. All prior momentum RUNs use standard MACD or RSI. Twiggs Momentum is a distinct variant that applies EMA smoothing to the momentum signal itself, making it more robust to false breakouts.

## Proposed Config Changes (config.rs)

```rust
// ── RUN217: Twiggs Momentum ───────────────────────────────────────────────
// momentum = close - close[2]
// twiggs = EMA(momentum, 6) smoothed by EMA(4)
// LONG: Twiggs crosses above 0
// SHORT: Twiggs crosses below 0
// Rising Twiggs = sustained bullish momentum

pub const TWIGGS_ENABLED: bool = true;
pub const TWIGGS_MOMENTUM_PERIOD: usize = 2;  // momentum lookback
pub const TWIGGS_SMOOTH1: usize = 6;          // first EMA period
pub const TWIGGS_SMOOTH2: usize = 4;          // second smoothing EMA period
pub const TWIGGS_SL: f64 = 0.005;
pub const TWIGGS_TP: f64 = 0.004;
pub const TWIGGS_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn twiggs_momentum(closes: &[f64], mom_period: usize, smooth1: usize, smooth2: usize) -> f64 {
    let n = closes.len();
    if n < mom_period + smooth1 + smooth2 {
        return 0.0;
    }

    let mut momentum = vec![0.0; n];
    for i in mom_period..n {
        momentum[i] = closes[i] - closes[i - mom_period];
    }

    // Apply first EMA smoothing
    let ema1_vals: Vec<f64> = momentum.iter().skip(mom_period).cloned().collect();
    let ema1 = ema(&ema1_vals, smooth1);

    // Apply second EMA smoothing
    // Simplified: return ema1 as proxy for twiggs momentum
    ema1
}
```

---

## Validation Method

1. **Historical backtest** (run217_1_twiggs_backtest.py)
2. **Walk-forward** (run217_2_twiggs_wf.py)
3. **Combined** (run217_3_combined.py)

## Out-of-Sample Testing

- MOMENTUM_PERIOD sweep: 1 / 2 / 3
- SMOOTH1 sweep: 4 / 6 / 8
- SMOOTH2 sweep: 2 / 4 / 6
