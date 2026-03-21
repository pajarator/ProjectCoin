# RUN413 — Z-Score Convergence with Volume Divergence

## Hypothesis

**Mechanism**: Z-Score measures how many standard deviations the current price is from its moving average. When Z-Score reaches extreme values (e.g., +3 or -3), it indicates the price has deviated significantly from the mean. Z-Score Convergence means multiple timeframes of Z-Score are all reaching extremes simultaneously. Volume Divergence confirms the move lacks institutional backing. When Z-Score is extreme across multiple timeframes AND volume diverges (price extreme but volume not confirming), the reversal probability is elevated.

**Why not duplicate**: RUN326 uses Z-Score Distance from VWAP. RUN368 uses Z-Score Convergence MTF. This RUN specifically uses Z-Score Convergence across multiple timeframes combined with Volume Divergence — the distinct mechanism is multi-timeframe Z-Score extremes with volume divergence confirmation, distinct from single-timeframe Z-score or MTF Z-score without volume filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN413: Z-Score Convergence with Volume Divergence ─────────────────────────────────
// zscore = (price - SMA(price, period)) / STD(price, period)
// mtf_zscore: calculate zscore on multiple timeframes (15m, 1h)
// convergence: zscore > Z_THRESH on multiple timeframes simultaneously
// vol_divergence: price at zscore extreme but volume not confirming
// LONG: mtf_zscore convergence on downside AND vol_divergence bullish
// SHORT: mtf_zscore convergence on upside AND vol_divergence bearish

pub const ZSCORE_VOL_DIV_ENABLED: bool = true;
pub const ZSCORE_VOL_DIV_Z_PERIOD: usize = 20;
pub const ZSCORE_VOL_DIV_Z_THRESH: f64 = 2.5;    // extreme threshold
pub const ZSCORE_VOL_DIV_MTF_COUNT: u32 = 3;    // number of timeframes for convergence
pub const ZSCORE_VOL_DIV_VOL_PERIOD: usize = 20;
pub const ZSCORE_VOL_DIV_SL: f64 = 0.005;
pub const ZSCORE_VOL_DIV_TP: f64 = 0.004;
pub const ZSCORE_VOL_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run413_1_zscore_vol_div_backtest.py)
2. **Walk-forward** (run413_2_zscore_vol_div_wf.py)
3. **Combined** (run413_3_combined.py)

## Out-of-Sample Testing

- Z_PERIOD sweep: 14 / 20 / 30
- Z_THRESH sweep: 2.0 / 2.5 / 3.0
- MTF_COUNT sweep: 2 / 3 / 4
- VOL_PERIOD sweep: 14 / 20 / 30
