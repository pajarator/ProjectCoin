# RUN411 — Schaff Trend Cycle with Bollinger Band Width Compression

## Hypothesis

**Mechanism**: The Schaff Trend Cycle (STC) is a unique momentum indicator that combines the concepts of MACD and Stochastic into a single cycle-based oscillator. It fires faster than MACD and is less prone to false signals than Stochastic. Bollinger Band Width Compression (squeeze) indicates low volatility and market compression. When STC triggers a signal AND BB Width is compressed, the squeeze is being released with cycle-based momentum confirmation — a powerful combination for explosive moves.

**Why not duplicate**: RUN353 uses Schaff Trend Cycle with RSI Extreme. This RUN uses STC with BB Width Compression — a distinctly different filter that uses volatility contraction to time STC entries. BB squeeze as a regime filter for STC is different from RSI extreme as a confirmation filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN411: Schaff Trend Cycle with Bollinger Band Width Compression ─────────────────────────────
// stc = schaff_trend_cycle(fast_ema, slow_ema, signal_period)
// stc_cross: stc crosses above/below STC_THRESH
// bb_width = (bb_upper - bb_lower) / bb_middle
// squeeze: bb_width < BB_SQUEEZE_THRESH (compressed volatility)
// LONG: stc crosses above STC_THRESH AND squeeze releasing (bb_width expanding)
// SHORT: stc crosses below STC_THRESH AND squeeze releasing (bb_width expanding)

pub const STC_BBW_ENABLED: bool = true;
pub const STC_BBW_FAST_PERIOD: usize = 23;
pub const STC_BBW_SLOW_PERIOD: usize = 50;
pub const STC_BBW_SIGNAL_PERIOD: usize = 10;
pub const STC_BBW_THRESH: f64 = 25.0;       // stc oversold/overbought threshold
pub const STC_BBW_BB_PERIOD: usize = 20;
pub const STC_BBW_BB_STD: f64 = 2.0;
pub const STC_BBW_SQUEEZE_THRESH: f64 = 0.05;
pub const STC_BBW_SL: f64 = 0.005;
pub const STC_BBW_TP: f64 = 0.004;
pub const STC_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run411_1_stc_bbw_backtest.py)
2. **Walk-forward** (run411_2_stc_bbw_wf.py)
3. **Combined** (run411_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 20 / 23 / 30
- SLOW_PERIOD sweep: 40 / 50 / 60
- SIGNAL_PERIOD sweep: 7 / 10 / 14
- SQUEEZE_THRESH sweep: 0.04 / 0.05 / 0.06
