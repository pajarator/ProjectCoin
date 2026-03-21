# RUN281 — Keltner Channel EMA Momentum: Trend Confirmation Filter

## Hypothesis

**Mechanism**: Only enter a LONG when price is above the EMA(20) AND the EMA is rising. Only enter a SHORT when price is below EMA(20) AND EMA is falling. The EMA direction confirms the broader trend before taking entries. Combine with Keltner channel for entries.

**Why not duplicate**: RUN188 uses Keltner Channel alone. RUN237 uses Williams %R with EMA filter. Keltner + EMA direction is a distinct combination for trend-confirmed entries.

## Proposed Config Changes (config.rs)

```rust
// ── RUN281: Keltner Channel EMA Momentum ────────────────────────────────
// keltner_upper = EMA + ATR × 2
// keltner_lower = EMA - ATR × 2
// ema_rising = EMA(close, 20) > EMA(close[5], 20)
// LONG: price crosses above keltner_upper AND ema_rising
// SHORT: price crosses below keltner_lower AND !ema_rising

pub const KELTNER_EMAMOM_ENABLED: bool = true;
pub const KELTNER_EMAMOM_EMA_PERIOD: usize = 20;
pub const KELTNER_EMAMOM_ATR_PERIOD: usize = 14;
pub const KELTNER_EMAMOM_MULT: f64 = 2.0;
pub const KELTNER_EMAMOM_EMA_MOM_PERIOD: usize = 5; // EMA momentum lookback
pub const KELTNER_EMAMOM_SL: f64 = 0.005;
pub const KELTNER_EMAMOM_TP: f64 = 0.004;
pub const KELTNER_EMAMOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run281_1_keltner_emamom_backtest.py)
2. **Walk-forward** (run281_2_keltner_emamom_wf.py)
3. **Combined** (run281_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 50
- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- EMA_MOM_PERIOD sweep: 3 / 5 / 7
