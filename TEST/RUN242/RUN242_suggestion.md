# RUN242 — EMA Distance Ratio: Percent Deviation Mean Reversion

## Hypothesis

**Mechanism**: EMA Distance = (close - EMA) / EMA × 100. It measures how far price has strayed from its EMA in percentage terms. When distance > +5% (price well above EMA) → extended, likely to revert down. When distance < -5% → price well below EMA, likely to revert up. The specific thresholds can be calibrated per coin based on historical distribution.

**Why not duplicate**: No prior RUN uses EMA distance ratio. All prior mean-reversion RUNs use Z-score or Bollinger Bands. EMA distance is unique because it's a *percentage-based* measure of deviation from a moving average — more intuitive and comparable across coins.

## Proposed Config Changes (config.rs)

```rust
// ── RUN242: EMA Distance Ratio ───────────────────────────────────────────
// ema_distance = (close - EMA(close, period)) / EMA(close, period) × 100
// LONG: ema_distance < -5% (price below EMA, mean-revert up)
// SHORT: ema_distance > +5% (price above EMA, mean-revert down)

pub const EMA_DIST_ENABLED: bool = true;
pub const EMA_DIST_PERIOD: usize = 20;       // EMA period
pub const EMA_DIST_LONG_THRESH: f64 = -5.0;  // oversold threshold (%)
pub const EMA_DIST_SHORT_THRESH: f64 = 5.0;  // overbought threshold (%)
pub const EMA_DIST_SL: f64 = 0.005;
pub const EMA_DIST_TP: f64 = 0.004;
pub const EMA_DIST_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn ema_distance(closes: &[f64], period: usize) -> f64 {
    let ema_val = ema(closes, period);
    let current = closes[closes.len() - 1];
    if ema_val == 0.0 {
        return 0.0;
    }
    ((current - ema_val) / ema_val) * 100.0
}
```

---

## Validation Method

1. **Historical backtest** (run242_1_ema_dist_backtest.py)
2. **Walk-forward** (run242_2_ema_dist_wf.py)
3. **Combined** (run242_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 50
- LONG_THRESH sweep: -3 / -5 / -7
- SHORT_THRESH sweep: 3 / 5 / 7
