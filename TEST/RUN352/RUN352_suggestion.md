# RUN352 — TRIX Histogram Divergence with Volume

## Hypothesis

**Mechanism**: TRIX (triple-smoothed momentum oscillator) histogram measures the rate of change of TRIX itself. When price makes a new high but TRIX histogram makes a lower high → bearish divergence → expect reversal down. When price makes a new low but TRIX histogram makes a higher low → bullish divergence → expect reversal up. Volume confirms: volume declining during bearish divergence = distribution (smarter money selling into strength).

**Why not duplicate**: RUN311 uses TRIX but as a crossover signal. This RUN specifically uses TRIX histogram divergence — the divergence detection combined with volume confirmation is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN352: TRIX Histogram Divergence with Volume ────────────────────────────────
// trix = EMA(EMA(EMA(close, period), period), period)
// trix_histogram = trix - EMA(trix, signal_period)
// bearish_divergence: price makes higher_high AND trix_hist makes lower_high
// bullish_divergence: price makes lower_low AND trix_hist makes higher_low
// volume_confirmation: volume declining during divergence = stronger signal
// LONG: bullish_divergence AND volume declining
// SHORT: bearish_divergence AND volume declining

pub const TRIX_DIV_ENABLED: bool = true;
pub const TRIX_DIV_PERIOD: usize = 15;
pub const TRIX_DIV_SIGNAL: usize = 5;
pub const TRIX_DIV_LOOKBACK: usize = 20;
pub const TRIX_DIV_VOL_CONFIRM: bool = true;
pub const TRIX_DIV_SL: f64 = 0.005;
pub const TRIX_DIV_TP: f64 = 0.004;
pub const TRIX_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run352_1_trix_div_backtest.py)
2. **Walk-forward** (run352_2_trix_div_wf.py)
3. **Combined** (run352_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 15 / 20
- SIGNAL sweep: 3 / 5 / 8
- LOOKBACK sweep: 14 / 20 / 30
