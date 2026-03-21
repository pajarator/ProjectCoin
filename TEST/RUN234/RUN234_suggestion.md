# RUN234 — Schaff Trend Cycle (STC): MACD-Stochastic Hybrid for Fast Signals

## Hypothesis

**Mechanism**: STC = 100 × (P, %K) where %K = stochastic(MACD). It first calculates MACD, then applies Stochastic to the MACD values, then smooths with an EMA. The result is a cycle indicator that combines MACD's trend-following with Stochastic's overbought/oversold. When STC crosses above 25 → LONG. When STC crosses below 75 → SHORT. STC is faster than MACD at detecting trend changes.

**Why not duplicate**: No prior RUN uses Schaff Trend Cycle. All prior MACD RUNs use standard MACD. STC is distinct because it applies Stochastic to MACD values — creating a hybrid that is more responsive than either indicator alone.

## Proposed Config Changes (config.rs)

```rust
// ── RUN234: Schaff Trend Cycle (STC) ─────────────────────────────────────
// macd_line = EMA(close, fast) - EMA(close, slow)
// stc = 100 × stochastic(macd_line, k_period), smoothed by ema
// LONG: STC crosses above 25 (from below)
// SHORT: STC crosses below 75 (from above)

pub const STC_ENABLED: bool = true;
pub const STC_FAST: usize = 12;              // MACD fast EMA
pub const STC_SLOW: usize = 26;              // MACD slow EMA
pub const STC_K_PERIOD: usize = 10;         // Stochastic %K period
pub const STC_SIGNAL: usize = 3;             // EMA smoothing of STC
pub const STC_OVERSOLD: f64 = 25.0;         // oversold threshold
pub const STC_OVERBOUGHT: f64 = 75.0;       // overbought threshold
pub const STC_SL: f64 = 0.005;
pub const STC_TP: f64 = 0.004;
pub const STC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run234_1_stc_backtest.py)
2. **Walk-forward** (run234_2_stc_wf.py)
3. **Combined** (run234_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
- K_PERIOD sweep: 8 / 10 / 14
