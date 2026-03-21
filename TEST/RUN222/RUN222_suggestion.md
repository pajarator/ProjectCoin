# RUN222 — Detrended Price Oscillator (DPO): Trend-Removed Price Cycles

## Hypothesis

**Mechanism**: DPO = close - SMA(close, period/2+1). It removes the trend component by comparing price to a shifted SMA. The result shows only the cyclic component — oscillations above zero = above-trend, below = below-trend. When DPO crosses above zero → bullish cycle. When DPO crosses below zero → bearish cycle. Used for mean-reversion: buy when DPO is deeply negative.

**Why not duplicate**: No prior RUN uses DPO. All prior oscillators include the trend component (MACD, RSI). DPO is unique because it explicitly removes the trend, making it a pure cycle indicator — better for mean-reversion strategies.

## Proposed Config Changes (config.rs)

```rust
// ── RUN222: Detrended Price Oscillator (DPO) ──────────────────────────────
// dpo = close - SMA(close, period/2 + 1)
// period_shift = period/2 + 1 (shifts SMA to center of lookback)
// LONG: DPO crosses above 0 from below AND DPO < -X (oversold)
// SHORT: DPO crosses below 0 from above AND DPO > X (overbought)

pub const DPO_ENABLED: bool = true;
pub const DPO_PERIOD: usize = 20;             // lookback period
pub const DPO_OVERSOLD: f64 = -1.0;         // oversold threshold (in price units)
pub const DPO_OVERBOUGHT: f64 = 1.0;        // overbought threshold
pub const DPO_SL: f64 = 0.005;
pub const DPO_TP: f64 = 0.004;
pub const DPO_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn dpo(closes: &[f64], period: usize) -> f64 {
    let n = closes.len();
    if n < period {
        return 0.0;
    }

    let shift = period / 2 + 1;
    let mut sum = 0.0;
    let start = n - period;
    for i in start..n {
        sum += closes[i];
    }
    let sma = sum / (period as f64);

    if n < shift {
        return 0.0;
    }
    let shifted_sma = if n >= period / 2 + 1 {
        let idx = n - shift;
        if idx > 0 { closes[idx] } else { sma }
    } else {
        sma
    };

    closes[n-1] - shifted_sma
}
```

---

## Validation Method

1. **Historical backtest** (run222_1_dpo_backtest.py)
2. **Walk-forward** (run222_2_dpo_wf.py)
3. **Combined** (run222_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30 / 50
