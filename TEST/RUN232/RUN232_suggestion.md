# RUN232 — Linear Regression Slope: Statistical Trend Steepness

## Hypothesis

**Mechanism**: Linear Regression Slope = rate of change of the linear regression line over N periods. It measures the *average* price change per bar, accounting for all noise. A slope of 0.001 means price is rising at 0.1% per bar on average. When slope crosses above threshold → trend up → LONG. When slope crosses below negative threshold → trend down → SHORT.

**Why not duplicate**: No prior RUN uses Linear Regression. All prior trend RUNs use EMA crosses or momentum oscillators. Linear Regression is unique because it measures the *average rate of change* across the entire lookback — a statistically principled trend measure.

## Proposed Config Changes (config.rs)

```rust
// ── RUN232: Linear Regression Slope ─────────────────────────────────────
// lr_slope = slope of linear regression line over period
// slope expressed as % per bar
// LONG: slope > 0.001 (0.1% per bar) AND rising
// SHORT: slope < -0.001 (-0.1% per bar) AND falling
// Neutral: |slope| < threshold → no trend

pub const LINREG_ENABLED: bool = true;
pub const LINREG_PERIOD: usize = 20;         // regression lookback
pub const LINREG_THRESHOLD: f64 = 0.001;    // trend threshold (0.1% per bar)
pub const LINREG_SL: f64 = 0.005;
pub const LINREG_TP: f64 = 0.004;
pub const LINREG_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn linreg_slope(closes: &[f64], period: usize) -> f64 {
    let n = closes.len();
    if n < period {
        return 0.0;
    }

    let start = n - period;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_xx = 0.0;

    for i in 0..period {
        let x = i as f64;
        let y = closes[start + i];
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_xx += x * x;
    }

    let denom = (period as f64) * sum_xx - sum_x * sum_x;
    if denom == 0.0 {
        return 0.0;
    }

    let slope = ((period as f64) * sum_xy - sum_x * sum_y) / denom;
    slope / closes[n-1] // normalize to % per bar
}
```

---

## Validation Method

1. **Historical backtest** (run232_1_linreg_backtest.py)
2. **Walk-forward** (run232_2_linreg_wf.py)
3. **Combined** (run232_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 40
- THRESHOLD sweep: 0.0005 / 0.001 / 0.002
