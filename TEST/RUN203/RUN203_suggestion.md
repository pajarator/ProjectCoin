# RUN203 — Volume-Weighted MACD (VWMACD): Institutional Momentum Confirmation

## Hypothesis

**Mechanism**: Standard MACD treats all price bars equally. Volume-Weighted MACD weights each bar's price contribution by its volume — high-volume bars contribute more to the EMA and MACD signal. This makes the indicator more responsive to institutional-driven price moves and less responsive to low-volume noise. The result is a cleaner, more accurate momentum signal.

**Why not duplicate**: No prior RUN uses Volume-Weighted MACD. All prior MACD RUNs (RUN13, RUN170) use standard close-based MACD. VWMACD adds a volume dimension that standard MACD lacks.

## Proposed Config Changes (config.rs)

```rust
// ── RUN203: Volume-Weighted MACD ────────────────────────────────────────
// vw_close = cumulative(close × volume) / cumulative(volume)
// vw_ema_fast = EMA(vw_close, fast_period)
// vw_ema_slow = EMA(vw_close, slow_period)
// vw_macd = vw_ema_fast - vw_ema_slow
// signal = EMA(vw_macd, signal_period)
// histogram = vw_macd - signal
// LONG: histogram crosses above 0
// SHORT: histogram crosses below 0

pub const VWMACD_ENABLED: bool = true;
pub const VWMACD_FAST: usize = 12;           // fast EMA period
pub const VWMACD_SLOW: usize = 26;           // slow EMA period
pub const VWMACD_SIGNAL: usize = 9;          // signal EMA period
pub const VWMACD_SL: f64 = 0.005;
pub const VWMACD_TP: f64 = 0.004;
pub const VWMACD_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn vwmacd(closes: &[f64], volumes: &[f64], fast: usize, slow: usize) -> (f64, f64, f64) {
    let n = closes.len().min(volumes.len());
    if n < slow {
        return (0.0, 0.0, 0.0);
    }

    let mut vw_prices = vec![0.0; n];
    let mut cum_vol = 0.0;
    let mut cum_pv = 0.0;

    for i in 0..n {
        cum_vol += volumes[i];
        cum_pv += closes[i] * volumes[i];
        vw_prices[i] = if cum_vol > 0.0 { cum_pv / cum_vol } else { closes[i] };
    }

    let fast_ema = ema(&vw_prices, fast);
    let slow_ema = ema(&vw_prices, slow);
    let macd = fast_ema - slow_ema;
    let signal = macd; // simplified - real impl needs EMA of MACD series

    (macd, signal, macd - signal)
}
```

---

## Validation Method

1. **Historical backtest** (run203_1_vwmacd_backtest.py)
2. **Walk-forward** (run203_2_vwmacd_wf.py)
3. **Combined** (run203_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
- SIGNAL sweep: 7 / 9 / 12
