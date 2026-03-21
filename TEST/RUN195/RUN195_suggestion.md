# RUN195 — Ease of Movement (EOM): Volume-Price Efficiency for Momentum Entry

## Hypothesis

**Mechanism**: Ease of Movement = (high - low) / volume × 100,000. It measures how easily price moves per unit of volume. High EOM = price moves easily on low volume (efficient), suggesting institutional accumulation/distribution. Low EOM = price struggles to move even with high volume (congestion). Crossover of EOM above its moving average = easy movement bullish; below = easy movement bearish.

**Why not duplicate**: No prior RUN uses Ease of Movement. All volume-based RUNs use MFI, OBV, or volume spikes. EOM is unique because it measures the *efficiency* of price movement per unit volume — a fundamentally different dimension.

## Proposed Config Changes (config.rs)

```rust
// ── RUN195: Ease of Movement (EOM) ───────────────────────────────────────
// eom = ((high - low) / volume) × 1_000_000
// eom_ma = SMA(eom, period)
// LONG: eom crosses above eom_ma AND eom > 0
// SHORT: eom crosses below eom_ma AND eom < 0
// Zero line crossover as secondary confirmation

pub const EOM_ENABLED: bool = true;
pub const EOM_PERIOD: usize = 14;          // smoothing period for EOM MA
pub const EOM_SL: f64 = 0.005;
pub const EOM_TP: f64 = 0.004;
pub const EOM_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn ease_of_movement(highs: &[f64], lows: &[f64], volumes: &[f64], period: usize) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(volumes.len());
    if n == 0 {
        return (0.0, 0.0);
    }

    let mut eom_sum = 0.0;
    for i in 0..n {
        let range = highs[i] - lows[i];
        let vol = volumes[i];
        if vol > 0.0 {
            eom_sum += (range / vol) * 1_000_000.0;
        }
    }

    let eom = eom_sum / (n as f64);
    let eom_ma = eom; // simplified - real impl needs SMA of eom series

    (eom, eom_ma)
}
```

---

## Validation Method

1. **Historical backtest** (run195_1_eom_backtest.py)
2. **Walk-forward** (run195_2_eom_wf.py)
3. **Combined** (run195_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 20 / 30
