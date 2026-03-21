# RUN345 — Keltner Channel with ADX Volatility Expansion Filter

## Hypothesis

**Mechanism**: Standard Keltner channel breakout fires too many signals in low-volatility conditions. Add ADX as a filter: only trade Keltner breakouts when ADX is rising AND above a minimum threshold — this confirms the market has enough directional energy for the breakout to continue. In low ADX conditions (ADX < 20), suppress Keltner signals because the market is choppy.

**Why not duplicate**: RUN188 uses Keltner channel breakout. RUN318 uses Keltner with volume confirmation. This RUN adds the ADX filter specifically — ADX must be rising for a valid breakout. The distinct mechanism is ADX-volatility-filtered Keltner breakouts.

## Proposed Config Changes (config.rs)

```rust
// ── RUN345: Keltner Channel with ADX Volatility Expansion Filter ───────────────────
// keltner_mid = EMA(close, period)
// keltner_upper = keltner_mid + ATR_mult * ATR(period)
// keltner_lower = keltner_mid - ATR_mult * ATR(period)
// adx_rising = adx > adx[3] AND adx > ADX_MIN
// LONG: close crosses above upper_band AND adx_rising
// SHORT: close crosses below lower_band AND adx_rising
// Filter: if ADX < ADX_MIN → no trades (choppy market)

pub const KC_ADX_ENABLED: bool = true;
pub const KC_ADX_ATR_PERIOD: usize = 14;
pub const KC_ADX_EMA_PERIOD: usize = 20;
pub const KC_ADX_ATR_MULT: f64 = 2.0;
pub const KC_ADX_ADX_PERIOD: usize = 14;
pub const KC_ADX_ADX_MIN: f64 = 20.0;
pub const KC_ADX_SL: f64 = 0.005;
pub const KC_ADX_TP: f64 = 0.004;
pub const KC_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run345_1_kc_adx_backtest.py)
2. **Walk-forward** (run345_2_kc_adx_wf.py)
3. **Combined** (run345_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 1.5 / 2.0 / 2.5
- EMA_PERIOD sweep: 15 / 20 / 30
- ADX_MIN sweep: 15 / 20 / 25
