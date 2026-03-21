# RUN383 — Mass Index with Aroon Oscillator Trend Exhaustion Confirmation

## Hypothesis

**Mechanism**: The Mass Index identifies trend reversals by measuring the narrowing and widening of the average range between high and low prices. A "reversal bulb" forms when the Mass Index rises above a threshold then drops below it — this often precedes trend changes. However, the Mass Index alone doesn't give trend direction. Pair it with the Aroon Oscillator: when Mass Index reverses AND Aroon confirms trend exhaustion (Aroon_UP and Aroon_DOWN both near 50%, meaning neither has dominance), the market is transitioning from trend to range. Fade the prior trend in this transition zone.

**Why not duplicate**: RUN314 uses Mass Index with ADR Confirmation. This RUN uses Mass Index with Aroon Oscillator — a distinctly different confirmation mechanism. Aroon Oscillator measuring trend exhaustion (neither indicator dominating) is a different signal than ADR volume confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN383: Mass Index with Aroon Oscillator Trend Exhaustion ─────────────────
// mass_index = sum(EMA_range / EMA_range_ema, PERIOD) where range = high - low
// reversal_bulb: mass rises above 27 then drops below 26.5
// aroon = (period - bars_since_high) / period * 100
// aroon_oscillator = aroon_up - aroon_down
// trend_exhaustion: aroon_oscillator near 0 (neither dominating)
// LONG: reversal_bulb forms AND prior trend weakening ( aroon_oscillator trending toward 0 )
// SHORT: reversal_bulb forms AND prior trend weakening

pub const MASS_AROON_ENABLED: bool = true;
pub const MASS_AROON_MASS_PERIOD: usize = 25;
pub const MASS_AROON_MASS_HIGH: f64 = 27.0;
pub const MASS_AROON_MASS_LOW: f64 = 26.5;
pub const MASS_AROON_AROON_PERIOD: usize = 14;
pub const MASS_AROON_AROON_EXHAUST: f64 = 20.0;  // aroon_oscillator must be within ±20 of 0
pub const MASS_AROON_SL: f64 = 0.005;
pub const MASS_AROON_TP: f64 = 0.004;
pub const MASS_AROON_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run383_1_mass_aroon_backtest.py)
2. **Walk-forward** (run383_2_mass_aroon_wf.py)
3. **Combined** (run383_3_combined.py)

## Out-of-Sample Testing

- MASS_PERIOD sweep: 20 / 25 / 30
- MASS_HIGH sweep: 25 / 27 / 29
- MASS_LOW sweep: 24.5 / 26.5 / 28.5
- AROON_EXHAUST sweep: 15 / 20 / 25
