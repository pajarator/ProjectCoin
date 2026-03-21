# RUN353 — Schaff Trend Cycle with RSI Extreme Filter

## Hypothesis

**Mechanism**: STC (Schaff Trend Cycle) is a fast cycle indicator that combines MACD, stochastic, and moving average concepts. It's faster than MACD at detecting trend changes. This RUN adds an RSI extreme filter: only take STC signals when RSI is at extreme (oversold < 35 or overbought > 65). STC gives the timing, RSI extreme confirms the market is at an exhaustion point.

**Why not duplicate**: RUN234 uses STC as a standalone indicator. This RUN specifically combines STC with RSI extreme filter — the RSI adds an extra layer of confirmation that the market is at an extreme, making the signal higher probability.

## Proposed Config Changes (config.rs)

```rust
// ── RUN353: Schaff Trend Cycle with RSI Extreme Filter ────────────────────────────────
// stc = schaff_trend_cycle(fast, slow, signal)
// stc_cross_up = stc crosses above STC_TRIGGER
// stc_cross_down = stc crosses below STC_TRIGGER
// LONG: stc_cross_up AND RSI < RSI_MAX
// SHORT: stc_cross_down AND RSI > RSI_MIN

pub const STC_RSI_ENABLED: bool = true;
pub const STC_RSI_FAST: usize = 23;
pub const STC_RSI_SLOW: usize = 50;
pub const STC_RSI_SIGNAL: usize = 10;
pub const STC_RSI_TRIGGER: f64 = 25.0;      // stc crossover trigger level
pub const STC_RSI_RSI_MAX: f64 = 65.0;    // max RSI for LONG
pub const STC_RSI_RSI_MIN: f64 = 35.0;    // min RSI for SHORT
pub const STC_RSI_SL: f64 = 0.005;
pub const STC_RSI_TP: f64 = 0.004;
pub const STC_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run353_1_stc_rsi_backtest.py)
2. **Walk-forward** (run353_2_stc_rsi_wf.py)
3. **Combined** (run353_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 18 / 23 / 34
- SLOW sweep: 40 / 50 / 75
- TRIGGER sweep: 20 / 25 / 30
- RSI_MAX sweep: 60 / 65 / 70
