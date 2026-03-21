# RUN318 — Keltner Channel with Volume Confirmation: ATR-Band Trend Entries

## Hypothesis

**Mechanism**: Keltner Channel uses ATR to create bands around an EMA. When price closes above the upper band → bullish breakout. When price closes below the lower band → bearish breakout. Unlike Bollinger Bands (stddev), Keltner uses ATR which is more responsive to volatility changes. Volume confirmation: require volume > avg_volume × vol_mult on the breakout bar. This filters out low-volume false breakouts.

**Why not duplicate**: RUN188 uses Keltner channel breakout. RUN265 uses Keltner + volume. This RUN uses the combination differently: volume confirmation is required at the *band* touch, not just Keltner direction. The distinction is that volume surge at the band is the primary trigger, not just price crossing the band.

## Proposed Config Changes (config.rs)

```rust
// ── RUN318: Keltner Channel with Volume Confirmation ─────────────────────────
// keltner_mid = EMA(close, period)
// keltner_upper = keltner_mid + ATR_mult * ATR(period)
// keltner_lower = keltner_mid - ATR_mult * ATR(period)
// LONG: close crosses above upper_band AND volume > avg_vol * VOL_MULT
// SHORT: close crosses below lower_band AND volume > avg_vol * VOL_MULT
// Exit: close crosses back through mid EMA

pub const KC_VOL_ENABLED: bool = true;
pub const KC_VOL_PERIOD: usize = 20;          // EMA period for mid line
pub const KC_VOL_ATR_PERIOD: usize = 14;     // ATR period
pub const KC_VOL_ATR_MULT: f64 = 2.0;        // band width multiplier
pub const KC_VOL_VOL_MULT: f64 = 1.5;        // volume must exceed this × avg
pub const KC_VOL_SL: f64 = 0.005;
pub const KC_VOL_TP: f64 = 0.004;
pub const KC_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run318_1_kc_vol_backtest.py)
2. **Walk-forward** (run318_2_kc_vol_wf.py)
3. **Combined** (run318_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
- PERIOD sweep: 15 / 20 / 30
