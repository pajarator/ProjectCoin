# RUN493 — Elder Ray Index with KST Momentum Confirmation

## Hypothesis

**Mechanism**: Elder Ray Index measures buying/selling power by comparing price highs/lows to an EMA. Bull Power = High - EMA, Bear Power = Low - EMA. KST Momentum Confirmation provides multi-timeframe momentum verification: when Elder Ray signals AND KST also confirms momentum in the same direction across multiple ROC periods, entries have both EMA-based trend structure and multi-timeframe momentum conviction.

**Why not duplicate**: RUN460 uses Elder Ray Index with Stochastic Confirmation. This RUN uses KST instead — distinct mechanism is KST's multi-timeframe smoothed momentum confirmation versus Stochastic's single-timeframe oscillator. KST provides different momentum insight from multiple ROC periods.

## Proposed Config Changes (config.rs)

```rust
// ── RUN493: Elder Ray Index with KST Momentum Confirmation ─────────────────────────────────
// elder_ray: bull_power = high - ema, bear_power = low - ema
// elder_ray_cross: bull/bear power crosses above/below 0
// kst: know_sure_thing momentum from multiple roc periods
// kst_cross: kst crosses above/below signal line
// LONG: bull_power > 0 AND kst_cross bullish
// SHORT: bear_power < 0 AND kst_cross bearish

pub const ELDER_KST_ENABLED: bool = true;
pub const ELDER_KST_EMA_PERIOD: usize = 13;
pub const ELDER_KST_KST_ROC1: usize = 10;
pub const ELDER_KST_KST_ROC2: usize = 15;
pub const ELDER_KST_KST_ROC3: usize = 20;
pub const ELDER_KST_KST_ROC4: usize = 30;
pub const ELDER_KST_KST_SIGNAL: usize = 9;
pub const ELDER_KST_SL: f64 = 0.005;
pub const ELDER_KST_TP: f64 = 0.004;
pub const ELDER_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run493_1_elder_kst_backtest.py)
2. **Walk-forward** (run493_2_elder_kst_wf.py)
3. **Combined** (run493_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 13 / 20
- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
- KST_SIGNAL sweep: 7 / 9 / 12
