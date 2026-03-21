# RUN369 — Ichimoku Cloud with Volume Confirmation

## Hypothesis

**Mechanism**: Ichimoku Cloud (RUN185 breakout, RUN299 twist) signals price position relative to the cloud. This RUN specifically combines cloud signals with volume confirmation: when price breaks above the cloud AND volume > avg_vol × vol_mult, the breakout is volume-confirmed. When price breaks below the cloud AND volume > avg_vol × vol_mult, the breakdown is confirmed. Volume separates genuine breakouts from false moves.

**Why not duplicate**: RUN185 uses Ichimoku breakout. RUN299 uses Ichimoku twist. This RUN specifically adds volume confirmation to cloud breakouts — the volume filter is the distinct mechanism that differentiates it from all prior Ichimoku RUNs.

## Proposed Config Changes (config.rs)

```rust
// ── RUN369: Ichimoku Cloud with Volume Confirmation ────────────────────────────────
// ichimoku_cloud = senkou_a vs senkou_b spread
// cloud_breakout_up = close crosses above cloud AND volume > avg_vol * VOL_MULT
// cloud_breakout_down = close crosses below cloud AND volume > avg_vol * VOL_MULT
// Volume confirms institutional participation in the breakout

pub const ICHIMOKU_VOL_ENABLED: bool = true;
pub const ICHIMOKU_VOL_TENKAN: usize = 9;
pub const ICHIMOKU_VOL_KIJUN: usize = 26;
pub const ICHIMOKU_VOL_SENKOU_B: usize = 52;
pub const ICHIMOKU_VOL_VOL_MULT: f64 = 1.5;
pub const ICHIMOKU_VOL_SL: f64 = 0.005;
pub const ICHIMOKU_VOL_TP: f64 = 0.004;
pub const ICHIMOKU_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run369_1_ichimoku_vol_backtest.py)
2. **Walk-forward** (run369_2_ichimoku_vol_wf.py)
3. **Combined** (run369_3_combined.py)

## Out-of-Sample Testing

- TENKAN sweep: 7 / 9 / 12
- KIJUN sweep: 22 / 26 / 34
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
