# RUN213 — Cumulative RSI: Smoothed Overbought/Oversold for Longer-Wave Detection

## Hypothesis

**Mechanism**: Cumulative RSI = running sum of RSI differences from 50 (the neutral line). When cumulative RSI rises sharply → sustained bullish momentum. When it falls sharply → sustained bearish momentum. Extreme readings (>+100 or <-100) signal mean-reversion opportunity. The cumulative nature smooths noise and identifies longer-wave cycles.

**Why not duplicate**: No prior RUN uses Cumulative RSI. All prior RSI RUNs use raw RSI values. Cumulative RSI is fundamentally different because it measures the *duration and magnitude* of RSI deviations, not just the instantaneous value.

## Proposed Config Changes (config.rs)

```rust
// ── RUN213: Cumulative RSI ───────────────────────────────────────────────
// cum_rsi = running sum of (RSI - 50) over period
// cum_rsi > +100 → overbought zone (mean-revert short)
// cum_rsi < -100 → oversold zone (mean-revert long)
// Zero-line crossover as trend confirmation

pub const CUM_RSI_ENABLED: bool = true;
pub const CUM_RSI_PERIOD: usize = 20;         // accumulation period
pub const CUM_RSI_RSI_PERIOD: usize = 14;     // base RSI period
pub const CUM_RSI_OVERSOLD: f64 = -100.0;     // oversold threshold
pub const CUM_RSI_OVERBOUGHT: f64 = 100.0;    // overbought threshold
pub const CUM_RSI_SL: f64 = 0.005;
pub const CUM_RSI_TP: f64 = 0.004;
pub const CUM_RSI_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn cumulative_rsi(closes: &[f64], rsi_period: usize, cum_period: usize) -> f64 {
    let n = closes.len();
    if n < cum_period + rsi_period {
        return 0.0;
    }

    let mut cum_rsi = 0.0;
    for i in rsi_period..n {
        let window = &closes[..=i];
        let rsi_val = rsi(window, rsi_period);
        cum_rsi += rsi_val - 50.0;
    }

    cum_rsi
}
```

---

## Validation Method

1. **Historical backtest** (run213_1_cum_rsi_backtest.py)
2. **Walk-forward** (run213_2_cum_rsi_wf.py)
3. **Combined** (run213_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- CUM_PERIOD sweep: 14 / 20 / 30
- OVERSOLD sweep: -80 / -100 / -120
- OVERBOUGHT sweep: 80 / 100 / 120
