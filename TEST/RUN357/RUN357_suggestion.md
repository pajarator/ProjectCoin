# RUN357 — Elder Ray Index with ADX Trend Strength Filter

## Hypothesis

**Mechanism**: Elder Ray (ERI) measures buying pressure (bull power) and selling pressure (bear power) relative to an EMA. Bull power above 0 = buyers are stronger than average. Bear power below 0 = sellers are stronger than average. Add ADX as a filter: require ADX > 20 for trend-following entries (strong trend needed). When ADX < 20, suppress Elder Ray entries because the market is choppy and mean-reversion signals are unreliable.

**Why not duplicate**: RUN205 uses Elder Ray Index (ERI) as standalone. RUN122 uses Elder Ray as a confirmation filter. RUN319 uses Elder Ray Bull/Bear power with EMA filter. This RUN specifically adds ADX as a trend-strength gate — the distinct mechanism is using ADX to filter Elder Ray signals based on whether the market has enough directional energy.

## Proposed Config Changes (config.rs)

```rust
// ── RUN357: Elder Ray Index with ADX Trend Strength Filter ───────────────────────────────
// bull_power = high - EMA(close, period)
// bear_power = low - EMA(close, period)
// eri_bull = bull_power / ATR(period)  (normalized)
// eri_bear = bear_power / ATR(period)  (normalized)
// adx_strong = ADX(period) > ADX_MIN
// LONG: eri_bull > 0 AND adx_strong
// SHORT: eri_bear < 0 AND adx_strong

pub const ERI_ADX_ENABLED: bool = true;
pub const ERI_ADX_PERIOD: usize = 13;
pub const ERI_ADX_ADX_PERIOD: usize = 14;
pub const ERI_ADX_ADX_MIN: f64 = 20.0;
pub const ERI_ADX_SL: f64 = 0.005;
pub const ERI_ADX_TP: f64 = 0.004;
pub const ERI_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run357_1_eri_adx_backtest.py)
2. **Walk-forward** (run357_2_eri_adx_wf.py)
3. **Combined** (run357_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 13 / 20
- ADX_MIN sweep: 15 / 20 / 25
- ADX_PERIOD sweep: 10 / 14 / 21
