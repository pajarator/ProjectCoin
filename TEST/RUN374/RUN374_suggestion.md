# RUN374 — TEMA Crossover with Volume Confirmation

## Hypothesis

**Mechanism**: TEMA (Triple Exponential Moving Average) applies triple EMA smoothing, reducing lag even further than DEMA or single EMA. TEMA crossover gives early trend signals. Volume confirmation: require volume > avg_vol × vol_mult on the crossover to confirm the trend has institutional backing. Low-volume crossovers are prone to failure.

**Why not duplicate**: RUN230 uses TEMA crossover without volume. This RUN specifically adds volume confirmation to TEMA crossover — the volume filter is the distinct mechanism that differentiates it from standard TEMA crossover.

## Proposed Config Changes (config.rs)

```rust
// ── RUN374: TEMA Crossover with Volume Confirmation ────────────────────────────────
// tema(n) = 3*EMA(close, n) - 3*EMA(EMA(close, n), n) + EMA(EMA(EMA(close, n), n), n)
// tema_fast = tema(FAST_PERIOD)
// tema_slow = tema(SLOW_PERIOD)
// crossover_up = tema_fast crosses above tema_slow AND volume > avg_vol * VOL_MULT
// crossover_down = tema_fast crosses below tema_slow AND volume > avg_vol * VOL_MULT

pub const TEMA_VOL_ENABLED: bool = true;
pub const TEMA_VOL_FAST: usize = 9;
pub const TEMA_VOL_SLOW: usize = 21;
pub const TEMA_VOL_VOL_MULT: f64 = 1.5;
pub const TEMA_VOL_SL: f64 = 0.005;
pub const TEMA_VOL_TP: f64 = 0.004;
pub const TEMA_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run374_1_tema_vol_backtest.py)
2. **Walk-forward** (run374_2_tema_vol_wf.py)
3. **Combined** (run374_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 7 / 9 / 12
- SLOW sweep: 16 / 21 / 30
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
