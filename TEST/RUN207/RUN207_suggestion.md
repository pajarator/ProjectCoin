# RUN207 — Hurst Exponent Regime Detection: Trending vs Mean-Reversion Market Mode

## Hypothesis

**Mechanism**: The Hurst Exponent (H) measures the "memory" of a time series. H > 0.5 = trending market (past price movements predict future direction). H < 0.5 = mean-reverting market (past movements predict opposite direction). H ≈ 0.5 = random walk. COINCLAW should adapt its strategy: H > 0.55 → favor momentum trades, H < 0.45 → favor mean-reversion trades.

**Why not duplicate**: No prior RUN uses Hurst Exponent. All prior regime detection RUNs use fixed thresholds on price-based indicators (SMA20 cross, OU process, ADX). Hurst is a mathematically principled measure of market "memory" — a fundamentally different regime classification method.

## Proposed Config Changes (config.rs)

```rust
// ── RUN207: Hurst Exponent Regime Detection ───────────────────────────────
// H = exponent in R/S analysis: R(n) / S(n) ~ n^H
// Simplified: H = slope of log(R/S) vs log(n)
// H > 0.55 → trending regime → favor momentum (breakout) trades
// H < 0.45 → mean-reversion regime → favor mean-reversion trades
// H 0.45-0.55 → neutral → current COINCLAW regime applies

pub const HURST_ENABLED: bool = true;
pub const HURST_WINDOW: usize = 100;         // lookback window for H calculation
pub const HURST_TREND_THRESH: f64 = 0.55;    // trending threshold
pub const HURST_MEAN_REV_THRESH: f64 = 0.45; // mean-reversion threshold
pub const HURST_MIN_SAMPLES: usize = 50;     // minimum bars for reliable H
```

Add in `indicators.rs`:

```rust
pub fn hurst_exponent(closes: &[f64], max_lags: usize) -> f64 {
    // Simplified Hurst estimation using variance ratio
    // H ≈ 0.5 + var(r_t+n) / (2 * var(r_t)) for lag n
    // This is a simplified proxy; full R/S analysis is iterative
    let n = closes.len();
    if n < max_lags / 2 {
        return 0.5; // neutral
    }

    let mut returns = vec![0.0; n-1];
    for i in 1..n {
        returns[i-1] = (closes[i] - closes[i-1]) / closes[i-1];
    }

    let var_1 = variance(&returns);
    if var_1 == 0.0 {
        return 0.5;
    }

    let lag = max_lags / 4;
    let mut var_n = 0.0;
    for i in lag..returns.len() {
        let diff = returns[i] - returns[i-lag];
        var_n += diff * diff;
    }
    var_n /= (returns.len() - lag) as f64;

    let h = 0.5 + (var_n / (2.0 * var_1)).log2() / (lag as f64).log2();
    h.max(0.0).min(1.0)
}
```

Modify `engine.rs` to check Hurst regime and select strategy type accordingly.

---

## Validation Method

1. **Historical backtest** (run207_1_hurst_backtest.py)
2. **Walk-forward** (run207_2_hurst_wf.py)
3. **Combined** (run207_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 50 / 100 / 200
- TREND_THRESH sweep: 0.50 / 0.55 / 0.60
- MEAN_REV_THRESH sweep: 0.40 / 0.45 / 0.50
