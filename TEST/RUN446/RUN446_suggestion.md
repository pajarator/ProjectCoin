# RUN446 — PPO Histogram with SuperTrend Confirmation

## Hypothesis

**Mechanism**: The Percentage Price Oscillator (PPO) is similar to MACD but uses percentage calculations, making it comparable across different price levels. The PPO Histogram shows the difference between PPO and its signal line. SuperTrend provides ATR-based trend direction and trailing stops. When PPO Histogram flips direction AND SuperTrend confirms the same direction, you have both momentum oscillator direction AND ATR-based trend confirmation working together.

**Why not duplicate**: RUN308 uses PPO Histogram Divergence standalone. This RUN adds SuperTrend confirmation — the distinct mechanism is using SuperTrend's ATR-based trend direction to confirm PPO histogram momentum changes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN446: PPO Histogram with SuperTrend Confirmation ─────────────────────────────────────
// ppo = (EMA(close, fast) - EMA(close, slow)) / EMA(close, slow) * 100
// ppo_signal = EMA(ppo, signal_period)
// ppo_histogram = ppo - ppo_signal
// hist_flip: histogram crosses above/below 0
// supertrend: atr-based trend direction
// supertrend_flip: trend changes direction
// LONG: histogram flips bullish AND supertrend bullish
// SHORT: histogram flips bearish AND supertrend bearish

pub const PPO_ST_ENABLED: bool = true;
pub const PPO_ST_FAST_PERIOD: usize = 12;
pub const PPO_ST_SLOW_PERIOD: usize = 26;
pub const PPO_ST_SIGNAL_PERIOD: usize = 9;
pub const PPO_ST_ST_PERIOD: usize = 10;
pub const PPO_ST_ST_MULT: f64 = 3.0;
pub const PPO_ST_SL: f64 = 0.005;
pub const PPO_ST_TP: f64 = 0.004;
pub const PPO_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run446_1_ppo_st_backtest.py)
2. **Walk-forward** (run446_2_ppo_st_wf.py)
3. **Combined** (run446_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 8 / 12 / 16
- SLOW_PERIOD sweep: 21 / 26 / 30
- SIGNAL_PERIOD sweep: 7 / 9 / 12
- ST_MULT sweep: 2.0 / 3.0 / 4.0
