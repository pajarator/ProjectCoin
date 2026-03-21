# RUN218 — Qstick Indicator: Open-Close Momentum as Trend Signal

## Hypothesis

**Mechanism**: Qstick = SMA(open - close, period). When Qstick crosses above zero → average bar has been bullish for the period → LONG. When Qstick crosses below zero → average bar has been bearish → SHORT. Qstick essentially measures the "average candle color" over a period — a pure momentum measure.

**Why not duplicate**: No prior RUN uses Qstick. All prior momentum RUNs use price-based or volume-based indicators. Qstick is unique because it specifically measures the open-close relationship, capturing intraday momentum direction cleanly.

## Proposed Config Changes (config.rs)

```rust
// ── RUN218: Qstick Indicator ─────────────────────────────────────────────
// qstick = SMA(open - close, period)
// LONG: qstick crosses above 0
// SHORT: qstick crosses below 0
// Magnitude of qstick = momentum conviction

pub const QSTICK_ENABLED: bool = true;
pub const QSTICK_PERIOD: usize = 8;           // smoothing period
pub const QSTICK_SL: f64 = 0.005;
pub const QSTICK_TP: f64 = 0.004;
pub const QSTICK_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn qstick(opens: &[f64], closes: &[f64], period: usize) -> f64 {
    let n = opens.len().min(closes.len());
    if n < period {
        return 0.0;
    }

    let mut sum = 0.0;
    for i in (n-period)..n {
        sum += opens[i] - closes[i];
    }

    sum / (period as f64)
}
```

---

## Validation Method

1. **Historical backtest** (run218_1_qstick_backtest.py)
2. **Walk-forward** (run218_2_qstick_wf.py)
3. **Combined** (run218_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 4 / 8 / 14 / 20
