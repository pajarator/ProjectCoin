# RUN346 — ATR-Adjusted Williams %R: Volatility-Normalized Overbought/Oversold

## Hypothesis

**Mechanism**: Standard Williams %R uses a fixed N-bar lookback for highest high and lowest low. In high-volatility periods, this lookback is too short (producing extreme readings). In low-volatility periods, it's too long (readings never reach extremes). This RUN adjusts the %R lookback dynamically using ATR: higher ATR → longer effective lookback → thresholds are volatility-normalized. The result is a %R that gives comparable readings across different volatility environments.

**Why not duplicate**: RUN237 uses Williams %R with EMA filter. RUN267 uses Williams %R percentile. RUN306 uses Williams %R multi-timeframe. No RUN adjusts the Williams %R calculation itself by ATR. The adaptive lookback is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN346: ATR-Adjusted Williams %R ───────────────────────────────────────────
// For each bar, compute atr_scaled_lookback = BASE_LOOKBACK * (current_ATR / avg_ATR)
// Higher ATR → longer effective lookback for %R calculation
// williams_r_scaled = (highest(atr_scaled_lookback) - close) / (highest(atr_scaled_lookback) - lowest(atr_scaled_lookback)) * -100
// LONG: williams_r_scaled < WR_OVERSOLD
// SHORT: williams_r_scaled > WR_OVERBOUGHT

pub const WR_ATR_ENABLED: bool = true;
pub const WR_ATR_BASE_LOOKBACK: usize = 14;
pub const WR_ATR_ATR_PERIOD: usize = 14;
pub const WR_ATR_AVG_ATR_PERIOD: usize = 100;  // period for computing average ATR
pub const WR_ATR_OVERSOLD: f64 = -80.0;
pub const WR_ATR_OVERBOUGHT: f64 = -20.0;
pub const WR_ATR_SL: f64 = 0.005;
pub const WR_ATR_TP: f64 = 0.004;
pub const WR_ATR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run346_1_wr_atr_backtest.py)
2. **Walk-forward** (run346_2_wr_atr_wf.py)
3. **Combined** (run346_3_combined.py)

## Out-of-Sample Testing

- BASE_LOOKBACK sweep: 10 / 14 / 21
- AVG_ATR_PERIOD sweep: 50 / 100 / 200
- OVERSOLD sweep: -85 / -80 / -75
- OVERBOUGHT sweep: -25 / -20 / -15
