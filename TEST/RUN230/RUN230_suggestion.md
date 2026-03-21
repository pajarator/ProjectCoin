# RUN230 — TEMA Crossover: Triple EMA for Low-Lag Trend Following

## Hypothesis

**Mechanism**: TEMA = 3×EMA(close, period) - 3×EMA(EMA, period) + EMA(EMA(EMA), period). The triple smoothing eliminates almost all lag compared to single EMA. When TEMA fast (e.g., 10) crosses above TEMA slow (e.g., 30) → LONG. When TEMA fast crosses below TEMA slow → SHORT. TEMA crossover is faster and more responsive than standard EMA crossover.

**Why not duplicate**: No prior RUN uses TEMA. All prior EMA cross RUNs use standard single or dual EMA. TEMA is specifically designed to have less lag than standard EMA — a meaningful improvement for trend-following entries.

## Proposed Config Changes (config.rs)

```rust
// ── RUN230: TEMA Crossover ───────────────────────────────────────────────
// tema_fast = TEMA(close, fast_period)
// tema_slow = TEMA(close, slow_period)
// LONG: tema_fast crosses above tema_slow
// SHORT: tema_fast crosses below tema_slow

pub const TEMA_ENABLED: bool = true;
pub const TEMA_FAST_PERIOD: usize = 10;     // fast TEMA period
pub const TEMA_SLOW_PERIOD: usize = 30;     // slow TEMA period
pub const TEMA_SL: f64 = 0.005;
pub const TEMA_TP: f64 = 0.004;
pub const TEMA_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn tema(closes: &[f64], period: usize) -> f64 {
    if closes.len() < period * 3 {
        return 0.0;
    }
    let ema1 = ema(closes, period);
    let ema2 = ema(&[ema1], period); // simplified - real impl needs full array
    let ema3 = ema(&[ema2], period);
    3.0 * ema1 - 3.0 * ema2 + ema3
}
```

---

## Validation Method

1. **Historical backtest** (run230_1_tema_backtest.py)
2. **Walk-forward** (run230_2_tema_wf.py)
3. **Combined** (run230_3_combined.py)

## Out-of-Sample Testing

- FAST_PERIOD sweep: 5 / 10 / 15
- SLOW_PERIOD sweep: 20 / 30 / 50
