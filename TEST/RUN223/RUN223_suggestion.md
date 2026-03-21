# RUN223 — Volume-Price Trend (VPT): Cumulative Momentum with Volume Weighting

## Hypothesis

**Mechanism**: VPT = prior VPT + volume × (close - prior_close) / prior_close. It combines price change direction with volume magnitude — a price rise on high volume adds more to VPT than a rise on low volume. Rising VPT = sustained accumulation. Falling VPT = sustained distribution. VPT divergence from price = reversal signal.

**Why not duplicate**: No prior RUN uses VPT. All prior cumulative indicators use OBV (which ignores magnitude) or VA (which uses close location). VPT is unique because it weights volume by the *percentage* price change — volume × return, not just volume.

## Proposed Config Changes (config.rs)

```rust
// ── RUN223: Volume-Price Trend (VPT) ─────────────────────────────────────
// vpt = prior_vpt + volume × (close - prior_close) / prior_close
// LONG: VPT rising AND price rising (confirmed)
// SHORT: VPT falling AND price falling (confirmed)
// Divergence: price rising but VPT flat/falling = weakness

pub const VPT_ENABLED: bool = true;
pub const VPT_PERIOD: usize = 20;            // smoothing MA period
pub const VPT_SL: f64 = 0.005;
pub const VPT_TP: f64 = 0.004;
pub const VPT_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn volume_price_trend(closes: &[f64], volumes: &[f64], period: usize) -> (f64, bool) {
    let n = closes.len().min(volumes.len());
    if n < 2 {
        return (0.0, false);
    }

    let mut vpt = 0.0;
    for i in 1..n {
        let pct_change = (closes[i] - closes[i-1]) / closes[i-1];
        vpt += volumes[i] * pct_change;
    }

    // VPT trend: compare to smoothed VPT
    let vpt_ma = vpt / (n as f64); // simplified
    let rising = vpt > vpt_ma;

    (vpt, rising)
}
```

---

## Validation Method

1. **Historical backtest** (run223_1_vpt_backtest.py)
2. **Walk-forward** (run223_2_vpt_wf.py)
3. **Combined** (run223_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
