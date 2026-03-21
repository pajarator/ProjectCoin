# RUN211 — Volume Accumulation (VA): Close-Location-Weighted Volume Flow

## Hypothesis

**Mechanism**: Volume Accumulation = OBV + close-location weighting. For each bar: if close > open (bullish bar), add volume × (close-low)/(high-low) to VA. If close < open (bearish bar), subtract volume × (high-close)/(high-low). This weights volume by the "effort" shown by the close location — close near the high with volume = strong accumulation.

**Why not duplicate**: No prior RUN uses Volume Accumulation. All prior volume RUNs use raw OBV, MFI, or simple volume spikes. VA is unique because it weights volume by *where* the close sits within the bar's range — a fundamentally different volume analysis.

## Proposed Config Changes (config.rs)

```rust
// ── RUN211: Volume Accumulation (VA) ─────────────────────────────────────
// va_multiplier = if close > open: (close-low)/(high-low)
//                 if close < open: (high-close)/(high-low)
//                 if close == open: 0
// VA = prior_VA + volume × va_multiplier
// LONG: VA rising AND price rising (confirmation)
// SHORT: VA falling AND price falling (confirmation)
// Divergence: price rising but VA falling = distribution warning

pub const VA_ENABLED: bool = true;
pub const VA_PERIOD: usize = 20;             // smoothing period for VA MA
pub const VA_SL: f64 = 0.005;
pub const VA_TP: f64 = 0.004;
pub const VA_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn volume_accumulation(highs: &[f64], lows: &[f64], closes: &[f64], volumes: &[f64], period: usize) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len()).min(volumes.len());
    if n < 2 {
        return (0.0, 0.0);
    }

    let mut va = 0.0;
    for i in 1..n {
        let range = highs[i] - lows[i];
        let mult = if range > 0.0 {
            if closes[i] > closes[i-1] {
                (closes[i] - lows[i]) / range
            } else if closes[i] < closes[i-1] {
                (highs[i] - closes[i]) / range
            } else {
                0.0
            }
        } else {
            0.0
        };
        va += volumes[i] * mult;
    }

    // VA trend: compare recent VA to prior VA
    let va_ma = va / (n as f64); // simplified
    let va_trend_up = va > va_ma;

    (va, if va_trend_up { 1.0 } else { -1.0 })
}
```

---

## Validation Method

1. **Historical backtest** (run211_1_va_backtest.py)
2. **Walk-forward** (run211_2_va_wf.py)
3. **Combined** (run211_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
