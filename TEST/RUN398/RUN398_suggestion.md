# RUN398 — Stochastic Oscillator with Bollinger Band Squeeze Exit

## Hypothesis

**Mechanism**: Bollinger Band squeeze occurs when the bands narrow significantly, indicating compressed volatility. When volatility expands after a squeeze, price often moves explosively in one direction. Stochastic Oscillator provides momentum confirmation. When BB is in squeeze (narrow) AND Stochastic fires an overbought/oversold crossover signal, the squeeze is releasing and the Stochastic crossover has high directional conviction. Take the trade in the direction of the squeeze release.

**Why not duplicate**: RUN337 uses Stochastic Extreme Shift (uses Stochastic extremes but not BB squeeze context). RUN332 uses Volume-Weighted Bollinger Band Touch. This RUN specifically uses BB squeeze (volatility contraction) as the regime condition with Stochastic crossovers as the entry trigger — the distinct mechanism is using BB squeeze to time Stochastic entries, filtering out signals during non-squeeze periods.

## Proposed Config Changes (config.rs)

```rust
// ── RUN398: Stochastic Oscillator with Bollinger Band Squeeze Exit ─────────────────────────────
// bb_width = (bb_upper - bb_lower) / bb_middle  (normalized width)
// squeeze: bb_width < BB_SQUEEZE_THRESH indicates compressed volatility
// stochastic: %K and %D crossover in overbought/oversold zone
// squeeze_release: bb_width expanding after squeeze AND stochastic crossover fires
// LONG: squeeze_release to upside AND stochastic %K crosses above %D in oversold zone
// SHORT: squeeze_release to downside AND stochastic %K crosses below %D in overbought zone

pub const STOCH_BBSQ_ENABLED: bool = true;
pub const STOCH_BBSQ_STOCH_PERIOD: usize = 14;
pub const STOCH_BBSQ_STOCH_SMOOTH: usize = 3;
pub const STOCH_BBSQ_STOCH_OVERSOLD: f64 = 20.0;
pub const STOCH_BBSQ_STOCH_OVERBOUGHT: f64 = 80.0;
pub const STOCH_BBSQ_BB_PERIOD: usize = 20;
pub const STOCH_BBSQ_BB_STD: f64 = 2.0;
pub const STOCH_BBSQ_SQUEEZE_THRESH: f64 = 0.05;  // bb_width below this = squeeze
pub const STOCH_BBSQ_SL: f64 = 0.005;
pub const STOCH_BBSQ_TP: f64 = 0.004;
pub const STOCH_BBSQ_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run398_1_stoch_bbsq_backtest.py)
2. **Walk-forward** (run398_2_stoch_bbsq_wf.py)
3. **Combined** (run398_3_combined.py)

## Out-of-Sample Testing

- STOCH_PERIOD sweep: 10 / 14 / 21
- STOCH_OVERSOLD sweep: 15 / 20 / 25
- STOCH_OVERBOUGHT sweep: 75 / 80 / 85
- SQUEEZE_THRESH sweep: 0.04 / 0.05 / 0.06
