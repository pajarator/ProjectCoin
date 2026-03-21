# RUN265 — Keltner Channel with Volume Confirmation: Trend Momentum Filter

## Hypothesis

**Mechanism**: Combine Keltner Channel breakout with volume confirmation. A breakout above upper Keltner band is more reliable when accompanied by above-average volume. Volume confirms that the breakout is driven by real institutional money, not just price manipulation.

**Why not duplicate**: RUN188 uses Keltner Channel without volume. RUN206 uses Price Channel with volume. Keltner + Volume is a distinct combination not yet tested.

## Proposed Config Changes (config.rs)

```rust
// ── RUN265: Keltner Channel with Volume Confirmation ─────────────────────
// keltner_upper = EMA + ATR × 2
// keltner_lower = EMA - ATR × 2
// vol_ma = SMA(volume, 20)
// LONG: close > keltner_upper AND volume > vol_ma × 1.5
// SHORT: close < keltner_lower AND volume > vol_ma × 1.5

pub const KELTNER_VOL_ENABLED: bool = true;
pub const KELTNER_VOL_EMA_PERIOD: usize = 20;
pub const KELTNER_VOL_ATR_PERIOD: usize = 14;
pub const KELTNER_VOL_MULT: f64 = 2.0;
pub const KELTNER_VOL_VOL_MA: usize = 20;
pub const KELTNER_VOL_VOL_MULT: f64 = 1.5;
pub const KELTNER_VOL_SL: f64 = 0.005;
pub const KELTNER_VOL_TP: f64 = 0.004;
pub const KELTNER_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run265_1_keltner_vol_backtest.py)
2. **Walk-forward** (run265_2_keltner_vol_wf.py)
3. **Combined** (run265_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
