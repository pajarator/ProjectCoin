# RUN254 — Volume Delta: Net Buyer/Seller Pressure from Tick Data

## Hypothesis

**Mechanism**: Volume Delta = buy volume - sell volume (estimated from price movement within each bar). When price closes in the upper half of the bar's range with positive delta → aggressive buying. When price closes in lower half with negative delta → aggressive selling. Cumulative delta crossing zero = net buyer/seller pressure shift.

**Why not duplicate**: No prior RUN uses Volume Delta. All prior volume RUNs use raw volume, OBV, or MFI. Volume Delta is unique because it estimates the *direction* of volume within each bar, not just total volume.

## Proposed Config Changes (config.rs)

```rust
// ── RUN254: Volume Delta ─────────────────────────────────────────────────
// delta = volume × ((close - low) - (high - close)) / (high - low)
// positive_delta = buying pressure, negative_delta = selling pressure
// cumulative_delta = running sum of delta
// LONG: cumulative_delta crosses above 0 (net buying)
// SHORT: cumulative_delta crosses below 0 (net selling)

pub const VOL_DELTA_ENABLED: bool = true;
pub const VOL_DELTA_PERIOD: usize = 20;     // smoothing period
pub const VOL_DELTA_SL: f64 = 0.005;
pub const VOL_DELTA_TP: f64 = 0.004;
pub const VOL_DELTA_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn volume_delta(highs: &[f64], lows: &[f64], closes: &[f64], volumes: &[f64]) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len()).min(volumes.len());
    if n == 0 {
        return (0.0, 0.0);
    }

    let mut cum_delta = 0.0;
    for i in 0..n {
        let range = highs[i] - lows[i];
        if range > 0.0 {
            let delta = volumes[i] * ((closes[i] - lows[i]) - (highs[i] - closes[i])) / range;
            cum_delta += delta;
        }
    }

    let delta_ma = cum_delta / (n as f64);
    let positive = cum_delta > delta_ma;

    (cum_delta, if positive { 1.0 } else { -1.0 })
}
```

---

## Validation Method

1. **Historical backtest** (run254_1_vol_delta_backtest.py)
2. **Walk-forward** (run254_2_vol_delta_wf.py)
3. **Combined** (run254_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
