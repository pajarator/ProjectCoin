# RUN471 — Schaff Trend Cycle with Volume Ratio Spike

## Hypothesis

**Mechanism**: Schaff Trend Cycle (STC) is a momentum oscillator that combines MACD characteristics with a cycle-based smoothing to identify trends faster than traditional indicators. Volume Ratio Spike confirms institutional involvement: when STC fires a signal AND volume is significantly above its recent average, the move has both trend direction and institutional conviction behind it.

**Why not duplicate**: RUN411 uses Schaff Trend Cycle with Bollinger Band Width Compression. This RUN uses Volume Ratio Spike instead — the distinct mechanism is volume-based institutional confirmation versus BB Width volatility compression. Volume spike directly measures buying/selling pressure rather than volatility state.

## Proposed Config Changes (config.rs)

```rust
// ── RUN471: Schaff Trend Cycle with Volume Ratio Spike ─────────────────────────────────
// stc: schaff_trend_cycle combining macd_and cycle_smoothing
// stc_cross: stc crosses above/below signal line (25/75)
// vol_ratio: volume / sma(volume, period)
// vol_spike: vol_ratio > threshold indicating unusual volume
// LONG: stc_cross bullish AND vol_ratio > 1.3
// SHORT: stc_cross bearish AND vol_ratio > 1.3

pub const STC_VOL_ENABLED: bool = true;
pub const STC_VOL_STC_FAST: usize = 23;
pub const STC_VOL_STC_SLOW: usize = 50;
pub const STC_VOL_STC_SIGNAL: usize = 10;
pub const STC_VOL_VOL_PERIOD: usize = 20;
pub const STC_VOL_RATIO_THRESH: f64 = 1.3;
pub const STC_VOL_SL: f64 = 0.005;
pub const STC_VOL_TP: f64 = 0.004;
pub const STC_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run471_1_stc_vol_backtest.py)
2. **Walk-forward** (run471_2_stc_vol_wf.py)
3. **Combined** (run471_3_combined.py)

## Out-of-Sample Testing

- STC_FAST sweep: 20 / 23 / 26
- STC_SLOW sweep: 45 / 50 / 55
- STC_SIGNAL sweep: 8 / 10 / 12
- VOL_PERIOD sweep: 14 / 20 / 30
- RATIO_THRESH sweep: 1.2 / 1.3 / 1.5
