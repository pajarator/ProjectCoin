# RUN209 — Trend Intensity Index (TII): Trend Strength Quantified

## Hypothesis

**Mechanism**: TII = (close - lowest low) / (highest high - lowest low) × 100. It measures where the current price sits within the lookback range — 100 = at the highest high (very strong trend), 0 = at the lowest low (very weak/trending down). When TII > 80 → overbought (exhaustion risk), when TII < 20 → oversold (accumulation zone).

**Why not duplicate**: No prior RUN uses Trend Intensity Index. All prior trend strength RUNs use ADX or DMI. TII is distinct because it measures where price *currently sits* relative to the historical range, giving a percentile-like reading of trend intensity.

## Proposed Config Changes (config.rs)

```rust
// ── RUN209: Trend Intensity Index (TII) ──────────────────────────────────
// tii = (close - lowest_low) / (highest_high - lowest_low) × 100
// LONG: tii crosses above 20 from below (emerging strength)
// SHORT: tii crosses below 80 from above (exhaustion)
// Wait for TII to exit extreme before entry

pub const TII_ENABLED: bool = true;
pub const TII_PERIOD: usize = 30;            // lookback period
pub const TII_OVERSOLD: f64 = 20.0;          // oversold threshold
pub const TII_OVERBOUGHT: f64 = 80.0;         // overbought threshold
pub const TII_SL: f64 = 0.005;
pub const TII_TP: f64 = 0.004;
pub const TII_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn trend_intensity(closes: &[f64], highs: &[f64], lows: &[f64], period: usize) -> f64 {
    let n = closes.len().min(highs.len()).min(lows.len());
    if n < period {
        return 50.0;
    }

    let window = &closes[n-period..];
    let highest = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let lowest = window.iter().cloned().fold(f64::INFINITY, f64::min);
    let range = highest - lowest;

    if range <= 0.0 {
        return 50.0;
    }

    let current = closes[n-1];
    ((current - lowest) / range) * 100.0
}
```

---

## Validation Method

1. **Historical backtest** (run209_1_tii_backtest.py)
2. **Walk-forward** (run209_2_tii_wf.py)
3. **Combined** (run209_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 20 / 30 / 50
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
