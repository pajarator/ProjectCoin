# RUN496 — Intraday Momentum Index with Choppiness Index

## Hypothesis

**Mechanism**: Intraday Momentum Index (IMI) combines RSI analysis with candlestick patterns to identify intraday overbought/oversold conditions. Unlike RSI which uses only closing prices, IMI considers the relationship between open and close prices within the trading range. Choppiness Index (CI) measures market "choppiness" — high CI means ranging market, low CI means trending. When IMI signals AND CI indicates a trending environment, entries have both intraday momentum timing and favorable market regime.

**Why not duplicate**: IMI is a unique indicator not yet tested in the RUN system. Choppiness Index as a confirmation mechanism is also distinct from the typical oscillator or volume confirmations used elsewhere.

## Proposed Config Changes (config.rs)

```rust
// ── RUN496: Intraday Momentum Index with Choppiness Index ─────────────────────────────────
// imi: intraday_momentum_index combining rsi with open_close range analysis
// imi_cross: imi crosses above/below 50 or extreme levels
// choppiness_index: ci value indicating trending (<40) vs ranging (>60)
// LONG: imi < 30 (oversold) AND ci < 50 (not choppy) AND imi starting to rise
// SHORT: imi > 70 (overbought) AND ci < 50 (not choppy) AND imi starting to fall

pub const IMI_CI_ENABLED: bool = true;
pub const IMI_CI_IMI_PERIOD: usize = 14;
pub const IMI_CI_IMI_OVERSOLD: f64 = 30.0;
pub const IMI_CI_IMI_OVERBOUGHT: f64 = 70.0;
pub const IMI_CI_CI_PERIOD: usize = 14;
pub const IMI_CI_CI_THRESH: f64 = 50.0;
pub const IMI_CI_SL: f64 = 0.005;
pub const IMI_CI_TP: f64 = 0.004;
pub const IMI_CI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run496_1_imi_ci_backtest.py)
2. **Walk-forward** (run496_2_imi_ci_wf.py)
3. **Combined** (run496_3_combined.py)

## Out-of-Sample Testing

- IMI_PERIOD sweep: 10 / 14 / 20
- IMI_OVERSOLD sweep: 25 / 30 / 35
- IMI_OVERBOUGHT sweep: 65 / 70 / 75
- CI_PERIOD sweep: 10 / 14 / 20
- CI_THRESH sweep: 45 / 50 / 55
