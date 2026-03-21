# RUN470 — Keltner Channel with Fisher Transform Confirmation

## Hypothesis

**Mechanism**: Keltner Channel uses ATR to create volatility-based bands around an EMA, providing clear support/resistance boundaries that adapt to market conditions. Fisher Transform normalizes price data and identifies trend reversals with sharp signal changes. When price reaches Keltner Channel boundary AND Fisher Transform confirms the reversal direction, the entry has both volatility-based structure and statistically transformed momentum confirmation.

**Why not duplicate**: RUN443 uses Keltner Channel with RSI Pullback Confirmation. This RUN uses Fisher Transform instead — the distinct mechanism is Fisher Transform's Gaussian normalization versus RSI oscillator extremes. Fisher Transform provides more responsive reversal signals based on price distribution.

## Proposed Config Changes (config.rs)

```rust
// ── RUN470: Keltner Channel with Fisher Transform Confirmation ─────────────────────────────────
// keltner_channel: ema_center +/- atr_mult * atr bands
// keltner_touch: price touches or exceeds upper/lower band
// fisher_transform: gaussian_normalized price transform for reversal detection
// fisher_cross: fisher crosses above/below signal line
// LONG: price touches keltner_lower AND fisher_cross bullish
// SHORT: price touches keltner_upper AND fisher_cross bearish

pub const KC_FISHER_ENABLED: bool = true;
pub const KC_FISHER_KC_PERIOD: usize = 20;
pub const KC_FISHER_KC_ATR_PERIOD: usize = 14;
pub const KC_FISHER_KC_MULT: f64 = 2.0;
pub const KC_FISHER_FISHER_PERIOD: usize = 10;
pub const KC_FISHER_SL: f64 = 0.005;
pub const KC_FISHER_TP: f64 = 0.004;
pub const KC_FISHER_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run470_1_kc_fisher_backtest.py)
2. **Walk-forward** (run470_2_kc_fisher_wf.py)
3. **Combined** (run470_3_combined.py)

## Out-of-Sample Testing

- KC_PERIOD sweep: 15 / 20 / 25
- KC_ATR_PERIOD sweep: 10 / 14 / 20
- KC_MULT sweep: 1.5 / 2.0 / 2.5
- FISHER_PERIOD sweep: 7 / 10 / 14
