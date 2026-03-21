# RUN443 — Keltner Channel with RSI Pullback Confirmation

## Hypothesis

**Mechanism**: Keltner Channel bands define dynamic support/resistance. Price touching the upper band doesn't always mean a reversal — sometimes it breaks through. RSI Pullback Confirmation adds timing precision: when price touches the Keltner band AND RSI is pulling back from overbought/oversold (but not yet extreme), the touch is more likely to hold as a reversal point. This distinguishes between "touching the band for a continuation" vs "touching the band as an exhaustion point."

**Why not duplicate**: RUN318 uses Keltner with Volume. RUN381 uses Keltner with Stochastic. RUN425 uses Keltner with CMO. This RUN uses RSI Pullback — specifically using RSI's pullback from extreme (rather than being at extreme) as the confirmation mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN443: Keltner Channel with RSI Pullback Confirmation ─────────────────────────────────────
// keltner_upper = EMA(close, period) + MULT * ATR(period)
// keltner_lower = EMA(close, period) - MULT * ATR(period)
// band_touch: price crosses keltner_upper or keltner_lower
// rsi_pullback: rsi is pulling back from overbought/oversold zone (crossing back through 50)
// rsi_pullback_confirm: rsi crosses 50 in the direction opposite to band touch
// LONG: price touches keltner_lower AND rsi_pullback_confirm bullish (RSI crossing 50 up)
// SHORT: price touches keltner_upper AND rsi_pullback_confirm bearish (RSI crossing 50 down)

pub const KC_RSI_PB_ENABLED: bool = true;
pub const KC_RSI_PB_KC_PERIOD: usize = 20;
pub const KC_RSI_PB_ATR_PERIOD: usize = 14;
pub const KC_RSI_PB_ATR_MULT: f64 = 2.0;
pub const KC_RSI_PB_RSI_PERIOD: usize = 14;
pub const KC_RSI_PB_SL: f64 = 0.005;
pub const KC_RSI_PB_TP: f64 = 0.004;
pub const KC_RSI_PB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run443_1_kc_rsi_pb_backtest.py)
2. **Walk-forward** (run443_2_kc_rsi_pb_wf.py)
3. **Combined** (run443_3_combined.py)

## Out-of-Sample Testing

- KC_PERIOD sweep: 15 / 20 / 30
- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- RSI_PERIOD sweep: 10 / 14 / 21
