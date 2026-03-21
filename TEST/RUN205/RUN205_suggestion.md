# RUN205 — Elder Ray Index: Bull Power vs Bear Power Relative to EMA

## Hypothesis

**Mechanism**: Elder Ray = price minus EMA(close, 13). Bull Power = high - EMA (ability to push above consensus). Bear Power = low - EMA (ability to push below consensus). When EMA is rising AND Bear Power is rising (less negative) → LONG entry. When EMA is falling AND Bull Power is falling (less positive) → SHORT entry. Filters out weak moves.

**Why not duplicate**: No prior RUN uses Elder Ray. All prior EMA-cross RUNs use price vs EMA directly. Elder Ray adds a layer of analysis by separately measuring how price exceeds EMA at highs (bull power) vs lows (bear power) — a more nuanced momentum measurement.

## Proposed Config Changes (config.rs)

```rust
// ── RUN205: Elder Ray Index ──────────────────────────────────────────────
// bull_power = high - EMA(close, period)
// bear_power = low - EMA(close, period)
// LONG: EMA rising AND bear_power > bear_power[prev] (rising, less negative)
// SHORT: EMA falling AND bull_power < bull_power[prev] (falling, less positive)
// Confluence: both conditions + regime alignment

pub const ELDER_ENABLED: bool = true;
pub const ELDER_EMA_PERIOD: usize = 13;      // standard Elder Ray period
pub const ELDER_BULL_THRESH: f64 = 0.0;      // bull power > 0 = strong
pub const ELDER_BEAR_THRESH: f64 = 0.0;      // bear power < 0 = strong (stored as neg)
pub const ELDER_SL: f64 = 0.005;
pub const ELDER_TP: f64 = 0.004;
pub const ELDER_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn elder_ray(closes: &[f64], highs: &[f64], lows: &[f64], period: usize) -> (f64, f64, bool, bool) {
    let ema_val = ema(closes, period);
    let n = closes.len();
    if n == 0 {
        return (0.0, 0.0, false, false);
    }

    let high = highs[n-1];
    let low = lows[n-1];

    let bull_power = high - ema_val;
    let bear_power = low - ema_val;

    // EMA trend: rising if current EMA > prior EMA
    let ema_rising = if n >= 2 {
        let prior_closes = &[closes[n-2]];
        let prior_ema = ema(prior_closes, period);
        ema_val > prior_ema
    } else {
        true
    };

    let bull_cond = bull_power > 0.0;
    let bear_cond = bear_power < 0.0;

    (bull_power, bear_power, bull_cond, bear_cond)
}
```

---

## Validation Method

1. **Historical backtest** (run205_1_elder_backtest.py)
2. **Walk-forward** (run205_2_elder_wf.py)
3. **Combined** (run205_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 13 / 20
