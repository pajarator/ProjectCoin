# RUN368 — Z-Score Convergence with Multi-Timeframe Confirmation

## Hypothesis

**Mechanism**: When multiple timeframes show Z-scores at extreme levels simultaneously, the signal is much stronger. If 15m Z-score > 2.0 AND 1h Z-score > 1.5, the upside is limited — SHORT. If both are < -2.0 and < -1.5, downside is limited — LONG. Multi-timeframe Z-score convergence means the market is at an extreme that's reinforced across timeframes.

**Why not duplicate**: RUN78 uses Z-score convergence filter. This RUN specifically uses multi-timeframe Z-score alignment as the primary entry mechanism, not just a filter. The distinct mechanism is requiring Z-score extremes on multiple timeframes simultaneously.

## Proposed Config Changes (config.rs)

```rust
// ── RUN368: Z-Score Convergence with Multi-Timeframe Confirmation ────────────────────────────
// z_15m = z_score(close, SMA20, STD20) on 15m
// z_1h = z_score(close, SMA20, STD20) on 1h
// convergence_up = z_15m > Z_EXTREME AND z_1h > Z_CONFIRM (both extended up)
// convergence_down = z_15m < -Z_EXTREME AND z_1h < -Z_CONFIRM (both extended down)
// LONG: convergence_down AND price showing recovery
// SHORT: convergence_up AND price showing rejection

pub const ZSCORE_MTF_ENABLED: bool = true;
pub const ZSCORE_MTF_Z_EXTREME: f64 = 2.0;
pub const ZSCORE_MTF_Z_CONFIRM: f64 = 1.5;
pub const ZSCORE_MTF_SL: f64 = 0.005;
pub const ZSCORE_MTF_TP: f64 = 0.004;
pub const ZSCORE_MTF_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run368_1_zscore_mtf_backtest.py)
2. **Walk-forward** (run368_2_zscore_mtf_wf.py)
3. **Combined** (run368_3_combined.py)

## Out-of-Sample Testing

- Z_EXTREME sweep: 1.5 / 2.0 / 2.5
- Z_CONFIRM sweep: 1.0 / 1.5 / 2.0
