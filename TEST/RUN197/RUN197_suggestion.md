# RUN197 — Commodity Channel Index (CCI): Statistical Deviation Mean Reversion

## Hypothesis

**Mechanism**: CCI = (Typical Price - SMA(Typical Price, period)) / (0.015 × Mean Deviation). Typical Price = (High + Low + Close) / 3. CCI measures how many standard deviations the price is from its mean. CCI > +100 = overbought (price far above average, likely to revert). CCI < -100 = oversold. Zero-line crossover is the primary signal; extreme readings (>±100) as confirmation.

**Why not duplicate**: No prior RUN uses CCI. All prior mean-reversion RUNs use RSI, Z-score, or Bollinger Bands. CCI is unique because it uses mean deviation (not standard deviation) and is normalized around a statistical average — a fundamentally different mean-reversion framework.

## Proposed Config Changes (config.rs)

```rust
// ── RUN197: Commodity Channel Index (CCI) Mean Reversion ────────────────
// typical_price = (high + low + close) / 3
// cci = (tp - SMA(tp, period)) / (0.015 × mean_deviation)
// CCI > +100 = overbought (revert short), CCI < -100 = oversold (revert long)
// Zero-line crossover as primary signal

pub const CCI_ENABLED: bool = true;
pub const CCI_PERIOD: usize = 20;          // lookback period
pub const CCI_OVERSOLD: f64 = -100.0;      // oversold threshold
pub const CCI_OVERBOUGHT: f64 = 100.0;     // overbought threshold
pub const CCI_NEUTRAL: f64 = 0.0;         // zero-line for crossover
pub const CCI_SL: f64 = 0.005;
pub const CCI_TP: f64 = 0.004;
pub const CCI_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn cci(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> f64 {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n < period {
        return 0.0;
    }

    let mut typical_prices = vec![0.0; n];
    let mut mean_devs = vec![0.0; n];

    for i in 0..n {
        typical_prices[i] = (highs[i] + lows[i] + closes[i]) / 3.0;
    }

    let tp_sum: f64 = typical_prices.iter().sum();
    let tp_ma = tp_sum / (n as f64);

    let mut mean_dev_sum = 0.0;
    for i in 0..n {
        mean_dev_sum += (typical_prices[i] - tp_ma).abs();
    }
    let mean_dev = mean_dev_sum / (n as f64);

    if mean_dev == 0.0 {
        return 0.0;
    }

    let current_tp = typical_prices[n - 1];
    ((current_tp - tp_ma) / (0.015 * mean_dev))
}
```

---

## Validation Method

1. **Historical backtest** (run197_1_cci_backtest.py)
2. **Walk-forward** (run197_2_cci_wf.py)
3. **Combined** (run197_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30 / 50
- OVERSOLD sweep: -80 / -100 / -120
- OVERBOUGHT sweep: 80 / 100 / 120
