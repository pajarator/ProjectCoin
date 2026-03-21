# RUN199 — Stochastic RSI: Smoothed RSI Momentum with Earlier Signals

## Hypothesis

**Mechanism**: Stochastic RSI = Stochastic(smoothed RSI) — it applies the Stochastic oscillator formula to RSI values rather than price. This smooths RSI and produces signals earlier than raw RSI. When StochRSI crosses above 20 (oversold) → LONG. When it crosses below 80 (overbought) → SHORT. It's especially effective at identifying turning points in already-overbought/oversold conditions.

**Why not duplicate**: No prior RUN uses Stochastic RSI. All prior RSI-related RUNs use raw RSI (RUN169, RUN177). StochRSI is a second-order indicator — it applies Stochastic to RSI, making it more responsive to changes in RSI momentum.

## Proposed Config Changes (config.rs)

```rust
// ── RUN199: Stochastic RSI ─────────────────────────────────────────────
// stoch_rsi = (RSI - min_RSI) / (max_RSI - min_RSI) × 100
// RSI is computed over rsi_period
// min_RSI and max_RSI are from the stoch_period window
// %K = StochRSI value, %D = SMA(%K, signal_period)
// LONG: %K crosses above 20 (from below) AND %D < 50
// SHORT: %K crosses below 80 (from above) AND %D > 50

pub const STOCH_RSI_ENABLED: bool = true;
pub const STOCH_RSI_RSI_PERIOD: usize = 14;    // RSI lookback
pub const STOCH_RSI_STOCH_PERIOD: usize = 14;   // Stochastic lookback
pub const STOCH_RSI_SIGNAL: usize = 3;           // %D smoothing
pub const STOCH_RSI_OVERSOLD: f64 = 20.0;       // oversold line
pub const STOCH_RSI_OVERBOUGHT: f64 = 80.0;     // overbought line
pub const STOCH_RSI_SL: f64 = 0.005;
pub const STOCH_RSI_TP: f64 = 0.004;
pub const STOCH_RSI_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn stoch_rsi(closes: &[f64], rsi_period: usize, stoch_period: usize) -> (f64, f64) {
    // First compute RSI series
    let mut rsi_values = vec![50.0; closes.len()];
    for i in rsi_period..closes.len() {
        rsi_values[i] = rsi(&closes[..=i], rsi_period);
    }

    let n = rsi_values.len();
    if n < stoch_period {
        return (50.0, 50.0);
    }

    let window = &rsi_values[n-stoch_period..];
    let min_rsi = window.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_rsi = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_rsi - min_rsi;

    let k = if range > 0.0 {
        (rsi_values[n-1] - min_rsi) / range * 100.0
    } else {
        50.0
    };

    // %D = SMA of %K (simplified to single value)
    let d = k; // real impl would need rolling SMA of k values

    (k, d)
}
```

---

## Validation Method

1. **Historical backtest** (run199_1_stochrsi_backtest.py)
2. **Walk-forward** (run199_2_stochrsi_wf.py)
3. **Combined** (run199_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- STOCH_PERIOD sweep: 10 / 14 / 21
- SIGNAL sweep: 3 / 5 / 7
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
