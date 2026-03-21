# RUN392 — TTM Sniper with Volume Percentile Confirmation

## Hypothesis

**Mechanism**: TTM Sniper is a composite indicator that combines multiple timeframes of price action to identify short-term support and resistance levels. It fires signals when price crosses key levels on multiple timeframes simultaneously. Add volume confirmation: only take TTM signals when volume is in the top percentile of recent history (high volume confirms institutional participation in the move). Low-volume breakouts often fail; high-volume breakouts have institutional backing.

**Why not duplicate**: RUN350 uses Opening Range Breakout with Volume. RUN359 uses Donchian Channel with Volume Surge. This RUN specifically uses TTM Sniper (a composite multi-timeframe indicator distinct from simple ORB or Donchian) with Volume Percentile as a threshold filter — the distinct mechanism is TTM's composite multi-timeframe signal combined with volume percentile ranking.

## Proposed Config Changes (config.rs)

```rust
// ── RUN392: TTM Sniper with Volume Percentile Confirmation ──────────────────────────
// ttm_sniper: composite of 3 timeframes for short-term S/R levels
// ttm_signal: price crosses key TTM level
// volume_percentile: current volume rank within lookback window
// vol_confirm: volume_percentile > VOL_PERCENTILE_THRESH
// LONG: ttm_sniper bullish AND vol_percentile > threshold
// SHORT: ttm_sniper bearish AND vol_percentile > threshold

pub const TTM_VOL_ENABLED: bool = true;
pub const TTM_VOL_TTM_PERIOD: usize = 5;   // shortest timeframe component
pub const TTM_VOL_TTM_HTF: usize = 3;      // number of higher timeframes
pub const TTM_VOL_VOL_PERIOD: usize = 20;  // volume percentile lookback
pub const TTM_VOL_VOL_THRESH: f64 = 75.0;  // volume must be top 25%
pub const TTM_VOL_SL: f64 = 0.005;
pub const TTM_VOL_TP: f64 = 0.004;
pub const TTM_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run392_1_ttm_vol_backtest.py)
2. **Walk-forward** (run392_2_ttm_vol_wf.py)
3. **Combined** (run392_3_combined.py)

## Out-of-Sample Testing

- TTM_PERIOD sweep: 3 / 5 / 7
- TTM_HTF sweep: 2 / 3 / 4
- VOL_PERIOD sweep: 15 / 20 / 30
- VOL_THRESH sweep: 70 / 75 / 80
