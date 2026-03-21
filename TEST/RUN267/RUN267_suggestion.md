# RUN267 — Williams %R Percentile Rank: Oscillator Extremes Relative to History

## Hypothesis

**Mechanism**: Like RSI percentile, Williams %R percentile rank tells you where the current reading falls in its historical distribution. %R at 95th percentile = historically overbought even if not at -10. %R at 5th percentile = historically oversold even if not at -90. Trade extremes of the percentile rank.

**Why not duplicate**: No prior RUN uses Williams %R percentile rank. RUN237 uses Williams %R with EMA filter but not percentile. Percentile rank on Williams %R is unique because it identifies historical extremes of this specific oscillator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN267: Williams %R Percentile Rank ────────────────────────────────
// williams_r = (HH - close) / (HH - LL) × -100
// williams_pct = percentile rank of current %R within %R history
// williams_pct > 90 → extremely overbought → SHORT
// williams_pct < 10 → extremely oversold → LONG

pub const WILLIAMS_PCT_ENABLED: bool = true;
pub const WILLIAMS_PCT_PERIOD: usize = 14;
pub const WILLIAMS_PCT_WINDOW: usize = 100;
pub const WILLIAMS_PCT_OVERSOLD: f64 = 10.0;
pub const WILLIAMS_PCT_OVERBOUGHT: f64 = 90.0;
pub const WILLIAMS_PCT_SL: f64 = 0.005;
pub const WILLIAMS_PCT_TP: f64 = 0.004;
pub const WILLIAMS_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run267_1_williams_pct_backtest.py)
2. **Walk-forward** (run267_2_williams_pct_wf.py)
3. **Combined** (run267_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- WINDOW sweep: 50 / 100 / 200
- OVERSOLD sweep: 5 / 10 / 15
- OVERBOUGHT sweep: 85 / 90 / 95
