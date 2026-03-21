# RUN194 — TRIX Momentum Oscillator: Triple-EMA Noise Filter for Direction

## Hypothesis

**Mechanism**: TRIX = rate-of-change of a triple-smoothed EMA (EMA of EMA of EMA). The triple smoothing eliminates market noise and small price fluctuations. A rising TRIX = sustained bullish momentum. TRIX crossing above zero = bullish confirmation. TRIX crossing below zero = bearish confirmation. Divergence from price signals reversal.

**Why not duplicate**: No prior RUN uses TRIX. All prior momentum oscillators use RSI, MACD, or Stochastic. TRIX is fundamentally different because triple smoothing makes it less sensitive to short-term noise than MACD (which is only double-smoothed).

## Proposed Config Changes (config.rs)

```rust
// ── RUN194: TRIX Momentum Oscillator ────────────────────────────────────
// trix = 100 × (EMA3(close, period) / EMA3(close, period-1) - 1)
// where EMA3 = triple EMA
// LONG: TRIX crosses above signal line (or zero)
// SHORT: TRIX crosses below signal line (or zero)
// Signal line = 9-period EMA of TRIX

pub const TRIX_ENABLED: bool = true;
pub const TRIX_PERIOD: usize = 15;         // EMA period for triple smoothing
pub const TRIX_SIGNAL: usize = 9;          // signal line EMA period
pub const TRIX_OVERSOLD: f64 = -0.5;      // oversold threshold (below = bearish)
pub const TRIX_OVERBOUGHT: f64 = 0.5;      // overbought threshold (above = bullish)
pub const TRIX_SL: f64 = 0.005;
pub const TRIX_TP: f64 = 0.004;
pub const TRIX_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn trix(closes: &[f64], period: usize) -> f64 {
    if closes.len() < period * 3 {
        return 0.0;
    }
    let ema1 = ema(closes, period);
    let ema2 = ema(&[ema1], period); // simplified - real impl needs full array
    let ema3 = ema(&[ema2], period);

    if ema2 == 0.0 {
        return 0.0;
    }
    let trix = 100.0 * (ema3 / ema2 - 1.0);
    trix
}
```

---

## Validation Method

1. **Historical backtest** (run194_1_trix_backtest.py)
2. **Walk-forward** (run194_2_trix_wf.py)
3. **Combined** (run194_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 15 / 20 / 30
- SIGNAL sweep: 7 / 9 / 12
- OVERSOLD sweep: -1.0 / -0.5 / -0.2
- OVERBOUGHT sweep: 0.2 / 0.5 / 1.0
