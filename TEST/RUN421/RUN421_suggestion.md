# RUN421 — Chaikin Oscillator with VWAP Deviation Confirmation

## Hypothesis

**Mechanism**: The Chaikin Oscillator measures the momentum of the Accumulation/Distribution Line by taking the difference between the fast and slow EMAs of the A/D line. It identifies whether money is flowing into or out of an asset. VWAP Deviation confirms when price has strayed too far from its volume-weighted average. When Chaikin Oscillator flips direction AND price hasdeviated significantly from VWAP, you have both money flow momentum change AND price distance confirmation of a reversal opportunity.

**Why not duplicate**: RUN389 uses Accumulation/Distribution with SuperTrend. This RUN specifically uses the Chaikin Oscillator (the derivative/momentum version of A/D line) with VWAP Deviation — the distinct mechanism is using Chaikin's momentum of money flow combined with VWAP deviation distance.

## Proposed Config Changes (config.rs)

```rust
// ── RUN421: Chaikin Oscillator with VWAP Deviation Confirmation ───────────────────────────────
// chaikin_osc = EMA(ad_line, fast_period) - EMA(ad_line, slow_period)
// chaikin_flip: oscillator crosses above/below 0
// vwap_deviation = |close - vwap| / vwap
// high_deviation: vwap_deviation > DEV_THRESH (price far from fair value)
// LONG: chaikin_flip bullish AND vwap_deviation > threshold
// SHORT: chaikin_flip bearish AND vwap_deviation > threshold

pub const CHAIKIN_VWAP_ENABLED: bool = true;
pub const CHAIKIN_VWAP_FAST_PERIOD: usize = 3;
pub const CHAIKIN_VWAP_SLOW_PERIOD: usize = 10;
pub const CHAIKIN_VWAP_VWAP_PERIOD: usize = 20;
pub const CHAIKIN_VWAP_DEV_THRESH: f64 = 0.01;  // 1% deviation from VWAP
pub const CHAIKIN_VWAP_SL: f64 = 0.005;
pub const CHAIKIN_VWAP_TP: f64 = 0.004;
pub const CHAIKIN_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run421_1_chaikin_vwap_backtest.py)
2. **Walk-forward** (run421_2_chaikin_vwap_wf.py)
3. **Combined** (run421_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 2 / 3 / 5
- SLOW_PERIOD sweep: 7 / 10 / 14
- VWAP_PERIOD sweep: 14 / 20 / 30
- DEV_THRESH sweep: 0.005 / 0.01 / 0.015
