# RUN320 — Volume-Weighted RSI with Multi-Period Confirmation

## Hypothesis

**Mechanism**: Standard RSI ignores volume — volume-weighted RSI (VWRSI) weights each price change by volume. A move up on high volume is more significant than the same move on low volume. Combine VWRSI across multiple timeframes (15m, 1h) — when both show oversold simultaneously → strong mean-reversion LONG. When both show overbought → strong mean-reversion SHORT. Volume weighting adds institutional conviction to the signal.

**Why not duplicate**: RUN131 uses volume-weighted RSI confirmation. RUN246 uses multi-timeframe RSI. RUN247 uses RSI-volume divergence. This RUN combines the two concepts: VWRSI (volume-weighted) applied across multiple timeframes simultaneously. No prior RUN uses volume-weighted RSI with multi-timeframe confluence.

## Proposed Config Changes (config.rs)

```rust
// ── RUN320: Volume-Weighted RSI Multi-Period ───────────────────────────────────
// vw_rsi = sum(volume * price_change) / sum(volume) over RSI period
// LONG: vw_rsi_15m < 30 AND vw_rsi_1h < 30 (both oversold)
// SHORT: vw_rsi_15m > 70 AND vw_rsi_1h > 70 (both overbought)
// Relaxed mode: at least 1 of 2 timeframes at extreme (REQUIRE_ALL = false)

pub const VWRSI_MTF_ENABLED: bool = true;
pub const VWRSI_MTF_PERIOD: usize = 14;
pub const VWRSI_MTF_OVERSOLD: f64 = 30.0;
pub const VWRSI_MTF_OVERBOUGHT: f64 = 70.0;
pub const VWRSI_MTF_REQUIRE_ALL: bool = true;  // true = both TF must agree
pub const VWRSI_MTF_SL: f64 = 0.005;
pub const VWRSI_MTF_TP: f64 = 0.004;
pub const VWRSI_MTF_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run320_1_vwrsi_mtf_backtest.py)
2. **Walk-forward** (run320_2_vwrsi_mtf_wf.py)
3. **Combined** (run320_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- OVERSOLD sweep: 25 / 30 / 35
- OVERBOUGHT sweep: 65 / 70 / 75
- REQUIRE_ALL sweep: true / false
