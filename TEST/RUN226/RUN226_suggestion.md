# RUN226 — Ehlers Super Smooth RSI: Zero-Lag RSI for Faster Signals

## Hypothesis

**Mechanism**: Standard RSI has lag because it's based on price changes. Ehlers Super Smooth RSI applies a roofing filter (high-pass filter) to remove noise before calculating RSI, resulting in a zero-lag response. The roofing filter uses a combination of high-pass and low-pass filters to isolate the meaningful price cycles. When Super Smooth RSI crosses above/below thresholds → faster signal than standard RSI.

**Why not duplicate**: No prior RUN uses Ehlers indicators. All prior RSI RUNs use standard RSI. Ehlers' roofing filter approach is specifically designed to reduce lag — a meaningful improvement for a mean-reversion strategy where earlier signals matter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN226: Ehlers Super Smooth RSI ──────────────────────────────────────
// Roofing filter = high-pass filter followed by low-pass SuperSmooth filter
// Then compute RSI on the filtered data
// SUPER_SMOOTH_RSI = RSI(filtered_close, period)
// LONG: crosses above 30
// SHORT: crosses below 70

pub const EHLERS_RSI_ENABLED: bool = true;
pub const EHLERS_RSI_PERIOD: usize = 14;     // RSI period
pub const EHLERS_HP_PERIOD: usize = 20;     // high-pass filter period
pub const EHLERS_RSI_OVERSOLD: f64 = 30.0;  // oversold threshold
pub const EHLERS_RSI_OVERBOUGHT: f64 = 70.0; // overbought threshold
pub const EHLERS_RSI_SL: f64 = 0.005;
pub const EHLERS_RSI_TP: f64 = 0.004;
pub const EHLERS_RSI_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn ehlers_supersmooth_rsi(closes: &[f64], rsi_period: usize, hp_period: usize) -> f64 {
    // High-pass filter first
    let n = closes.len();
    if n < hp_period {
        return 50.0;
    }

    let alpha = 2.0 / (hp_period as f64 + 1.0);
    let mut hp_filter = vec![0.0; n];
    for i in 1..n {
        hp_filter[i] = (closes[i] - closes[i-1]) + (1.0 - alpha) * hp_filter[i-1];
    }

    // Low-pass (SuperSmooth) filter - simplified 2-pole EMA
    let fast = 0.5;
    let slow = 0.3;
    let mut smooth = vec![0.0; n];
    for i in 1..n {
        smooth[i] = fast * hp_filter[i] + (1.0 - fast) * smooth[i-1];
        smooth[i] = slow * smooth[i] + (1.0 - slow) * smooth[i-1];
    }

    let filtered = smooth[n-1];
    // Compute RSI on filtered data
    rsi(&[filtered], rsi_period)
}
```

---

## Validation Method

1. **Historical backtest** (run226_1_ehlers_backtest.py)
2. **Walk-forward** (run226_2_ehlers_wf.py)
3. **Combined** (run226_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- HP_PERIOD sweep: 14 / 20 / 30
